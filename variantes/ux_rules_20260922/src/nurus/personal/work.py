"""Un trabajo local compartido: analizar, exportar y recuperar cambios humanos."""
from copy import deepcopy
from dataclasses import dataclass, field, asdict
from datetime import date
from hashlib import sha256
from pathlib import Path
import json

from nurus.rus.columns import normalize
from nurus.rus.reader import read_workbook
from nurus.rus.rules import as_date, as_int, tribunal, historical_match
from nurus.services.exports import _export_preserved_payload
from .config import atomic_json
from .motor.contexto import current
from .motor.composicion import Incidencias
from .motor.reglas_espera import generar_observacion_espera
from .motor.reglas_cumplimiento import generar_observacion_cumplimiento
from .motor.reglas_informes import generar_observacion_informes


@dataclass
class Row:
    id: str
    source_row: int
    values: dict
    observation: str
    rules: list
    actions: list
    warnings: list
    excluded: bool = False
    review: dict = field(default_factory=dict)


def actions_for(events):
    """Selección por identificador de rama, nunca por palabras de la observación."""
    actions=[]
    if any(e.endswith(('E05_SOLO_CORREO','E05_PROYECTO_Y_CORREO')) for e in events):actions.append('programa_espera')
    if 'ESPERA.E05_PROYECTO_Y_CORREO' in events:actions.append('PC_IE')
    if any(e.startswith('INFORMES.I01_') for e in events):actions.extend(['programa_vencido','PC_INFO'])
    if any(e.startswith('INFORMES.I02_') for e in events):actions.append('programa_por_vencer')
    if any(e.startswith(('CUMPLIMIENTO.C04_','CUMPLIMIENTO.C05_')) for e in events):actions.append('medidas')
    return actions


def _explicit(value):
    if value is None:return False
    if isinstance(value,str):return bool(value.strip())
    return True


def _human_reviewed(row):
    """Distingue una edición humana real de columnas técnicas reimportadas vacías."""
    review=row.review or {}
    if 'OBSERVACION' in review and str(review.get('OBSERVACION',''))!=str(row.observation or ''):return True
    return any(_explicit(review.get(key)) for key in ('FECHA_OBS','TT','CC','RES'))


def _source_value(row,key,default=''):
    wanted=normalize(key)
    for column,value in row.values.items():
        if normalize(column)==wanted:return value
    return default


def _review_value(row,key,fallback=None):
    if key in (row.review or {}):return row.review.get(key,'')
    if fallback is not None:return fallback
    return _source_value(row,key,'')


def _sync_resolution_warning(row):
    from .resolutions import resolution_review_issue
    prefix='RES no reconocido:'
    row.warnings=[item for item in row.warnings if not str(item).startswith(prefix)]
    issue=resolution_review_issue((row.review or {}).get('RES'))
    if issue:row.warnings.append(issue)

class Work:
    def __init__(self, config):
        self.config=deepcopy(config)
        self.rows=[]
        self.path=''
        self.output=''
        self.warnings=[]
        self.exception=''
        self.output_hash=''
        self.receipts={}
        # Campos conservados para recuperar sesiones antiguas. En el flujo vigente
        # la ausencia del cruce de Cumplimiento es solo una advertencia.
        self.needs_cross=False
        self.cross_missing=False

    def analyze(self,path,mode,*,sheet=None,cross_sheet=None,as_of=None):
        batch=read_workbook(path,mode,sheet_name=sheet,cross_sheet_name=cross_sheet)
        self.path=batch.source_path
        self.mode=str(batch.mode)
        self.as_of=(as_of or date.today()).isoformat()
        self.sheet=batch.primary_sheet
        self.header=batch.header_row
        self.mapping=dict(batch.column_mapping)
        self.source_hash=batch.workbook_sha256
        self.content=batch.source_bytes
        self.warnings=list(batch.warnings)
        self.needs_cross=False
        self.cross_missing=self.mode=='CUMPLIMIENTO' and not batch.cross_records
        self.rows=[]
        index={}
        key_fields=('rit','rut','nombre','tribunal','programa')
        # Catálogo Asistente G-08: última fila futura para la identidad completa.
        for row in batch.cross_records:
            cm=batch.cross_mapping
            due=as_date(row.values.get(cm.get('vencimiento','')),batch.excel_epoch)
            if due and due>date.fromisoformat(self.as_of) and all(cm.get(k) for k in key_fields):
                key=tuple(historical_match(row.values.get(cm[k],'')) for k in key_fields)
                if key in index and index[key]!=due:self.warnings.append('Cruce duplicado: se usa la última fila según el perfil Asistente.')
                index[key]=due
        if self.mode=='CUMPLIMIENTO' and (not batch.cross_records or not all(batch.cross_mapping.get(k) for k in (*key_fields,'vencimiento'))):
            self.cross_missing=True
            self.needs_cross=False
            self.warnings.append('Sin cruce utilizable: no se evalúa C-10. El resto del análisis y la exportación continúan sin bloqueo.')
        cols=dict(self.mapping)
        fn={'ESPERA':generar_observacion_espera,'CUMPLIMIENTO':generar_observacion_cumplimiento,'INFORMES':generar_observacion_informes}[self.mode]
        for record in batch.records:
            values=dict(record.values)
            for name in ('nacimiento','resolucion','ingreso','egreso_proy','oido','prox_aud','ficha_ind','ficha_fae','vencimiento'):
                column=cols.get(name)
                if column and isinstance(values.get(column),(int,float)):
                    values[column]=as_date(values[column],batch.excel_epoch)
            ctx={'config':self.config,'events':[],'date':date.fromisoformat(self.as_of),'invalid':[]}
            token=current.set(ctx)
            inc=Incidencias()
            warnings=[]
            try:
                if self.mode=='CUMPLIMIENTO':
                    end=as_date(values.get(cols.get('egreso_proy','')),batch.excel_epoch)
                    counts=[as_int(values.get(cols.get(k,''))) for k in ('dias_cumpl','dias_egresar')]
                    if end and end>ctx['date'] and any(n is not None and n<0 for n in counts):
                        warnings.append('Días negativos y egreso futuro: verifica la contradicción; no se afirma que la medida esté vencida.')
                        ctx['invalid'].append('CUMPLIMIENTO.C04_VENCIDA')
                court=tribunal(values.get(cols.get('tribunal',''),'')) or ''
                if not court:warnings.append('Tribunal no reconocido; verifica acciones sugeridas.')
                key=tuple(historical_match(values.get(cols.get(k,''),'')) for k in key_fields)
                args={'incidencias':inc,'fila_excel':record.source.row_number}
                if self.mode=='CUMPLIMIENTO':args['fecha_hoja2']=index.get(key)
                observation=fn(values,court,cols,**args)
                warnings.extend(i['MOTIVO'] for i in inc._items)
            except (ValueError,TypeError,KeyError) as exc:
                observation=''
                warnings.append('No se pudo evaluar esta fila: '+str(exc))
            finally:current.reset(token)
            events=[e for e in ctx['events'] if e not in self.config['desactivadas']]
            excluded='COMUN.NO_SEGUIMIENTO' in events
            self.rows.append(Row(record.record_id,record.source.row_number,dict(record.values),observation,list(dict.fromkeys(events)),[] if excluded else actions_for(events),warnings,excluded))
        return self

    def document_exception(self,reason):
        """Compatibilidad con sesiones antiguas; ya no es requisito para exportar."""
        if not reason.strip():raise ValueError('Indica el motivo de la excepción de cruce.')
        self.exception=reason.strip()

    def export(self,destination,*,backend='native',reduced_fidelity=False):
        """Exporta la propuesta o las ediciones humanas vigentes de Trabajo.

        Si existe al menos una edición humana, las filas realmente revisadas quedan
        REVISADAS y escriben sus campos de revisión. Las restantes se conservan como
        PENDIENTES, sin atribuirles una revisión inexistente.
        """
        reviewed={row.id:_human_reviewed(row) for row in self.rows}
        reviewed_stage=any(reviewed.values())
        batch={'source_name':Path(self.path).name,'source_path':self.path,'source_hash':self.source_hash,
               'primary_sheet':self.sheet,'header_row':self.header,'mode':self.mode,
               'export_stage':'reviewed' if reviewed_stage else 'proposal'}
        records=[]
        for row in self.rows:
            is_reviewed=reviewed[row.id]
            records.append({
                'record_id':row.id,'source_sheet':self.sheet,'source_row':row.source_row,'source_hash':self.source_hash,
                'decision':'excluded' if row.excluded else ('approved' if is_reviewed else 'pending'),
                'evaluation_status':'blocked' if row.warnings else 'reviewed',
                'edit_reason':'; '.join(row.warnings),
                'edited_observation':_review_value(row,'OBSERVACION',row.observation),
                'rule_ids_json':json.dumps(row.rules),
                'rus_recorded':bool(is_reviewed and not row.excluded),
                'review_date':_review_value(row,'FECHA_OBS'),
                'tt_value':_review_value(row,'TT'),
                'workload_value':_review_value(row,'CC'),
                'resolution_value':_review_value(row,'RES'),
            })
        result=_export_preserved_payload(self.content,{'batch':batch,'records':records,'exceptions':[self.exception] if self.exception else []},destination,backend=backend,allow_reduced_fidelity=reduced_fidelity)
        self.output=str(result.path)
        self.output_hash=sha256(Path(self.output).read_bytes()).hexdigest()
        return self.output

    def refresh(self):
        """Actualiza la copia conocida, asociando por ID incluso después de ordenar."""
        if not self.output:raise ValueError('Primero procesa y exporta el trabajo.')
        path=Path(self.output)
        if not path.exists():raise ValueError('La copia fue movida. Usa Localizar copia para indicar su nueva ubicación.')
        digest=sha256(path.read_bytes()).hexdigest()
        if digest==self.output_hash:return False
        if getattr(self,'external_input',False):
            fresh=type(self).external(path,self.config,self.mode,sheet=self.sheet)
            if {r.id for r in fresh.rows}!={r.id for r in self.rows}:
                raise ValueError('Cambió el conjunto de personas de la planilla; usa Cargar planilla modificada para incorporar el nuevo conjunto.')
            self.rows=fresh.rows;self.header=fresh.header;self.mapping=fresh.mapping
            self.output_hash=digest
            return True
        import pandas as pd
        frame=pd.read_excel(path,sheet_name=self.sheet,header=self.header-1,dtype=object,keep_default_na=False)
        if 'NURUS_ID_REGISTRO' not in frame:raise ValueError('Falta la columna de identidad; no se pueden asociar las ediciones.')
        by_id={r.id:r for r in self.rows}
        incoming={}
        for values in frame.to_dict('records'):
            key=str(values.get('NURUS_ID_REGISTRO','')).strip()
            if not key:continue
            if key not in by_id or key in incoming:raise ValueError('Hay identidades ajenas o duplicadas en la copia; no se aplicaron cambios.')
            old=by_id[key]
            for field in ('rit','rut','nombre','tribunal','programa'):
                column=self.mapping.get(field)
                if column and historical_match(values.get(column,''))!=historical_match(old.values.get(column,'')):
                    raise ValueError('Cambió la identidad de una fila. No se aplicaron cambios; verifica '+field+'.')
            incoming[key]={k:values.get(k,'') for k in ('OBSERVACION','FECHA_OBS','TT','CC','RES')}
        if set(incoming)!=set(by_id):raise ValueError('Faltan registros en la copia; no se aplicaron cambios.')
        for key,review in incoming.items():
            by_id[key].review=review
            _sync_resolution_warning(by_id[key])
        self.output_hash=digest
        return True

    @classmethod
    def external(cls,path,config,mode='ESPERA',*,sheet=None):
        """Importa constancias tal como están, sin exigir un paso previo por el motor."""
        from .importing import read_external
        from .motor.utilidades import es_derivacion_sin_seg
        data=read_external(path,mode,sheet)
        obj=cls(config)
        obj.path=data['path'];obj.output=obj.path;obj.mode=data['mode'];obj.as_of=date.today().isoformat()
        obj.sheet=data['sheet'];obj.header=data['header'];obj.mapping=data['mapping']
        obj.content=data['content'];obj.source_hash=data['digest'];obj.output_hash=data['digest'];obj.needs_cross=False;obj.cross_missing=False
        obj.external_input=True;obj.warnings=[];seen={}
        for number,values,review,rules in data['records']:
            identity='|'.join(historical_match(values.get(obj.mapping.get(k,''),'')) for k in ('rit','rut','nombre','tribunal','programa'))
            ordinal=seen.get(identity,0);seen[identity]=ordinal+1
            rid=str(values.get('NURUS_ID_REGISTRO','')).strip() or sha256((obj.sheet+'|'+identity+'|'+str(ordinal)).encode()).hexdigest()
            if any(r.id==rid for r in obj.rows):raise ValueError('El identificador de registro está duplicado en la planilla.')
            excluded=es_derivacion_sin_seg(str(values.get(obj.mapping.get('programa',''),'')))
            events=list(dict.fromkeys(rules))
            actions=[] if excluded else actions_for(events)
            row=Row(rid,number,values,str(review.get('OBSERVACION','')),events,actions,[],excluded,review)
            _sync_resolution_warning(row)
            obj.rows.append(row)
        return obj

    def save(self,directory):
        directory=Path(directory);directory.mkdir(parents=True,exist_ok=True)
        self.storage_directory=str(directory)
        source=directory/(self.source_hash+'.bin')
        if not source.exists():source.write_bytes(self.content)
        data={k:v for k,v in self.__dict__.items() if k not in ('rows','content')}
        data['rows']=[asdict(r) for r in self.rows]
        atomic_json(directory/'trabajo.json',data)

    @classmethod
    def load(cls,directory):
        directory=Path(directory)
        data=json.loads((directory/'trabajo.json').read_text(encoding='utf-8'))
        obj=cls(data['config']);obj.__dict__.update(data)
        if getattr(obj,'needs_cross',False):obj.cross_missing=True
        obj.needs_cross=False
        if not hasattr(obj,'cross_missing'):obj.cross_missing=False
        obj.rows=[Row(**r) for r in data['rows']]
        for row in obj.rows:_sync_resolution_warning(row)
        obj.content=(directory/(obj.source_hash+'.bin')).read_bytes()
        if sha256(obj.content).hexdigest()!=obj.source_hash:raise ValueError('La copia de origen guardada no coincide con el trabajo.')
        return obj
