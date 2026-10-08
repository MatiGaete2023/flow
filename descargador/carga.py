"""Contrato de carga capturado: lectura completa, sin inferir cobertura de firmas."""
from collections import Counter
from datetime import date, datetime, timedelta
from hashlib import sha256
import json
import re

HEADERS=('RIT','TRIBUNAL','NOMBRE FUNCIONARIO','NOMBRE JUEZ','TRÁMITE','INVALIDADO','DOC. RES','DOC. ESC','DÍAS','CARGO',
    'PRIMER PREGRABADO FECHA','PRIMER PREGRABADO HORA','ÚLTIMO PREGRABADO FECHA','ÚLTIMO PREGRABADO HORA',
    'DESPACHO FECHA','DESPACHO HORA','DEVOLUCIÓN FECHA','DEVOLUCIÓN HORA','NUEVO DESPACHO FECHA','NUEVO DESPACHO HORA',
    'CANTIDAD ESCRITOS RESOLUCIÓN','FECHA FIRMA','HORA FIRMA')


def date_blocks(start, end, maximum=30):
    if not isinstance(start,date) or not isinstance(end,date) or start>end or not 1<=maximum<=30:
        raise ValueError('Rango de carga inválido; cada bloque admite hasta 30 días inclusivos.')
    result=[]
    while start<=end:
        stop=min(end,start+timedelta(days=maximum-1));result.append((start,stop));start=stop+timedelta(days=1)
    return result


def parse_day(value):
    if isinstance(value,datetime):return value.date()
    if isinstance(value,date):return value
    try:return datetime.strptime(str(value).strip(),'%d/%m/%Y').date()
    except ValueError:return None


def canonical(value):
    import motor as m
    if isinstance(value,datetime):value=value.strftime('%d/%m/%Y')
    if isinstance(value,float) and value.is_integer():value=int(value)
    return m.normalized(value)


def record_rows(grid):
    """Valida encabezado, orden y cantidad antes de leer filas posicionales."""
    _validate_excel_schema(grid)
    return _record_rows_unchecked(grid)


def _record_rows_unchecked(grid):
    """Lee filas posicionales de una tabla web o un reporte ya validado."""
    rows=[]
    for row in grid:
        if row and re.fullmatch(r'[A-Z]+-\d+-\d{4}',canonical(row[0])):
            if len(row)!=23:
                import motor as m
                raise m.PocError('Cambió el esquema de carga: se esperaban 23 columnas.')
            rows.append(tuple(canonical(v) for v in row))
    return rows


def _validate_excel_schema(grid):
    """Exige el encabezado completo y ordenado del XLS antes de leer sus filas."""
    expected=tuple(canonical(value) for value in HEADERS)
    known=set(expected);headers=[]
    for index,row in enumerate(grid):
        normalized=tuple(canonical(value) for value in row)
        # El umbral reconoce un encabezado aunque una columna haya sido renombrada,
        # sin confundirlo con filas de título como el texto fijo «Concepción».
        looks_like_header=(normalized and normalized[0]==expected[0]) or (len(normalized)==len(expected) and len(known.intersection(normalized))>=12)
        if looks_like_header:
            headers.append((index,row,normalized))
    if len(headers)!=1:
        import motor as m
        if not headers:
            received=next((tuple(row) for row in grid if row and re.fullmatch(r'[A-Z]+-\d+-\d{4}',canonical(row[0]))),())
            raise m.PocError('No se encontró el encabezado de carga esperado: '+repr(HEADERS)+'; recibido: '+repr(received)+'.')
        raise m.PocError('El informe contiene más de un encabezado de carga.')
    header_index,row,normalized=headers[0]
    if normalized!=expected:
        import motor as m
        raise m.PocError('Encabezado de carga inesperado; esperado: '+repr(HEADERS)+'; recibido: '+repr(tuple(row))+'.')
    record_indices=[index for index,row in enumerate(grid) if row and re.fullmatch(r'[A-Z]+-\d+-\d{4}',canonical(row[0]))]
    if any(index<header_index for index in record_indices):
        import motor as m
        raise m.PocError('Hay registros antes del encabezado de carga esperado.')


def records_in_grid(grid):
    _validate_excel_schema(grid)
    return Counter(_record_rows_unchecked(grid))


def result_records(form, profile):
    import motor as m
    candidates=[_record_rows_unchecked(m.table_grid(t)) for t in form.xpath('.//table')]
    candidates=[rows for rows in candidates if rows]
    if len(candidates)>1:raise m.PocError('Más de un listado de carga; revisar la estructura.')
    return Counter(candidates[0]) if candidates else Counter()


def signatures(grid, tribunal_code, tribunal_name, start, end):
    """Solo firma válida no invalidada. No convierte glosas en decisiones."""
    import motor as m
    candidates=[];occurrences=Counter();legacy_ordinals=Counter();group_sizes=Counter()
    for row in record_rows(grid):
        if row[1]!=canonical(tribunal_name):raise m.PocError('Una fila de carga pertenece a otro tribunal.')
        signed=parse_day(row[21])
        if row[5] not in ('','--','NO') or signed is None or not start<=signed<=end:continue
        # Conservar el hash antiguo solo como alias exacto; su contenido original
        # no puede reconstruirse desde un store que guardó únicamente el hash.
        raw=json.dumps(row,ensure_ascii=False,separators=(',',':'))
        legacy_digest=sha256(raw.encode()).hexdigest();legacy_ordinals[legacy_digest]+=1
        hour=row[22] if re.fullmatch(r'\d{2}:\d{2}(?::\d{2})?',row[22]) else ''
        # El portal no entrega un ID remoto de resolución. La clave local usa
        # únicamente atributos de firma; cambios de funcionarios/flujo se excluyen.
        identity=(canonical(tribunal_code),row[1],row[0],row[4],row[6],signed.isoformat(),hour)
        identity_json=json.dumps(identity,ensure_ascii=False,separators=(',',':'))
        digest=sha256(('rus-load-signature-v2:'+identity_json).encode()).hexdigest()
        occurrences[identity]+=1;group_sizes[identity]+=1
        candidates.append({'tribunal_codigo':tribunal_code,'tribunal':tribunal_name,'rit':row[0],
            'tramite':row[4],'firma':signed.isoformat(),'hora':hour,'documento':row[6],
            '_identity':identity,'_digest':digest,'_occurrence':occurrences[identity],
            '_legacy':legacy_digest+':'+str(legacy_ordinals[legacy_digest])})
    result=[]
    for item in candidates:
        ambiguous=group_sizes[item['_identity']]>1
        alias_status='huella_compuesta_ambigua_sin_id_remoto' if ambiguous else 'huella_compuesta_local_sin_id_remoto'
        item['identidad']=alias_status+';huella_legacy='+item['_legacy']
        item['huella']=item['_digest']+':'+str(item['_occurrence'])
        for key in ('_identity','_digest','_occurrence','_legacy'):item.pop(key)
        result.append(item)
    return result


def coverage(start,end,queries,*,complete_history=False):
    # Un rango del reporte de carga no acredita el rango de firmas de la historia.
    valid=all(q.get('estado') in ('VALIDADA','SIN_RESULTADOS') for q in queries) and bool(queries)
    return {'desde':start.isoformat(),'hasta':end.isoformat(),
        'estado':'COMPLETA' if valid and complete_history else 'PARCIAL',
        'criterio_temporal':'historia_firmas' if complete_history else 'filtro_carga_no_confirmado',
        'sin_coincidencias':'Sin firmas en el período' if valid and complete_history else 'Sin coincidencias en los informes consultados'}
