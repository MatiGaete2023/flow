"""Planificacion y descargas secuenciales, con una carpeta por lote."""
import base64
import hashlib
from collections import Counter
from dataclasses import dataclass,asdict
from datetime import datetime
import json
from pathlib import Path
import re
import unicodedata
from urllib.parse import urlencode,unquote_plus

import motor as m
from perfiles import Selection,TABS,response_profile,selection_profile
from evidencias import screenshot_pdf

MODALITIES={'1':'Residencia','2':'Ambulatorio','3':'FAE','4':'DCE'}


def safe_name(value):
    value=''.join(c for c in unicodedata.normalize('NFKD',value) if not unicodedata.combining(c))
    value=re.sub(r'[<>:"/\\|?*\x00-\x1f]',' ',value)
    return ' '.join(value.replace('.',' ').split()).rstrip(' .')[:125]


def tribunal_name(value):
    value=safe_name(value)
    value=re.sub(r'^Juzgado de Familia\s+','Jdo Familia ',value,flags=re.I)
    value=re.sub(r'^Jgdo\s+L\s+y\s+G\s+de\s+','Jdo Letras y Garantia ',value,flags=re.I)
    value=re.sub(r'^Jgdo\s+(?:Letras|L)\s+de\s+','Jdo Letras ',value,flags=re.I)
    return value


def file_base(selection,catalog):
    tribunal=tribunal_name(catalog['tribunales'][selection.tribunal])
    if selection.screen=='calendario_informes':return safe_name('Informes '+tribunal)
    if selection.screen=='calendario_medidas':return safe_name('Medidas por vencer '+tribunal)
    if selection.screen=='litigantes':return safe_name(f'Litigantes {catalog["estados"][selection.state]} {tribunal}')
    if selection.screen=='carga':return safe_name(f'Carga {tribunal} {selection.start} {selection.end}')
    if selection.tab=='Informes':return safe_name('Informes '+tribunal)
    if selection.tab=='Egresados':return safe_name('Egresados '+tribunal)
    prefix='ESP' if selection.tab=='Espera' else 'CUMP'
    return safe_name(f'{prefix} {MODALITIES[selection.modality]} {tribunal}')


@dataclass(frozen=True)
class Dates:
    start:str
    end:str

    def __post_init__(self):
        try:
            a=datetime.strptime(self.start,'%d/%m/%Y');b=datetime.strptime(self.end,'%d/%m/%Y')
            if a>b or not 1900<=a.year<=2100 or not 1900<=b.year<=2100:raise ValueError()
            if a.strftime('%d/%m/%Y')!=self.start or b.strftime('%d/%m/%Y')!=self.end:raise ValueError()
        except ValueError:raise m.PocError('Use fechas válidas DD/MM/AAAA, con Desde anterior o igual a Hasta.') from None


@dataclass(frozen=True)
class Batch:
    tribunals:tuple[str,...]
    modalities:tuple[str,...]
    tabs:tuple[str,...]
    report:str='1'
    dates:Dates|None=None
    keep_filters:bool=False
    screen:str='seguimiento'
    state:str='1'
    month:str|None=None
    year:str|None=None

    def plan(self,catalog):
        if not self.tribunals or (self.screen=='seguimiento' and (not self.modalities or not self.tabs)):
            raise m.PocError('Seleccione tribunales, modalidades y pestañas.')
        for values in (self.tribunals,self.modalities,self.tabs):
            if len(values)!=len(set(values)):raise m.PocError('El lote tiene opciones repetidas.')
        if set(self.modalities)-set(MODALITIES):raise m.PocError('Modalidad no auditada.')
        if self.screen=='seguimiento':
            result=[Selection(t,tab,mod,self.report if tab=='Informes' or (tab=='Cumplimiento' and self.report in ('4','5')) else None)
                    for t in self.tribunals for tab in self.tabs for mod in self.modalities]
        else:
            if self.screen.startswith('calendario_') and self.dates:raise m.PocError('El calendario usa mes y año, no un rango de fechas.')
            result=[Selection(t,'','',screen=self.screen,state=self.state,month=self.month,year=self.year) for t in self.tribunals]
            if self.screen=='carga':
                if not self.dates:raise m.PocError('Carga requiere un rango de fechas.')
                result=[Selection(t,'','',screen='carga',start=self.dates.start,end=self.dates.end) for t in self.tribunals]
        for selection in result:selection_profile(selection,catalog)
        return result


class ChromeTransport:
    def __init__(self,bridge,profile):self.bridge=bridge;self.profile=profile
    @staticmethod
    def body(result,excel=False):
        try:
            if not isinstance(result,dict) or not isinstance(result.get('body'),str):raise ValueError()
            if len(result['body'])>4*((80*1024*1024+2)//3):raise ValueError()
            if not excel and not isinstance(result.get('type'),str):raise ValueError()
            data=base64.b64decode(result['body'],validate=True)
            if result['status']!=200 or not data or len(data)>80*1024*1024:raise ValueError()
            if not excel and 'html' not in result['type'].lower():raise ValueError()
            return data
        except (KeyError,TypeError,ValueError):
            raise m.PocError('Respuesta vacía, vencida o fuera del formato permitido.') from None
    def search(self,pairs,timeout_ms):
        m.query_pairs(urlencode(pairs),self.profile)
        return self.body(self.bridge.call('search',{'pairs':pairs,'timeout':timeout_ms}))
    def download(self,url,timeout_ms):
        m.validate_export_url(url,self.profile)
        return self.body(self.bridge.call('download',{'url':url,'timeout':timeout_ms}),True)


class BatchRunner:
    def __init__(self,bridge,catalog,emit,cancel):
        self.bridge=bridge;self.catalog=catalog;self.emit=emit;self.cancel=cancel

    def run(self,batch,destination,plan_override=None):
        plan=batch.plan(self.catalog) if plan_override is None else list(plan_override)
        if not plan:raise m.PocError('El lote no tiene consultas.')
        for selection in plan:selection_profile(selection,self.catalog)
        folder=Path(destination)/m.now().strftime('%Y-%m-%d %H-%M-%S')
        # Si coincide el segundo, se conserva fecha/hora y se añade precision.
        if folder.exists():folder=Path(destination)/m.now().strftime('%Y-%m-%d %H-%M-%S-%f')
        folder.mkdir(parents=True,exist_ok=False)
        self.emit('folder',str(folder));self.emit('plan',len(plan))
        bases=Counter(file_base(s,self.catalog) for s in plan);ordinals=Counter();completed=[]
        state='EN_CURSO'
        def metadata(selection):
            return {**asdict(selection),'tribunal_nombre':self.catalog['tribunales'][selection.tribunal],
                    'modalidad_nombre':MODALITIES.get(selection.modality,''),
                    'fecha_descarga':m.now().isoformat()}
        planned=[metadata(selection) for selection in plan]
        def summary():
            payload={'version_contrato':1,'fecha_hora':m.now().isoformat(),'estado':state,'plan':planned,
                     'consultas_esperadas':len(plan),
                     'consultas_completadas':len(completed),'consultas':completed,
                     'filtro_fecha':None if batch.dates is None else {'desde':batch.dates.start,'hasta':batch.dates.end},
                     'otros_filtros':'formulario' if batch.keep_filters else 'todos los registros'}
            temp=folder/'resumen.tmp';temp.write_text(json.dumps(payload,indent=2,ensure_ascii=False),encoding='utf-8');temp.replace(folder/'resumen.json')
        summary();locked=False
        try:
            self.bridge.call('lock');locked=True
            for index,selection in enumerate(plan,1):
                if self.cancel.is_set():raise m.PocError('Lote cancelado. Se conservan los archivos verificados.')
                desired=selection_profile(selection,self.catalog)
                self.emit('query',{'index':index,'total':len(plan),'label':desired.label})
                prepared=self.bridge.call('prepare',{'selection':selection.__dict__,
                    'dates':None if batch.dates is None else batch.dates.__dict__,'keep_filters':batch.keep_filters})
                try:
                    wire_pairs=prepared['pairs']
                    if (not isinstance(wire_pairs,list) or len(wire_pairs)>100 or
                        any(not isinstance(pair,(list,tuple)) or len(pair)!=2 or
                            any(not isinstance(value,str) for value in pair) for pair in wire_pairs)):
                        raise ValueError()
                    pairs=m.query_pairs(urlencode([tuple(pair) for pair in wire_pairs]),desired)
                except (KeyError,TypeError,ValueError):
                    raise m.PocError('El formulario no entregó parámetros válidos.') from None
                fields=dict(pairs)
                if selection.screen not in ('carga',) and not selection.screen.startswith('calendario_') and fields.get('FLG_Consulta')!=('1' if batch.dates else '0'):
                    raise m.PocError('No se pudo activar correctamente el filtro de fecha.')
                if selection.screen!='carga' and batch.dates and (unquote_plus(fields.get('FEC_Inicio',''))!=batch.dates.start or unquote_plus(fields.get('FEC_Fin',''))!=batch.dates.end or fields.get('CHK_Consulta')!='on'):
                    raise m.PocError('Las fechas enviadas no coinciden con la selección.')
                transport=ChromeTransport(self.bridge,desired)
                profile,first=response_profile(transport.search(pairs,60000),pairs,selection,self.catalog)
                manifest=f'verificacion {index:03}.json'
                evidence=None
                if first is None:
                    if self.cancel.is_set():raise m.PocError('Lote cancelado. Se conservan los archivos verificados.')
                    self.emit('status','Preparando captura de la consulta sin registros…')
                    try:
                        native=self.bridge.call('evidence_open',{'pairs':pairs,'label':profile.label})
                        native_html=ChromeTransport.body(native)
                        digest=hashlib.sha256(native_html).hexdigest()
                        if native.get('digest')!=digest:raise m.PocError('La pantalla de evidencia no coincide con la respuesta recibida.')
                        _,native_first=response_profile(native_html.decode('utf-8'),pairs,selection,self.catalog)
                        if native_first is not None:raise m.PocError('La consulta cambió y ahora tiene registros. Inicie un lote nuevo; no se guardó un PDF vacío.')
                        native_fields=m.fields_from_form(m.root_html(native_html.decode('utf-8')).xpath('//form[@name=$name]',name=profile.form_name)[0])
                        date_fields=('FEC_Desde','FEC_Hasta') if selection.screen=='carga' else ('FEC_Inicio','FEC_Fin')
                        if batch.dates and any(unquote_plus(native_fields.get(k,''))!=v for k,v in zip(date_fields,(batch.dates.start,batch.dates.end))):
                            raise m.PocError('Las fechas de la pantalla a capturar no corresponden al lote.')
                        if self.cancel.is_set():raise m.PocError('Lote cancelado. Se conservan los archivos verificados.')
                        capture=self.bridge.call('evidence_capture',{'digest':digest})
                        pdf,checksum=screenshot_pdf(capture,digest,profile.label)
                        base=file_base(selection,self.catalog);ordinals[base]+=1
                        suffix=f' {ordinals[base]}' if bases[base]>1 else ''
                        name=base+suffix+'.pdf';path=folder/name;m.save_exclusive(path,pdf)
                        if hashlib.sha256(path.read_bytes()).hexdigest()!=checksum:
                            path.unlink(missing_ok=True);raise m.PocError('No se pudo verificar la escritura del PDF.')
                        evidence={'archivo':name,'registros':0,'sha256':checksum,'tipo':'PDF_CAPTURA_SITFA'}
                    finally:
                        try:self.bridge.call('evidence_close',timeout=8)
                        except Exception:pass
                    m.write_manifest(folder,[],0,'SIN_RESULTADOS',profile,manifest)
                    info=json.loads((folder/manifest).read_text(encoding='utf-8'));info['evidencia']=evidence
                    temp=folder/(manifest+'.tmp');temp.write_text(json.dumps(info,indent=2,ensure_ascii=False),encoding='utf-8');temp.replace(folder/manifest)
                    self.emit('evidence',evidence)
                    result=[];pages=0
                else:
                    self.emit('total',first.total);base=file_base(selection,self.catalog)
                    def filename(number,total):
                        ordinals[base]+=1
                        suffix=f' {ordinals[base]}' if total>1 or bases[base]>1 else ''
                        return base+suffix+'.xls'
                    transport.profile=profile
                    result=m.run_sequence(transport,pairs,first,folder,profile=profile,cancel=self.cancel,
                        folder_ready=True,manifest_name=manifest,filename_factory=filename,
                        progress=lambda item,path:self.emit('progress',{**item,'consulta':profile.label}))
                    pages=first.total
                completed.append({'consulta':profile.label,'estado':'VALIDADA' if pages else 'SIN_RESULTADOS',
                    'paginas':len(result),'registros':sum(x['registros'] for x in result),'verificacion':manifest,
                    'seleccion':planned[index-1]})
                info=json.loads((folder/manifest).read_text(encoding='utf-8'))
                info['seleccion']=planned[index-1]
                info['filtro_fecha']=None if batch.dates is None else asdict(batch.dates)
                temp=folder/(manifest+'.tmp');temp.write_text(json.dumps(info,indent=2,ensure_ascii=False),encoding='utf-8');temp.replace(folder/manifest)
                if evidence:completed[-1]['evidencia']=evidence
                summary();self.emit('query_done',completed[-1])
            state='VALIDADA';summary()
            self.emit('done',{'folder':str(folder),'queries':len(completed),
                'files':len(list(folder.glob('*.xls'))),'pdfs':len(list(folder.glob('*.pdf'))),'records':sum(x['registros'] for x in completed)})
            return completed
        except BaseException:
            state='INCOMPLETA';summary();raise
        finally:
            # También dejar alcance de la consulta interrumpida para resultados parciales.
            for index,selection in enumerate(planned,1):
                path=folder/f'verificacion {index:03}.json'
                if path.is_file():
                    try:
                        info=json.loads(path.read_text(encoding='utf-8'));info.setdefault('seleccion',selection)
                        temp=path.with_suffix('.tmp');temp.write_text(json.dumps(info,indent=2,ensure_ascii=False),encoding='utf-8');temp.replace(path)
                    except (OSError,ValueError):self.emit('warning','E_RESUMEN: no se pudo completar el alcance de una verificación.')
            if locked:
                try:
                    restored=self.bridge.call('unlock',timeout=8)
                    if isinstance(restored,dict) and restored.get('restored') is False:
                        self.emit('warning','No se restauraron todos los filtros de SITFA. Revisa el formulario antes de volver a consultar; los archivos verificados se conservan.')
                except Exception:
                    self.emit('warning','No se pudo confirmar la restauración de los filtros de SITFA. Revisa la pestaña antes de volver a consultar; los archivos verificados se conservan.')
