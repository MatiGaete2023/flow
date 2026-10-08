"""Solicitud local declarativa. Nunca contiene URLs, código ni comandos."""
import json
import os
from pathlib import Path
import tempfile
from .contract import CaptureError, json_object, HEX, iso_time

FIELDS={'record_id','source_system','court_code','cause_id','ingreso_id','source','reason'}
SOURCES={'bitacoras','historia_tramites','calendarios','documentos'}


def validate_request(data):
    if set(data)!={'schema_id','schema_version','capture_sha256','as_of','requests'} or data['schema_id']!='CSMP_REQUEST' or data['schema_version']!='1.0.0':raise CaptureError('Solicitud incompatible.')
    if data['capture_sha256'] and not HEX.fullmatch(data['capture_sha256']):raise CaptureError('Huella inválida.')
    iso_time(data['as_of'],True)
    if not isinstance(data['requests'],list) or len(data['requests'])>20000:raise CaptureError('Alcance de solicitud excesivo.')
    seen=set()
    for item in data['requests']:
        if not isinstance(item,dict) or set(item)!=FIELDS or any(not isinstance(v,str) or len(v)>2000 for v in item.values()):raise CaptureError('Campos de solicitud inválidos.')
        if not all(item[k] for k in ('record_id','source_system','court_code','cause_id','ingreso_id','source','reason')) or item['source'] not in SOURCES:raise CaptureError('Solicitud sin identidad/tipo válido.')
        key=tuple(item[k] for k in ('source_system','court_code','cause_id','ingreso_id','source'))
        if key in seen:raise CaptureError('Consulta repetida.')
        seen.add(key)
    return data


def read_request(path):
    path=Path(path)
    if path.stat().st_size>4*1024*1024:raise CaptureError('Solicitud demasiado grande.')
    return validate_request(json_object(path.read_text(encoding='utf-8-sig')))


def write_request(path,data):
    validate_request(data);path=Path(path);path.parent.mkdir(parents=True,exist_ok=True)
    if path.exists():raise CaptureError('El destino ya existe.')
    fd,temp=tempfile.mkstemp(prefix='.request-',dir=path.parent)
    try:
        with os.fdopen(fd,'w',encoding='utf-8') as stream:json.dump(data,stream,ensure_ascii=False,indent=2)
        os.link(temp,path)
    finally:Path(temp).unlink(missing_ok=True)
    return path
