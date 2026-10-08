"""Actas estructuradas sin texto libre ni datos personales; no aprueba gates."""
from collections import Counter
from datetime import datetime,timezone
import json
from pathlib import Path
import re

from .contract import CaptureError,json_object
from nurus.services.file_output import write_new_file

CASES=tuple([f'{prefix}{i:02d}' for prefix,size in
             (('D-T',20),('A-T',20),('R-T',32),('INT-',27)) for i in range(1,size+1)]+[f'F{i:02d}' for i in range(16)])
RESULTS=('PASÓ','FALLÓ','NO_EJECUTADO','NO_APLICA')
KINDS=('SINTETICA_LOCAL','SINTETICA_WINDOWS','INSTITUCIONAL')
ENVIRONMENTS={'SINTETICA_LOCAL':('LINUX','WINDOWS','MACOS'),
              'SINTETICA_WINDOWS':('WINDOWS_CI',),
              'INSTITUCIONAL':('WINDOWS_10_INSTITUCIONAL','WINDOWS_INSTITUCIONAL')}
LIMITATIONS=('FALTA_ENTORNO','FALTA_EVIDENCIA','FALTA_OFFICE','FALTA_CONTRATO_REMOTO',
             'FALTA_APROBACION_FUNCIONAL','COBERTURA_PARCIAL','IDENTIDAD_INCIERTA',
             'FUERA_DEL_ALCANCE','FALLO_TECNICO','DIFERENCIA_BASELINE','NO_APLICA_AL_ARTEFACTO')
COUNTS=('expected','observed','matched','failed','unknown')
ARTIFACT_FIELDS={'source_commit','assistant_version','downloader_version','extension_version'}
RECORD_FIELDS={'case','gate','evidence_kind','environment','result','executed_at','counts',
               'limitations','evidence_retained_locally','evidence_sha256','operator_reference'}


def fail(message):raise CaptureError('Acta: '+message)


def timestamp(value):
    if not isinstance(value,str) or not re.fullmatch(r'\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:Z|[+-]\d{2}:\d{2})',value):fail('fecha con zona requerida.')
    try:parsed=datetime.fromisoformat(value.replace('Z','+00:00'))
    except ValueError:fail('fecha inválida.')
    return parsed


def validate_acta(data):
    if not isinstance(data,dict) or set(data)!={'schema_id','schema_version','created_at','artifact','records'}:fail('campos de documento inválidos.')
    if data['schema_id']!='CSMP_ACCEPTANCE' or data['schema_version']!='1.0.0':fail('versión incompatible.')
    timestamp(data['created_at'])
    artifact=data['artifact']
    if not isinstance(artifact,dict) or set(artifact)!=ARTIFACT_FIELDS:fail('artefacto incompleto.')
    if not isinstance(artifact['source_commit'],str) or not re.fullmatch(r'[0-9a-f]{40}',artifact['source_commit']):fail('SHA de código inválido.')
    for key in ARTIFACT_FIELDS-{'source_commit'}:
        if not isinstance(artifact[key],str) or not re.fullmatch(r'\d+\.\d+\.\d+(?:[.-]?(?:dev|a|b|rc)\d+)?',artifact[key]):fail('versión de componente inválida.')
    records=data['records']
    if not isinstance(records,list) or not 1<=len(records)<=2000:fail('cantidad de casos inválida.')
    seen=set()
    for record in records:
        if not isinstance(record,dict) or set(record)!=RECORD_FIELDS:fail('campos de caso inválidos.')
        # No se incluyen causas, ingresos, nombres, RUT, rutas, URLs ni glosas.
        if any(not isinstance(record[key],str) for key in
               ('case','gate','evidence_kind','environment','result','executed_at','operator_reference')):fail('tipo de campo inválido.')
        if record['case'] not in CASES or record['gate'] not in ('',*(f'G{i}' for i in range(14))):fail('caso/gate desconocido.')
        if record['evidence_kind'] not in KINDS or record['result'] not in RESULTS:fail('tipo de evidencia/resultado desconocido.')
        key=(record['case'],record['gate'],record['evidence_kind'])
        if key in seen:fail('caso repetido para gate y tipo de evidencia.')
        seen.add(key)
        counts=record['counts']
        if not isinstance(counts,dict) or set(counts)-set(COUNTS) or any(type(v) is not int or not 0<=v<=200000 for v in counts.values()):fail('conteos inválidos.')
        limits=record['limitations']
        if not isinstance(limits,list) or any(not isinstance(v,str) or v not in LIMITATIONS for v in limits) or len(set(limits))!=len(limits):fail('limitaciones deben ser códigos conocidos sin duplicados.')
        if type(record['evidence_retained_locally']) is not bool:fail('custodia local debe ser booleana.')
        hashes=record['evidence_sha256']
        if not isinstance(hashes,list) or len(hashes)>20 or any(not isinstance(v,str) or not re.fullmatch(r'[0-9a-f]{64}',v) for v in hashes) or len(set(hashes))!=len(hashes):fail('huellas de evidencia inválidas.')
        if record['result']=='NO_EJECUTADO':
            if any(record[k] for k in ('executed_at','environment','operator_reference','counts','evidence_sha256','evidence_retained_locally')):fail('caso no ejecutado contiene comprobantes de ejecución.')
        elif record['result']=='NO_APLICA':
            if 'NO_APLICA_AL_ARTEFACTO' not in limits:fail('no aplica requiere motivo de alcance.')
            if any(record[k] for k in ('executed_at','environment','operator_reference','counts','evidence_sha256','evidence_retained_locally')):fail('no aplica no es una ejecución.')
        else:
            timestamp(record['executed_at'])
            if not record['gate'] or record['environment'] not in ENVIRONMENTS[record['evidence_kind']]:fail('ejecución sin gate/entorno coherente.')
            if not re.fullmatch(r'OP-[0-9A-F]{12}',record['operator_reference']):fail('referencia opaca de operador requerida.')
            if not record['evidence_retained_locally'] or not hashes:fail('ejecución sin custodia local y huella de evidencia.')
            if record['result']=='PASÓ' and counts.get('failed',0)>0:fail('resultado aprobado con fallos declarados.')
            if record['result']=='FALLÓ' and not limits:fail('fallo sin código de limitación.')
    return data


def template(artifact,*,cases=CASES,gate='',evidence_kind='INSTITUCIONAL'):
    now=datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
    data={'schema_id':'CSMP_ACCEPTANCE','schema_version':'1.0.0','created_at':now,
          'artifact':dict(artifact),'records':[
              {'case':case,'gate':gate,'evidence_kind':evidence_kind,'environment':'',
               'result':'NO_EJECUTADO','executed_at':'','counts':{},'limitations':[],
               'evidence_retained_locally':False,'evidence_sha256':[],'operator_reference':''} for case in cases]}
    return validate_acta(data)


def read_acta(path):
    path=Path(path)
    if path.stat().st_size>1024*1024:fail('documento excede 1 MiB.')
    raw=path.read_bytes()
    if len(raw)>1024*1024:fail('documento excede 1 MiB.')
    try:data=json_object(raw.decode('utf-8-sig'))
    except UnicodeDecodeError:fail('se requiere UTF-8.')
    return validate_acta(data)


def write_acta(path,data):
    validate_acta(data)
    raw=json.dumps(data,ensure_ascii=False,indent=2).encode('utf-8')+b'\n'
    if len(raw)>1024*1024:fail('documento excede 1 MiB.')
    return write_new_file(path,lambda p:Path(p).write_bytes(raw))


def summary(data):
    validate_acta(data)
    return {'source_commit':data['artifact']['source_commit'],'records':len(data['records']),
            'reported_results':{kind:dict(Counter(r['result'] for r in data['records'] if r['evidence_kind']==kind)) for kind in KINDS},
            'validation_scope':'ESTRUCTURA_Y_COHERENCIA_DECLARADA',
            'gate_acceptance':'PENDIENTE_REVISION_HUMANA'}
