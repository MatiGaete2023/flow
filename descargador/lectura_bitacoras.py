"""Consulta secuencial de bitácoras: no dispone de operaciones de escritura RUS."""
import hashlib
import json
from collections import Counter
from copy import deepcopy
from dataclasses import asdict
from pathlib import Path
from urllib.parse import urlencode
from lxml import etree
import motor as m
from lotes import Batch, Dates, MODALITIES, ChromeTransport
from perfiles import selection_profile, response_profile
from vinculos import extract, visible_cell_text, calls
from csmp_shared import parser

MANIFEST='bitacoras.json'

def write(path,data):
    temp=path.with_suffix('.tmp')
    temp.write_text(json.dumps(data,ensure_ascii=False,indent=2),encoding='utf-8')
    temp.replace(path)

def load(path):
    path=Path(path)
    if path.stat().st_size>32*1024*1024:raise m.PocError('El lote excede el tamaño permitido.')
    data=json.loads(path.read_text(encoding='utf-8'))
    if data.get('version')!=1 or data.get('tipo')!='BITACORAS_LECTURA':
        raise m.PocError('El archivo no es un lote de lectura de bitácoras.')
    return data

def saved_batch(data):
    fields=dict(data['seleccion'])
    for key in ('tribunals','modalities','tabs'):fields[key]=tuple(fields[key])
    if fields.get('dates'):fields['dates']=Dates(**fields['dates'])
    return Batch(**fields)

def listing_rows(body,profile):
    """Devuelve también las filas sin enlace; ninguna desaparece del control."""
    # UTF-8 conserva las tildes de respuestas sin META. Si no es UTF-8,
    # deja que el parser usado por la descarga normal lea la declaración.
    if isinstance(body,bytes):
        try:body=body.decode('utf-8-sig')
        except UnicodeDecodeError:pass
    root=m.root_html(body);table=root.xpath('//form[@name=$form]//table[@id=$table]',form=profile.form_name,table=profile.table_id)
    if len(table)!=1:raise m.PocError('No se identifica una tabla única de registros.')
    header=None;columns=None;result=[]
    for position,row in enumerate(table[0].xpath('./tr|./tbody/tr|./thead/tr'),1):
        values=[visible_cell_text(c) for c in row.xpath('./td|./th')]
        current=m.record_columns(values)
        if current:header=row;columns=current;continue
        if columns is None or not m.record_from_row(values,columns):continue
        sample=etree.Element('table');sample.append(deepcopy(header));sample.append(deepcopy(row))
        links=extract(sample,profile.tribunal)
        names=[m.normalized(visible_cell_text(c)) for c in header.xpath('./td|./th')]
        def value(labels):
            slots=[i for i,v in enumerate(names) if v in labels]
            return values[slots[0]] if len(slots)==1 and slots[0]<len(values) else ''
        item={'rit':value(('RIT',)), 'nombre':values[columns.name],
              'programa':value(('DERIVACION','NOMBRE CENTRO','PROGRAMA')),
              'rut':value(('RUT','RUT (->RCEI)')),'fila':position}
        if len(links)==1:item.update(links[0])
        else:
            # Solo diagnóstico categórico; no registra HTML o datos del NNA.
            commands=' '.join(n.get(attr,'') for n in row.xpath('.//*[@onclick or @href]') for attr in ('onclick','href'))
            observation=calls(commands,'ShowObservaciones')
            item['vinculo_diagnostico']=(
                'SIN_ACCION_OBSERVACIONES' if not observation else
                'MULTIPLES_ACCIONES_OBSERVACIONES' if len(observation)>1 else
                'ACCION_OBSERVACIONES_NO_VERIFICADA'
            )
        result.append(item)
    if Counter(m.record_key(r['rit'],r['nombre']) for r in result)!=m.result_records(root.xpath('//form[@name=$form]',form=profile.form_name)[0],profile):
        raise m.PocError('El control de filas no coincide con el listado validado.')
    return result


def anonymous_diagnostic(data):
    """Resumen apto para compartir: nunca reproduce filas, identificadores ni HTML."""
    allowed_states=('LEIDA','FALLIDA','SIN_VINCULO','PENDIENTE')
    allowed_link=('SIN_ACCION_OBSERVACIONES','MULTIPLES_ACCIONES_OBSERVACIONES',
                  'ACCION_OBSERVACIONES_NO_VERIFICADA')
    allowed_extension=('DIARY_CONTEXT','DIARY_LINK','DIARY_IDENTITY',
                       'DIARY_NETWORK','DIARY_TIMEOUT','DIARY_RESPONSE')
    def safe_version(value):
        import re
        text=str(value or '')
        return text if re.fullmatch(r'[0-9]{1,3}(?:\.[0-9]{1,3}){2}(?:\.dev[0-9]{1,5})?',text) else 'NO_VERIFICADA'
    def safe_commit(value):
        import re
        text=str(value or '')
        return text.lower() if re.fullmatch(r'[0-9a-fA-F]{40}',text) else 'NO_VERIFICADO'
    def state_code(value):
        return value if value in allowed_states else 'OTRO'
    records=data.get('registros',[])
    global_states=Counter(state_code(r.get('estado')) for r in records)
    global_links=Counter(r.get('vinculo_diagnostico')
                         if r.get('vinculo_diagnostico') in allowed_link else 'NO_ESPECIFICADO'
                         for r in records if r.get('estado')=='SIN_VINCULO')
    global_extension=Counter((r.get('diagnostico') or {}).get('codigo_extension')
                             if (r.get('diagnostico') or {}).get('codigo_extension') in allowed_extension
                             else 'OTRA_FALLA'
                             for r in records if r.get('estado')=='FALLIDA')
    scopes=[]
    for q in data.get('consultas',[]):
        key=(q.get('tribunal'),q.get('modality'),q.get('tab'))
        subset=[r for r in records if (r.get('tribunal_codigo'),r.get('modalidad'),r.get('pestana'))==key]
        status=str(q.get('estado',''))
        scopes.append({
            'tribunal_codigo':str(key[0]) if str(key[0]).isdigit() else 'NO_VERIFICADO',
            'modalidad_codigo':str(key[1]) if key[1] in ('1','2','3','4') else 'NO_VERIFICADA',
            'pestana':key[2] if key[2] in ('Espera','Cumplimiento','Informes','Egreso') else 'NO_VERIFICADA',
            'estado_consulta':status if status in ('ENUMERADA','SIN_REGISTROS','FALLIDA','EN_CURSO') else 'OTRO',
            'paginas':q.get('paginas',0) if isinstance(q.get('paginas'),int) else 0,
            'registros':len(subset),
            'estados':dict(Counter(state_code(r.get('estado')) for r in subset))
        })
    return {
        'tipo':'BITACORAS_DIAGNOSTICO_ANONIMO',
        'version':1,
        'estado_lote':data.get('estado') if data.get('estado') in ('COMPLETA','INCOMPLETA','CANCELADA','EN_CURSO') else 'OTRO',
        'descargador_version':safe_version((data.get('origen') or {}).get('version')),
        'descargador_commit':safe_commit((data.get('origen') or {}).get('commit')),
        'extension_version':safe_version((data.get('extension') or {}).get('version')),
        'registros':len(records),
        'estados':dict(global_states),
        'diagnosticos_sin_vinculo':dict(global_links),
        'errores_extension':dict(global_extension),
        'consultas':scopes,
        'anteriores_no_reenumeradas':len(data.get('anteriores_no_reenumeradas',[])),
        'contiene_identificadores_personales':False
    }

class DiaryRunner:
    def __init__(self,bridge,catalog,emit,cancel):
        self.bridge,self.catalog,self.emit,self.cancel=bridge,catalog,emit,cancel

    def run(self,batch,destination,resume=None):
        if batch.screen!='seguimiento':raise m.PocError('Las bitácoras se consultan desde Seguimiento.')
        plan=batch.plan(self.catalog);read=parser()
        from nurus.bitacora_html import visible_identity_warnings
        def annotate_visible(record,snapshot):
            notes=visible_identity_warnings(snapshot,record['rit'],record['nombre'])
            record['advertencias']=notes
            record['error']=' '.join(notes)
        old={};previous_records=[]
        if resume:
            folder=Path(resume).resolve();previous=load(folder/MANIFEST)
            if asdict(batch)!=asdict(saved_batch(previous)):raise m.PocError('El alcance no coincide con el lote recuperado.')
            old={r['clave']:r for r in previous.get('registros',[]) if r.get('estado')=='LEIDA'}
            previous_records=previous.get('registros',[])+previous.get('anteriores_no_reenumeradas',[])
        else:
            folder=Path(destination).resolve()/('Bitacoras '+m.now().strftime('%Y-%m-%d %H-%M-%S-%f'))
            folder.mkdir(parents=True,exist_ok=False)
        state={'version':1,'tipo':'BITACORAS_LECTURA','estado':'EN_CURSO','seleccion':asdict(batch),
               'fecha':m.now().isoformat(),'consultas':[],'registros':[]}
        try:
            from _build_meta import BUILD_VERSION,BUILD_COMMIT
            state['origen']={'version':BUILD_VERSION,'commit':BUILD_COMMIT}
        except ImportError:state['origen']={'version':'desarrollo','commit':'sin metadatos'}
        manifest=folder/MANIFEST
        def checkpoint():write(manifest,state)
        def stopped():
            if self.cancel.is_set():raise m.PocError('Lectura cancelada. Se conserva lo ya comprobado.')
        def reuse(record):
            previous=old.get(record['clave'])
            if not previous or previous.get('params')!=record.get('params'):return False
            name=previous.get('archivo','')
            if Path(name).name!=name or not name.endswith('.html'):return False
            path=folder/name
            if not path.is_file() or path.stat().st_size>8*1024*1024:return False
            data=path.read_bytes()
            if hashlib.sha256(data).hexdigest()!=previous.get('sha256'):return False
            snapshot=read(data.decode('utf-8-sig'),record['params'])
            annotate_visible(record,snapshot)
            record.update({k:previous[k] for k in ('archivo','sha256','capturada','entradas')})
            record['estado']='LEIDA';record['reutilizada']=True
            return True
        checkpoint();self.emit('folder',str(folder));locked=False
        try:
            from contrato_integral import verify_extension
            state['extension']=verify_extension(self.bridge);checkpoint()
            self.bridge.call('lock');locked=True
            for index,selection in enumerate(plan,1):
                stopped();profile=selection_profile(selection,self.catalog)
                query={**asdict(selection),'tribunal_nombre':self.catalog['tribunales'][selection.tribunal],
                       'modalidad_nombre':MODALITIES[selection.modality],'estado':'EN_CURSO','paginas':0,'registros':0}
                state['consultas'].append(query);checkpoint()
                self.emit('query',{'index':index,'total':len(plan),'label':profile.label})
                phase='PREPARAR_FORMULARIO'
                try:
                    prepared=self.bridge.call('prepare',{'selection':asdict(selection),'dates':asdict(batch.dates) if batch.dates else None,'keep_filters':batch.keep_filters})
                    wire=prepared['pairs']
                    if not isinstance(wire,list) or len(wire)>100 or any(not isinstance(p,(list,tuple)) or len(p)!=2 or any(not isinstance(v,str) for v in p) for p in wire):
                        raise m.PocError('El formulario no entregó parámetros válidos.')
                    pairs=m.query_pairs(urlencode([tuple(p) for p in wire]),profile)
                    fields=dict(pairs)
                    if fields.get('FLG_Consulta')!=('1' if batch.dates else '0'):raise m.PocError('El filtro de fechas no coincide.')
                    if batch.dates:
                        from urllib.parse import unquote_plus
                        if any(unquote_plus(fields.get(k,''))!=v for k,v in (('FEC_Inicio',batch.dates.start),('FEC_Fin',batch.dates.end))):raise m.PocError('Las fechas enviadas no coinciden.')
                    phase='CONSULTAR_LISTADO'
                    transport=ChromeTransport(self.bridge,profile);body=transport.search(pairs,60000)
                    phase='VALIDAR_LISTADO'
                    profile,current=response_profile(body,pairs,selection,self.catalog);transport.profile=profile
                    if current is None:query['estado']='SIN_REGISTROS';checkpoint();continue
                    if current.current!=1:raise m.PocError('La consulta no comenzó en la primera página.')
                    total=current.total;original=m.invariant(pairs);seen=set()
                    for number in range(1,total+1):
                        stopped()
                        if number>1:
                            pairs=[(k,str(number) if k==profile.page_field else current.pagination.get(k,v)) for k,v in pairs]
                            if m.invariant(pairs)!=original:raise m.PocError('Cambió la consulta entre páginas.')
                            phase='CONSULTAR_LISTADO'
                            body=transport.search(pairs,60000)
                            phase='VALIDAR_LISTADO'
                            current=m.search_page(body,pairs,number,total,profile)
                        phase='LEER_FILAS'
                        for item in listing_rows(body,profile):
                            stopped()
                            record={**item,'tribunal_codigo':selection.tribunal,'tribunal_nombre':query['tribunal_nombre'],
                                    'modalidad':selection.modality,'modalidad_nombre':query['modalidad_nombre'],
                                    'pestana':selection.tab,'pagina':number,'estado':'PENDIENTE','error':''}
                            identity=(selection.tribunal,item.get('causa_id',''),item.get('ingreso_id',''))
                            if all(identity):
                                if identity in seen:raise m.PocError('Un ingreso está repetido en el listado; se detuvo esta consulta.')
                                seen.add(identity)
                            record['clave']=hashlib.sha256(json.dumps([selection.tab,selection.modality,identity,item['rit'],item['nombre'],item['programa'],item['rut'],item['fila'] if not all(identity) else ''],ensure_ascii=False).encode()).hexdigest()
                            state['registros'].append(record);query['registros']+=1
                            if not all(identity):
                                record.update(estado='SIN_VINCULO',error='La fila no tiene un enlace de bitácora inequívoco. No se construyó una consulta por RIT. Diagnóstico: '+item.get('vinculo_diagnostico','NO_VERIFICADO'))
                            else:
                                record['params']={'tipo_popUp':'12','CRR_IdCausa':item['causa_id'],'COD_Tribunal':selection.tribunal,
                                    'TIP_Consulta':selection.modality,'ID_Ingreso':item['ingreso_id'],'COD_Etapa':item['etapa'],'FLG_MejorNinez':item['antiguo']}
                                checkpoint()
                                try:
                                    if not reuse(record):
                                        result=self.bridge.call('diary_open',{'params':record['params'],'timeout':60000})
                                        data=ChromeTransport.body(result)
                                        digest=hashlib.sha256(data).hexdigest()
                                        if len(data)>8*1024*1024 or digest!=result.get('digest') or result.get('params')!=record['params']:
                                            raise m.PocError('La respuesta de bitácora no coincide con la solicitud.')
                                        from datetime import datetime
                                        datetime.fromisoformat(result['startedAt'].replace('Z','+00:00'))
                                        snapshot=read(data.decode('utf-8-sig'),record['params'])
                                        annotate_visible(record,snapshot)
                                        name='bitacora-'+record['clave'][:24]+'-'+digest[:16]+'.html'
                                        path=folder/name
                                        if path.exists():
                                            if path.read_bytes()!=data:raise m.PocError('El archivo local de bitácora cambió.')
                                        else:m.save_exclusive(path,data)
                                        record.update(estado='LEIDA',archivo=name,sha256=digest,capturada=result['startedAt'],entradas=len(snapshot['entries']))
                                except (m.PocError,ValueError,KeyError,OSError,RuntimeError) as exc:
                                    record.update(estado='FALLIDA',error=str(exc) if isinstance(exc,(m.PocError,ValueError)) else 'No se pudo comprobar o guardar esta bitácora.')
                                    if getattr(exc,'code',None):record['diagnostico']={'codigo_extension':exc.code}
                            checkpoint()
                            self.emit('diary_progress',{'pagina':number,'registros':1,'archivo':record.get('archivo',record['estado']),'rit':record['rit'],'estado':record['estado']})
                            if self.cancel.wait(.15):stopped()
                        query['paginas']=number;checkpoint()
                    query['estado']='ENUMERADA';checkpoint()
                except (m.PocError,ValueError,KeyError,OSError) as exc:
                    import traceback
                    trace=traceback.extract_tb(exc.__traceback__)
                    diagnostic={'fase':phase,'tipo':type(exc).__name__,
                        'ubicacion':[{'modulo':Path(t.filename).name,'linea':t.lineno} for t in trace]}
                    if isinstance(exc,KeyError) and len(exc.args)==1 and str(exc.args[0]) in m.ALLOWED_FIELDS|{'pairs','body','status','type','rit','nombre','ingreso_id'}:
                        diagnostic['campo']=str(exc.args[0])
                    query.update(estado='FALLIDA',diagnostico=diagnostic,error=str(exc) if isinstance(exc,m.PocError) else
                        'No se pudo comprobar esta consulta · '+phase+' · '+type(exc).__name__+'. Revisa el diagnóstico del lote.')
                    self.emit('warning',query['error'])
                    checkpoint()
                    if self.cancel.is_set() or not self.bridge.connected:break
            complete=len(state['consultas'])==len(plan) and all(q['estado'] in ('ENUMERADA','SIN_REGISTROS') for q in state['consultas'])
            state['estado']='CANCELADA' if self.cancel.is_set() else 'COMPLETA' if complete and all(r['estado']=='LEIDA' for r in state['registros']) else 'INCOMPLETA'
        except m.PocError as exc:
            state.update(estado='CANCELADA' if self.cancel.is_set() else 'INCOMPLETA',error=str(exc))
        finally:
            current_keys={r['clave'] for r in state['registros']}
            retained={r['clave']:r for r in previous_records if r['clave'] not in current_keys}
            state['anteriores_no_reenumeradas']=list(retained.values())
            checkpoint()
            # Archivo independiente para soporte técnico sin datos judiciales de NNA.
            write(folder/'diagnostico_bitacoras.json',anonymous_diagnostic(state))
            if locked:
                try:self.bridge.call('unlock',timeout=8)
                except Exception:pass
        value={'folder':str(folder),'manifest':str(manifest),'sha256':hashlib.sha256(manifest.read_bytes()).hexdigest(),
               'estado':state['estado'],'leidas':sum(r['estado']=='LEIDA' for r in state['registros']),
               'fallidas':sum(r['estado']!='LEIDA' for r in state['registros']),
               'consultas_fallidas':sum(q['estado']=='FALLIDA' for q in state['consultas']),
               'detalle':next((q.get('error','') for q in state['consultas'] if q['estado']=='FALLIDA'),state.get('error',''))}
        value['diagnostico_anonimo']=str(folder/'diagnostico_bitacoras.json')
        self.emit('diary_done',value)
        return value
