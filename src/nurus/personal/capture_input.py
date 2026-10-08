"""Importador transaccional del paquete hacia Work; motor tradicional conservado."""
from collections import defaultdict
from copy import deepcopy
from datetime import date
from hashlib import sha256
from io import BytesIO
import json
from pathlib import Path
from openpyxl import Workbook
from nurus.capture import read_capture
from nurus.capture.contract import COMMON as SOURCE_FIELDS
from nurus.capture.temporal import chronology,identity,RULESET_VERSION
from nurus.rus.columns import COMMON,MODE_COLUMNS,map_columns
from nurus.rus.models import ReadBatch,SourceRecord,SourceReference,Mode
from nurus.rus.rules import as_date
from .work import Work,Row

CANONICAL={key:names[0] for key,names in COMMON.items()}
for columns in MODE_COLUMNS.values():CANONICAL.update({key:names[0] for key,names in columns.items()})


def import_capture(path,config,*,as_of=None,previous=None):
    from .operations import checkpoint,progress
    checkpoint();progress('Validando paquete y cobertura')
    capture=read_capture(path);as_of=as_of or date.today()
    config_hash=sha256(json.dumps(config,sort_keys=True,ensure_ascii=False,default=str).encode()).hexdigest()
    if previous and getattr(previous,'capture_digest','')==capture.digest and previous.as_of==as_of.isoformat() and getattr(previous,'capture_config_hash','')==config_hash and getattr(previous,'ruleset_version','')==RULESET_VERSION and getattr(previous,'mapping_version','')==capture.manifest['mapping_version']:
        return deepcopy(previous)
    records=list(capture.rows('registros'))
    # Un productor independiente puede aportar calendarios sin duplicarlos en
    # registros. Solo se omite una copia de la misma fila física, nunca por RIT.
    def physical(row):
        return tuple(row[k] for k in ('query_id','source_sha256','original_file','original_sheet','original_row'))
    represented={physical(row) for row in records}
    for calendar in capture.rows('calendarios'):
        if physical(calendar) in represented:continue
        record={k:calendar[k] for k in SOURCE_FIELDS}
        is_report=calendar['event_type'].strip().upper() in ('INFORME','INFORMES')
        record.update(record_id='calendar:'+calendar['calendar_id'],record_type='INFORMES' if is_report else 'OTRO',
                      identity_status='VERIFICADA' if calendar['association']=='VERIFICADA' else 'INCIERTA',
                      raw_json=calendar['raw_json'],normalized_json=calendar['normalized_json'],
                      source_table='calendarios',calendar_id=calendar['calendar_id'])
        if any(row['record_id']==record['record_id'] for row in records):
            raise ValueError('Identificador de calendario colisiona con registro; trabajo anterior intacto.')
        records.append(record)
    canonical=[];groups=defaultdict(list);provenance={};contexts={};row_modes={}
    calendars=defaultdict(list)
    for row in capture.rows('calendarios'):
        if row['association']=='VERIFICADA' and row['event_type'].strip().upper() in ('INFORME','INFORMES'):
            calendars[identity(row)].append(row)
    cross={}
    event_index=defaultdict(list)
    for table in ('bitacoras','historia_tramites'):
        for event in capture.rows(table):
            if event.get('identity_status')=='VERIFICADA' and event.get('scope','INGRESO')=='INGRESO':event_index[identity(event)].append(event)
    for number,record in enumerate(records,2):
        if number%100==0:checkpoint();progress('Relacionando fuentes por ingreso',number-1,len(records))
        raw=json.loads(record['normalized_json']);mode=record['record_type']
        if mode not in ('ESPERA','CUMPLIMIENTO','INFORMES'):mode='ESPERA'
        mapping=map_columns(raw,mode);values={CANONICAL[key]:raw[column] for key,column in mapping.items()}
        # Solo campos mapeados. Raw/normalizado completos quedan en procedencia.
        values.update(NURUS_TIPO_REGISTRO=record['record_type'],MODALIDAD=record['modality_raw'],SITFA_CAUSA_RUS=record['cause_id'],SITFA_INGRESO_RUS=record['ingreso_id'],SITFA_TRIBUNAL_CODIGO=record['court_code'],SITFA_SHA256=record['source_sha256'],SITFA_ARCHIVO=record['original_file'])
        canonical.append(values);groups[(mode,tuple(sorted(mapping)))].append((record,number,values,mapping));provenance[record['record_id']]=record;row_modes[record['record_id']]=mode
        contexts[record['record_id']]=chronology(capture,record,as_of,event_index=event_index)
        matching=calendars.get(identity(record),[]) if record['identity_status']=='VERIFICADA' else []
        dates=[]
        for cal in matching:
            source=json.loads(cal['normalized_json']);cm=map_columns(source,'INFORMES')
            due=as_date(source.get(cm.get('vencimiento',''),''))
            if due and due>as_of:dates.append(due)
        if len(set(dates))==1:cross[record['record_id']]=dates[0]
    headers=list(dict.fromkeys(key for row in canonical for key in row))
    if not headers:headers=['RIT','TRIBUNAL','NOMBRE','DERIVACION']
    book=Workbook();sheet=book.active;sheet.title='Trabajo';sheet.append(headers)
    for row in canonical:
        sheet.append([row.get(k,'') for k in headers])
        for cell in sheet[sheet.max_row]:
            if isinstance(cell.value,str):
                if len(cell.value.encode('utf-16-le'))//2>32767:raise ValueError('Campo de registro excede límite Excel; se conserva el paquete y el trabajo anterior.')
                cell.data_type='s'
    sheet.freeze_panes='A2';sheet.auto_filter.ref=sheet.dimensions
    out=BytesIO();book.save(out);book.close();content=out.getvalue();digest=sha256(content).hexdigest()
    candidate=Work(config);candidate.rows=[];candidate.mapping={key:title for key,title in CANONICAL.items() if title in headers}
    candidate.mode=next(iter(groups),('ESPERA',()))[0];candidate.sheet='Trabajo';candidate.header=1;candidate.as_of=as_of.isoformat()
    candidate.path=str(Path(path).resolve());candidate.content=content;candidate.source_hash=digest;candidate.sitfa_sources={};candidate.cross_dates={}
    for (mode,_),items in groups.items():
        checkpoint();progress('Analizando reglas tradicionales: '+mode)
        keys=set(key for _,_,_,mapping in items for key in mapping)
        mapping={key:CANONICAL[key] for key in keys}
        batch=ReadBatch(mode=Mode(mode),records=tuple(SourceRecord(values,SourceReference(Path(path).name,digest,'Trabajo',number),record['record_id']) for record,number,values,_ in items),source_path=str(path),workbook_name=Path(path).name,workbook_sha256=digest,primary_sheet='Trabajo',header_row=1,column_mapping=mapping,source_bytes=content)
        analyzed=Work(config).analyze(path,mode,as_of=as_of,batch=batch,cross_by_record=cross)
        candidate.warnings.extend(analyzed.warnings);candidate.cross_dates.update(analyzed.cross_dates)
        for row in analyzed.rows:
            record=provenance[row.id]
            if record['record_type'] not in ('ESPERA','CUMPLIMIENTO','INFORMES'):
                row.observation='';row.rules=[];row.actions=[];row.warnings.append('Tipo de fuente fuera del motor tradicional; no evaluado.')
            row.warnings.append(contexts[row.id]['reason'])
            candidate.rows.append(row)
    candidate.rows.sort(key=lambda row:row.source_row)
    candidate.capture_digest=capture.digest;candidate.capture_id=capture.manifest['capture_id'];candidate.capture_config_hash=config_hash
    candidate.capture_manifest=capture.manifest;candidate.capture_sources=provenance;candidate.capture_context=contexts;candidate.capture_modes=row_modes
    candidate.ruleset_version=RULESET_VERSION;candidate.mapping_version=capture.manifest['mapping_version']
    candidate.source_scope='PAQUETE';candidate.signed_activity={};candidate.activity_reviewed={}
    if previous and getattr(previous,'capture_digest','')==capture.digest:
        # Reanalizar otra fecha/configuración no sustituye decisiones humanas.
        old={row.id:row for row in previous.rows}
        for row in candidate.rows:
            prior=old.get(row.id)
            if prior:
                for field in ('review','decisions','overrides','word_overrides','sync_base','sync_local','sync_state'):
                    setattr(row,field,deepcopy(getattr(prior,field)))
                if row.overrides:
                    from .record_edits import recalculate
                    recalculate(candidate,row)
        candidate.session=deepcopy(previous.session);candidate.receipts=deepcopy(previous.receipts)
        candidate.output=previous.output;candidate.output_hash=previous.output_hash
        candidate.revision=previous.revision+1
    for row in candidate.rows:
        contexts[row.id]['findings'].extend({'rule_id':rule,'rule_version':RULESET_VERSION,'source_ids':[provenance[row.id]['source_sha256']],'record_identity':list(identity(provenance[row.id])),'detected_at':as_of.isoformat(),'fact':row.observation,'condition':'Regla tradicional conservada; no acredita actuaciones remotas.','coverage':provenance[row.id]['coverage'],'status':'APLICA','reason':'Evaluación del motor tradicional.','proposed_actions':list(row.actions),'confidence_class':'PARCIAL'} for rule in row.rules)
    candidate.warnings.append('Fuentes faltantes: '+', '.join(k+': '+v for k,v in capture.manifest['table_states'].items() if v not in ('COMPLETA','VACIA_COMPROBADA')))
    progress('Paquete validado; trabajo candidato disponible',len(records),len(records));return candidate
