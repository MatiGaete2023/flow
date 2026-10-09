"""CSMP_CAPTURE_V1: intercambio local, sin UI, red, Office ni extracción ZIP."""
import csv
from copy import deepcopy
from dataclasses import dataclass
from datetime import datetime
from hashlib import sha256
from io import StringIO, BytesIO
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import tempfile
import threading
import zipfile

VERSION = '1.0.0'
MAPPING_VERSION = 'baseline-20261008-v1'
STATES = ('NO_SOLICITADA','NO_CONSULTADA','COMPLETA','VACIA_COMPROBADA','PARCIAL','FALLIDA','CANCELADA','IDENTIDAD_INCIERTA','NO_DISPONIBLE')
COMMON = ('capture_id','query_id','source_system','court_code','court_name','rit_original','cause_id','ingreso_id','person_id','program_id','modality_raw','modality_code','tab','captured_at','original_file','original_sheet','original_row','source_sha256','coverage')
TABLES = {
    'consultas': ('query_id','source_system','screen','tab','court_code','court_name','modality_code','coverage','captured_at','filters_json','expected_pages','saved_pages','period_from','period_to','time_zone','history_complete','institutional_verified','error'),
    'registros': COMMON + ('record_id','record_type','identity_status','raw_json','normalized_json'),
    'bitacoras': COMMON + ('event_id','event_at','time_precision','time_zone','origin','origin_verified','author','event_type','text','response','response_at','identity_status'),
    'calendarios': COMMON + ('calendar_id','event_at','event_type','status','association','raw_json','normalized_json'),
    'historia_tramites': COMMON + ('event_id','event_at','time_precision','time_zone','scope','event_type','description','reference','status','document_id','identity_status'),
    'documentos': COMMON + ('document_id','parent_event_id','scope','reference','local_file','sha256','download_state','content_read'),
    'incidencias': ('capture_id','query_id','record_id','incident_id','phase','code','severity','message','recoverable'),
}
# Baseline: 200k filas, respuesta 80 MiB, HTML 8 MiB. Límites agregados conservadores.
MAX_BYTES = 80 * 1024 * 1024
MAX_FIELD = 8 * 1024 * 1024
MAX_ROWS = 200000
MAX_FILES = 2048
MAX_RATIO = 200
REQUIRED = ('consultas','registros','incidencias')
CSV_LOCK=threading.RLock()
HEX = re.compile(r'^[0-9a-f]{64}$')

class CaptureError(ValueError):
    pass


def fail(message):
    raise CaptureError(message)


def json_object(text):
    def pairs(items):
        result = {}
        for key,value in items:
            if key in result: fail('Clave JSON duplicada.')
            result[key] = value
        return result
    try:
        value = json.loads(text, object_pairs_hook=pairs, parse_constant=lambda _: fail('Número JSON no finito.'))
    except (ValueError, TypeError, RecursionError) as exc:
        raise CaptureError('JSON no válido.') from exc
    if not isinstance(value,dict): fail('Se esperaba un objeto JSON.')
    return value


def safe_path(name):
    if not isinstance(name,str) or not name or len(name)>240: fail('Ruta ZIP inválida.')
    path = PurePosixPath(name)
    if path.is_absolute() or '\\' in name or any(p in ('','..','.') for p in name.split('/')): fail('Ruta ZIP insegura.')
    if any(':' in p or p.endswith((' ','.')) or re.search(r'[\x00-\x1f]',p) or p.split('.')[0].upper() in {'CON','PRN','AUX','NUL',*(f'COM{i}' for i in range(1,10)),*(f'LPT{i}' for i in range(1,10))} for p in path.parts): fail('Ruta incompatible con Windows.')
    return name


def iso_time(value, date_allowed=False):
    if not isinstance(value,str): fail('Fecha debe ser texto.')
    try:
        if date_allowed and re.fullmatch(r'\d{4}-\d{2}-\d{2}',value):
            datetime.strptime(value,'%Y-%m-%d'); return
        if not re.fullmatch(r'\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,6})?(?:Z|[+-]\d{2}:\d{2})?',value): fail('Fecha ISO 8601 inválida.')
        datetime.fromisoformat(value.replace('Z','+00:00'))
    except ValueError as exc: raise CaptureError('Fecha ISO 8601 inválida.') from exc


@dataclass(frozen=True)
class Capture:
    manifest: dict
    tables: dict
    digest: str

    def rows(self,table):
        return self.tables.get(table,())


def csv_bytes(table,rows):
    stream=StringIO(newline=''); writer=csv.DictWriter(stream,fieldnames=TABLES[table],lineterminator='\n')
    writer.writeheader()
    for row in rows:
        if set(row)-set(TABLES[table]): fail('Columnas no definidas en '+table)
        if any(not isinstance(v,str) for v in row.values()): fail('Valores CSV deben ser texto; IDs nunca números.')
        writer.writerow({k:row.get(k,'') for k in TABLES[table]})
    return stream.getvalue().encode('utf-8')


def validate_manifest(m):
    if m.get('schema_id')!='CSMP_CAPTURE': fail('Contrato desconocido.')
    version=m.get('schema_version','')
    if not isinstance(version,str) or not re.fullmatch(r'1\.\d+\.\d+',version): fail('Versión mayor incompatible.')
    for key in ('capture_id','mapping_version'):
        if not isinstance(m.get(key),str) or not m[key].strip(): fail('Falta '+key)
    if m.get('required_features',[])!=[]: fail('El paquete requiere capacidades no soportadas.')
    iso_time(m.get('captured_at'))
    if not isinstance(m.get('contains_sensitive_data'),bool) or not isinstance(m.get('originals_authorized'),bool): fail('Política de datos no declarada.')
    downloader=m.get('downloader')
    if not isinstance(downloader,dict) or not all(isinstance(downloader.get(k),str) and downloader[k] for k in ('version','commit')): fail('Productor no identificado.')
    for name in ('files','queries'):
        if not isinstance(m.get(name),list): fail('Falta lista '+name)
    if not isinstance(m.get('table_states'),dict) or set(m['table_states'])!=set(TABLES): fail('Declarar estados de todas las tablas.')
    if any(s not in STATES for s in m['table_states'].values()): fail('Estado de tabla inválido.')
    query_ids=set()
    for q in m['queries']:
        if not isinstance(q,dict) or not isinstance(q.get('query_id'),str) or not q['query_id'] or q['query_id'] in query_ids: fail('ID de consulta inválido o duplicado.')
        query_ids.add(q['query_id'])
        for key in TABLES['consultas']:
            if not isinstance(q.get(key),str): fail('Consulta sin campo '+key)
        if q['coverage'] not in STATES: fail('Cobertura inválida.')
    seen=set()
    for f in m['files']:
        if not isinstance(f,dict): fail('Archivo inválido.')
        name=safe_path(f.get('path'))
        if name.casefold() in seen or name=='manifest.json': fail('Archivo declarado dos veces.')
        seen.add(name.casefold())
        if not HEX.fullmatch(str(f.get('sha256',''))): fail('Huella inválida.')
        if type(f.get('bytes')) is not int or not 0<=f['bytes']<=MAX_BYTES: fail('Tamaño inválido.')
        if f.get('type')=='normalized_table':
            table=f.get('table')
            if table not in TABLES or name!=f'data/{table}.csv': fail('Tabla o ruta desconocida.')
            if type(f.get('rows')) is not int or not 0<=f['rows']<=MAX_ROWS: fail('Número de filas inválido.')
        elif f.get('type')=='original':
            if not m['originals_authorized'] or not name.startswith(('originales/','evidencias/')): fail('Original no autorizado.')
        else: fail('Tipo de archivo desconocido.')


def _validate_query(q):
    """Fechas, paginación y garantías antes de relacionar filas."""
    iso_time(q['captured_at'])
    for k in ('expected_pages','saved_pages'):
        if not q[k].isdigit(): fail('Páginas inválidas.')
    if int(q['saved_pages'])>int(q['expected_pages']): fail('Cobertura de páginas inconsistente.')
    if q['coverage']=='COMPLETA' and q['saved_pages']!=q['expected_pages']: fail('Consulta completa con páginas faltantes.')
    for k in ('history_complete','institutional_verified'):
        if q[k] not in ('true','false'): fail('Garantía de fuente inválida.')
    json_object(q['filters_json'])
    for k in ('period_from','period_to'):
        if q[k]: iso_time(q[k],True)
    if q['period_from'] and q['period_to'] and q['period_from']>q['period_to']: fail('Período invertido.')


def _validate_rows(m,tables):
    queries={q['query_id']:q for q in m['queries']}
    if list(tables['consultas'])!=m['queries']: fail('Consultas CSV/manifiesto no coinciden.')
    for q in queries.values():
        _validate_query(q)
    record_ids=set(); event_ids=set(); document_ids=set(); calendar_ids=set(); incident_ids=set()
    for table,rows in tables.items():
        if table=='consultas': continue
        for row in rows:
            if row['capture_id']!=m['capture_id']: fail('Fila de otra captura.')
            if row['query_id'] not in queries: fail('Consulta referenciada no existe.')
            q=queries[row['query_id']]
            if table!='incidencias':
                for k in ('court_code','source_system','modality_code'):
                    if row[k]!=q[k]: fail('Identidad de consulta cruzada.')
                if row['coverage'] not in STATES: fail('Cobertura de fila inválida.')
                if row['source_sha256'] and not HEX.fullmatch(row['source_sha256']): fail('Huella de procedencia inválida.')
                if not row['source_sha256'] or not row['original_file'] or not row['original_row'].isdigit() or int(row['original_row'])<1: fail('Procedencia incompleta.')
                iso_time(row['captured_at'])
                if 'identity_status' in row and row['identity_status'] not in ('VERIFICADA','INCIERTA','CAUSA'): fail('Estado de identidad inválido.')
                if row.get('identity_status')=='VERIFICADA' and not all(row[k] for k in ('court_code','cause_id','ingreso_id')): fail('Identidad remota incompleta.')
                if row.get('event_at'): iso_time(row['event_at'],True)
                for k in ('time_precision',):
                    if k in row and row[k] not in ('DIA','SEGUNDO','DESCONOCIDA'): fail('Precisión desconocida.')
                for k in ('raw_json','normalized_json'):
                    if k in row: json_object(row[k])
            if table=='registros':
                if row['record_type'] not in ('ESPERA','CUMPLIMIENTO','INFORMES','EGRESADOS','OTRO'): fail('Tipo de registro inválido.')
                key,seen='record_id',record_ids
            elif table in ('bitacoras','historia_tramites'):
                key,seen='event_id',event_ids
                if table=='bitacoras' and (row['origin'] not in ('CENTRO','OTRO','DESCONOCIDO') or row['origin_verified'] not in ('true','false')): fail('Origen no válido.')
                if row.get('scope')=='CAUSA' and row['ingreso_id']: fail('Trámite de causa no atribuible a ingreso.')
                if row.get('scope') and row['scope'] not in ('CAUSA','INGRESO'): fail('Alcance inválido.')
                if row.get('response_at'): iso_time(row['response_at'],True)
            elif table=='documentos':
                key,seen='document_id',document_ids
                if row['content_read']!='false': fail('Este contrato no acredita lectura de documentos.')
                if row['download_state'] not in ('NO_SOLICITADA','NO_DISPONIBLE','FALLIDA','DESCARGADO'): fail('Estado documental inválido.')
                if row['download_state']=='DESCARGADO':
                    f=next((f for f in m['files'] if f['path']==row['local_file']),None)
                    if not f or f['type']!='original' or f['sha256']!=row['sha256']: fail('Documento sin archivo íntegro autorizado.')
            elif table=='calendarios':
                key,seen='calendar_id',calendar_ids
                if row['association'] not in ('VERIFICADA','NO_ASOCIADO'): fail('Asociación inválida.')
                if row['association']=='VERIFICADA' and not all(row[k] for k in ('court_code','cause_id','ingreso_id')): fail('Calendario sin identidad completa.')
            else: key,seen='incident_id',incident_ids
            if not row[key] or row[key] in seen: fail('ID vacío o duplicado en '+table)
            seen.add(row[key])
    events={row['event_id']:row for table in ('bitacoras','historia_tramites') for row in tables.get(table,())}
    for d in tables.get('documentos',()):
        if d['parent_event_id'] not in event_ids: fail('Documento sin trámite padre.')
        parent=events[d['parent_event_id']]
        if any(d[key]!=parent[key] for key in ('source_system','court_code','cause_id','ingreso_id')) or d['scope']!=parent.get('scope','INGRESO'):fail('Documento de otra identidad o alcance.')
    for event in tables.get('historia_tramites',()):
        if event['document_id'] and event['document_id'] not in document_ids:fail('Trámite refiere documento inexistente.')
    for r in tables['incidencias']:
        if r['record_id'] and r['record_id'] not in record_ids: fail('Incidencia con registro inexistente.')
    for table in TABLES:
        state=m['table_states'][table];rows=tables.get(table,())
        if state in ('NO_SOLICITADA','NO_CONSULTADA','VACIA_COMPROBADA','FALLIDA','CANCELADA','NO_DISPONIBLE') and rows and table not in REQUIRED: fail('Estado de tabla incompatible con filas.')
        if state=='VACIA_COMPROBADA' and rows: fail('Tabla vacía declarada con filas.')
        if table not in tables and state not in ('NO_SOLICITADA','NO_CONSULTADA','NO_DISPONIBLE','FALLIDA','CANCELADA'): fail('Tabla disponible omitida.')
    from .coverage import validate_coverage
    validate_coverage(m,tables,fail)


def read_capture(path):
    """Valida por completo antes de retornar; nunca extrae originales ni toca sesiones."""
    path=Path(path)
    if path.stat().st_size>MAX_BYTES: fail('ZIP supera 80 MiB.')
    raw_package=path.read_bytes()
    if len(raw_package)>MAX_BYTES:fail('ZIP supera 80 MiB.')
    digest=sha256(raw_package).hexdigest()
    try:
        with zipfile.ZipFile(BytesIO(raw_package)) as archive:
            infos=archive.infolist()
            if len(infos)>MAX_FILES: fail('Demasiados archivos ZIP.')
            names=set(); total=0
            for info in infos:
                name=safe_path(info.filename)
                mode=info.external_attr>>16
                if name.casefold() in names or info.is_dir() or stat.S_ISLNK(mode) or (stat.S_IFMT(mode) not in (0,stat.S_IFREG)): fail('Duplicado o enlace ZIP no permitido.')
                names.add(name.casefold());total+=info.file_size
                if info.flag_bits&1 or info.compress_type not in (zipfile.ZIP_STORED,zipfile.ZIP_DEFLATED): fail('Cifrado/compresión no permitidos.')
                if info.file_size>MAX_BYTES or info.file_size/max(1,info.compress_size)>MAX_RATIO or total>MAX_BYTES: fail('Expansión ZIP excede límite.')
            if 'manifest.json' not in names: fail('Sin manifiesto.')
            if archive.getinfo('manifest.json').file_size>4*1024*1024: fail('Manifiesto demasiado grande.')
            m=json_object(archive.read('manifest.json').decode('utf-8-sig'));validate_manifest(m)
            declared={'manifest.json',*(f['path'] for f in m['files'])}
            if set(i.filename for i in infos)!=declared: fail('Archivos ZIP no coinciden con manifiesto.')
            tables={}; count=0
            with CSV_LOCK:
                old_limit=csv.field_size_limit()
                try:
                    csv.field_size_limit(MAX_FIELD)
                    for f in m['files']:
                        raw=archive.read(f['path'])
                        if len(raw)!=f['bytes'] or sha256(raw).hexdigest()!=f['sha256']: fail('Hash o tamaño incorrecto.')
                        if f['type']!='normalized_table': continue
                        table=f['table'];reader=csv.DictReader(StringIO(raw.decode('utf-8-sig'),newline=''),strict=True)
                        fields=reader.fieldnames or []
                        if fields[:len(TABLES[table])]!=list(TABLES[table]) or len(set(fields))!=len(fields) or any(not field for field in fields): fail('Encabezado de tabla incompatible.')
                        rows=[]
                        for row in reader:
                            if None in row or any(v is None or '\x00' in v for v in row.values()): fail('Fila CSV irregular.')
                            rows.append(row);count+=1
                            if count>MAX_ROWS: fail('Captura supera 200.000 filas.')
                        if len(rows)!=f['rows']: fail('Cantidad de filas incorrecta.')
                        tables[table]=rows
                finally: csv.field_size_limit(old_limit)
            if not all(t in tables for t in REQUIRED): fail('Falta tabla requerida.')
            _validate_rows(m,tables)
            return Capture(deepcopy(m),tables,digest)
    except (zipfile.BadZipFile,UnicodeError,csv.Error,KeyError,EOFError,RuntimeError) as exc:
        raise CaptureError('Paquete corrupto o incompatible.') from exc


def write_capture(destination,metadata,tables,*,originals=None):
    """Publicación nueva y atómica: validar temporal y enlazar sin sobrescribir."""
    destination=Path(destination);destination.parent.mkdir(parents=True,exist_ok=True)
    if destination.exists(): fail('El paquete de destino ya existe.')
    if metadata.get('schema_version',VERSION)!=VERSION: fail('El productor solo escribe versión 1.0.0.')
    m=deepcopy(metadata);m.update(schema_id='CSMP_CAPTURE',schema_version=VERSION,files=[])
    m.setdefault('mapping_version',MAPPING_VERSION);m.setdefault('required_features',[])
    payload={}
    for table,rows in tables.items():
        if table not in TABLES: fail('Tabla desconocida.')
        raw=csv_bytes(table,rows);name=f'data/{table}.csv';payload[name]=raw
        m['files'].append({'path':name,'table':table,'type':'normalized_table','rows':len(rows),'bytes':len(raw),'sha256':sha256(raw).hexdigest()})
    for name,raw in (originals or {}).items():
        safe_path(name);payload[name]=raw
        m['files'].append({'path':name,'type':'original','bytes':len(raw),'sha256':sha256(raw).hexdigest()})
    payload['manifest.json']=json.dumps(m,ensure_ascii=False,sort_keys=True,indent=2).encode('utf-8')
    fd,temp=tempfile.mkstemp(prefix='.capture-',suffix='.zip',dir=destination.parent);os.close(fd)
    try:
        with zipfile.ZipFile(temp,'w',compression=zipfile.ZIP_DEFLATED) as archive:
            for name,raw in payload.items():
                # Fecha técnica ZIP fija; tiempos de captura están en el manifiesto.
                info=zipfile.ZipInfo(name,(2026,10,8,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED
                info.external_attr=(stat.S_IFREG|0o600)<<16;archive.writestr(info,raw)
        read_capture(temp)
        os.link(temp,destination)
    finally: Path(temp).unlink(missing_ok=True)
    return destination
