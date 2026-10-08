"""Análisis común de bitácoras: hechos consultados, sin proponer ni registrar texto."""
from calendar import monthrange
from collections import defaultdict
from dataclasses import dataclass
from datetime import date, datetime
from difflib import SequenceMatcher
from hashlib import sha256
import json
from pathlib import Path

from nurus.rus.columns import normalize


# Repeated-text review is quadratic in the number of distinct observations.
# If this limit is reached, the caller must see a partial audit rather than a
# false assurance that no more possible repetitions exist.
MAX_DUPLICATE_PAIR_CHECKS = 50_000
EXCEL_CELL_TEXT_LIMIT = 32_767


def _utf16_units(text):
    return len(text.encode('utf-16-le'))//2


def _text_parts(text):
    """Split at word boundaries where possible, measuring Excel UTF-16 units."""
    parts=[];remaining=text
    while remaining:
        used=0;end=0;last_space=0
        for index,char in enumerate(remaining):
            units=2 if ord(char)>0xFFFF else 1
            if used+units>EXCEL_CELL_TEXT_LIMIT:break
            used+=units;end=index+1
            if char.isspace():last_space=end
        if end==0:raise ValueError('No se puede representar un carácter en el límite de Excel.')
        if end<len(remaining) and last_space:end=last_space
        parts.append(remaining[:end]);remaining=remaining[end:]
    return parts


def _externalize_text(text,*,key,entry_id,field,identity,sidecar_name,items,parts_rows):
    if _utf16_units(text)<=EXCEL_CELL_TEXT_LIMIT:
        return text,'','',''
    digest=sha256(text.encode('utf-8')).hexdigest()
    pieces=_text_parts(text)
    items.append({'id':key,'entry_id':entry_id,'field':field,'identity':identity,
                  'length_utf16':_utf16_units(text),'sha256':digest,'text':text,'parts':len(pieces)})
    for number,piece in enumerate(pieces,1):
        parts_rows.append([*identity,entry_id,field,number,len(pieces),piece,sidecar_name,key,digest])
    reference=f'Texto extenso en hoja «Textos largos», clave {key}'
    return reference,sidecar_name,key,digest


def earliest(today):
    month=today.year*12+today.month-1-4
    year,zero=divmod(month,12)
    return date(year,zero+1,min(today.day,monthrange(year,zero+1)[1]))


def validate_period(start,end,today=None):
    today=today or date.today()
    if not isinstance(start,date) or not isinstance(end,date) or start>end or end>today or start<earliest(today):
        raise ValueError('Las bitácoras admiten hasta los últimos cuatro meses calendario, sin fechas futuras.')


def cc_for_type(kind):
    key=normalize(kind)
    if key==normalize('Al Tribunal'):return 1
    if key==normalize('Administrativa'):return 0
    return None


@dataclass(frozen=True)
class Observation:
    tribunal_codigo: str
    causa_id: str
    ingreso_id: str
    entry_id: str
    fecha: str
    autor: str
    tipo: str
    etapa: str
    texto: str
    origen: str = 'desconocido'
    respuesta: str = ''
    fecha_respuesta: str = ''
    respuesta_comprobada: bool = False
    fuente: str = ''
    fila_fuente: int = 0
    rit: str = ''
    tribunal: str = ''
    persona: str = ''
    centro: str = ''

    def moment(self):
        try:return datetime.fromisoformat(self.fecha)
        except (TypeError,ValueError):return None

    def identity(self):
        base=(self.tribunal_codigo,self.causa_id,self.ingreso_id)
        if not all(base):raise ValueError('Una entrada carece de identidad de causa o ingreso.')
        if self.entry_id:return (*base,'remota',self.entry_id)
        if not self.fuente or self.fila_fuente<1:
            raise ValueError('Una entrada sin ID remoto necesita fuente y posición comprobables.')
        return (*base,'fuente',self.fuente,self.fila_fuente)


def deduplicate(entries):
    result={}
    for entry in entries:
        key=entry.identity()
        if key in result:
            old=result[key]
            facts=('fecha','autor','tipo','etapa','texto','origen','respuesta','fecha_respuesta','respuesta_comprobada')
            if any(getattr(old,k)!=getattr(entry,k) for k in facts):
                raise ValueError('Dos extracciones contienen datos distintos para una misma entrada de bitácora.')
        else:result[key]=entry
    return list(result.values())


def response_state(entry):
    if cc_for_type(entry.tipo)==0:return 'Administrativa; no exige respuesta del tribunal'
    if cc_for_type(entry.tipo) is None:return 'Tipo pendiente de clasificación'
    if not entry.respuesta_comprobada:return 'Respuesta no comprobable con esta lectura'
    return 'Respuesta registrada' if entry.respuesta.strip() else 'Sin respuesta registrada'


def analyze(entries,start,end,*,today=None,author=None,coverage='PARCIAL'):
    validate_period(start,end,today)
    if coverage not in ('COMPLETA','VACIA_COMPROBADA','PARCIAL','FALLIDA','NO_CONSULTADA'):
        raise ValueError('Cobertura de bitácora desconocida.')
    entries=list(entries)
    if coverage=='VACIA_COMPROBADA' and entries:
        raise ValueError('Una bitácora con entradas no puede tener cobertura vacía comprobada.')
    complete=deduplicate(entries);invalid=[];current=[]
    if len({(e.tribunal_codigo,e.causa_id,e.ingreso_id) for e in complete})>1:
        raise ValueError('Analiza cada ingreso por separado para no mezclar sus observaciones.')
    for entry in complete:
        moment=entry.moment()
        if moment is None:invalid.append(entry);continue
        if start<=moment.date()<=end and (not author or normalize(entry.autor)==normalize(author)):current.append(entry)
    if invalid and coverage=='COMPLETA':coverage='PARCIAL'
    groups=defaultdict(list)
    for entry in current:
        if entry.texto.strip():groups[normalize(entry.texto)].append(entry)
    repeated={e.identity() for items in groups.values() if len(items)>1 for e in items}
    # Compare every distinct-text pair unless the maximum possible ratio from
    # lengths alone is already below the threshold.  This bound is exact and
    # safe: 2*min(n,m)/(n+m) is an upper bound for SequenceMatcher's ratio.
    # Keep originals in their own groups; this only marks possible repetitions.
    possible=set();texts=sorted(groups);pair_checks=0;scan_complete=True
    for i,text in enumerate(texts):
        if len(text)<30:continue
        for other in texts[i+1:]:
            if len(other)<30:continue
            if pair_checks>=MAX_DUPLICATE_PAIR_CHECKS:
                scan_complete=False
                break
            pair_checks+=1
            # Use 23/25 exactly so floating-point rounding cannot skip a pair
            # whose upper bound is precisely the .92 threshold.
            if 50*min(len(text),len(other)) < 23*(len(text)+len(other)):
                continue
            if SequenceMatcher(None,text,other,autojunk=False).ratio()>=.92:
                possible.update(e.identity() for e in (*groups[text],*groups[other]))
        if not scan_complete:break
    if not scan_complete:coverage='PARCIAL'
    center=[e for e in current if e.origen=='centro']
    # Ordenar fechas con y sin hora/zona sin inventar una zona del servidor.
    def key(e):return e.moment().replace(tzinfo=None)
    latest=[]
    if center:
        last=max(key(e) for e in center);latest=[e for e in center if key(e)==last]
    unknown_origin=sum(e.origen=='desconocido' for e in current)
    return {'entries':current,'latest_center':latest,'coverage':coverage,'invalid_dates':invalid,
            'unknown_origin':unknown_origin,'repeated':repeated,'possible_repeated':possible,
            'duplicate_scan_complete':scan_complete,'duplicate_pair_checks':pair_checks,
            'duplicate_scan_reason':('Se alcanzó el límite de comparación; la revisión de posibles reiteraciones está incompleta.'
                                     if not scan_complete else ''),
            'start':start.isoformat(),'end':end.isoformat()}


def export_audit(queries,destination,start,end,*,today=None,author=None,extra_sheets=()):
    """Queries contiene cada ingreso, incluso vacío/fallido: nunca desaparece del resumen."""
    from .reports import _book
    validate_period(start,end,today);history=[];summary=[];issues=[];parts_rows=[];sidecar_items=[]
    sidecar=Path(destination).with_suffix('.bitacora-textos.json');sidecar_name=sidecar.name
    for query_index,query in enumerate(queries,1):
        entries=query.get('entries',[])
        result=analyze(entries,start,end,today=today,author=author,coverage=query.get('coverage','NO_CONSULTADA'))
        identity=(query.get('tribunal_codigo',''),query.get('causa_id',''),query.get('ingreso_id',''))
        if not all(identity):raise ValueError('La consulta de bitácora no identifica su ingreso.')
        if any((e.tribunal_codigo,e.causa_id,e.ingreso_id)!=identity for e in entries):
            raise ValueError('El contenido de una bitácora no pertenece a su ingreso solicitado.')
        recent=result['entries'];last=result['latest_center']
        latest_text='\n\n'.join(e.texto for e in last)
        summary_key=f'resumen-{query_index}'
        latest_text,json_file,text_key,text_hash=_externalize_text(latest_text,key=summary_key,
            entry_id=';'.join(e.entry_id for e in last),field='Últimas observaciones del centro',
            identity=identity,sidecar_name=sidecar_name,items=sidecar_items,parts_rows=parts_rows)
        detail=[]
        if result['unknown_origin'] or result['invalid_dates']:
            detail.append('Última del centro pendiente de comprobar')
        if result['duplicate_scan_reason']:
            detail.append(result['duplicate_scan_reason'])
        summary.append([*identity,query.get('rit',''),result['coverage'],start.isoformat(),end.isoformat(),len(recent),
            last[0].fecha if last else '',latest_text,'Empate de fecha; conservar todas' if len(last)>1 else '',
            sum(cc_for_type(e.tipo)==1 for e in recent),sum(cc_for_type(e.tipo)==0 for e in recent),
            result['unknown_origin'],'; '.join(detail),json_file,text_key,text_hash])
        for entry in recent:
            entry_identity=entry.identity()
            stable_key='entrada-'+sha256(repr(entry_identity).encode('utf-8')).hexdigest()[:24]
            row=[*identity,entry.rit,entry.entry_id,entry.fecha,entry.autor,entry.tipo,cc_for_type(entry.tipo),
                entry.etapa,entry.origen,entry.texto,response_state(entry),entry.respuesta,entry.fecha_respuesta,
                entry.identity() in result['repeated'],entry.identity() in result['possible_repeated'],entry.fuente]
            text,row_json,row_key,row_hash=_externalize_text(entry.texto,key=stable_key,entry_id=entry.entry_id,
                field='Texto',identity=identity,sidecar_name=sidecar_name,items=sidecar_items,parts_rows=parts_rows)
            row[11]=text
            response,response_json,response_key,response_hash=_externalize_text(entry.respuesta,key=stable_key+'-respuesta',
                entry_id=entry.entry_id,field='Respuesta',identity=identity,sidecar_name=sidecar_name,
                items=sidecar_items,parts_rows=parts_rows)
            row[13]=response
            row.extend([row_json or response_json,row_key or response_key,row_hash if row_json else response_hash])
            history.append(row)
        for entry in result['invalid_dates']:issues.append([*identity,entry.entry_id,'Fecha ilegible',entry.fecha])
        if result['coverage'] not in ('COMPLETA','VACIA_COMPROBADA'):
            detail=result['duplicate_scan_reason'] or query.get('error','')
            issues.append([*identity,'','Lectura '+result['coverage'].lower(),detail])
    sheets=[
        ('Resumen',('Tribunal código','Causa RUS','Ingreso RUS','RIT','Cobertura','Desde','Hasta','Entradas recientes',
          'Última fecha centro','Texto','Ambigüedad','Con carga','Administrativas','Origen desconocido','Detalle',
          'JSON de textos extensos','Clave de texto','SHA-256 texto'),summary),
        ('Bitácoras',('Tribunal código','Causa RUS','Ingreso RUS','RIT','Entrada RUS','Fecha','Autor','Tipo','CC','Etapa',
          'Origen','Texto','Estado respuesta','Respuesta','Fecha respuesta','Texto reiterado','Posible reiteración','Fuente',
          'JSON de textos extensos','Clave de texto','SHA-256 texto'),history),
        ('Incidencias',('Tribunal código','Causa RUS','Ingreso RUS','Entrada RUS','Motivo','Detalle'),issues)]
    if parts_rows:
        sheets.append(('Textos largos',('Tribunal código','Causa RUS','Ingreso RUS','Entrada RUS','Campo','Parte','Total partes',
            'Texto','Archivo JSON UTF-8','Clave JSON','SHA-256 texto'),parts_rows))
    sheets.extend(extra_sheets)
    created_sidecar=False
    if sidecar_items:
        from nurus.services.file_output import write_new_file
        payload={'version':1,'workbook':Path(destination).name,'items':sidecar_items}
        write_new_file(sidecar,lambda path:Path(path).write_text(json.dumps(payload,ensure_ascii=False,indent=2),encoding='utf-8'))
        created_sidecar=True
    try:
        return _book(sheets,destination)
    except BaseException:
        if created_sidecar:sidecar.unlink(missing_ok=True)
        raise
