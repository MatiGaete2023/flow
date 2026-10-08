"""Importación del lote de lectura: control de archivos, ingresos y omisiones."""
from pathlib import Path
from hashlib import sha256
import json
from .bitacora_har import read_popup, export_har_audit
from nurus.bitacora_html import visible_identity_warnings

def import_lote(manifest):
    manifest=Path(manifest).resolve()
    if manifest.stat().st_size>32*1024*1024:raise ValueError('Lote de bitácoras demasiado grande.')
    raw=manifest.read_bytes();data=json.loads(raw.decode('utf-8'))
    if data.get('tipo')!='BITACORAS_LECTURA' or data.get('version')!=1:
        raise ValueError('El archivo no es un lote de lectura de bitácoras.')
    records=data.get('registros');scopes=data.get('consultas')
    if not isinstance(records,list) or not isinstance(scopes,list):raise ValueError('Falta el control de lecturas del lote.')
    queries={};ledger=[]
    for record in records:
        identity=tuple(record.get(k,'') for k in ('tribunal_codigo','causa_id','ingreso_id'))
        state=record.get('estado','FALLIDA');error=record.get('error','');query=None
        if state=='LEIDA':
            try:
                name=record['archivo'];path=(manifest.parent/name).resolve()
                if path.parent!=manifest.parent or path.suffix!='.html' or path.stat().st_size>8*1024*1024:
                    raise ValueError('Archivo de bitácora fuera de alcance.')
                body=path.read_bytes()
                if sha256(body).hexdigest()!=record['sha256']:raise ValueError('La copia HTML cambió después de su verificación.')
                query=read_popup(body.decode('utf-8-sig'),record['params'],source=name,captured_at=record['capturada'])
                if tuple(query[k] for k in ('tribunal_codigo','causa_id','ingreso_id'))!=identity:
                    raise ValueError('La bitácora pertenece a otro ingreso.')
                notes=visible_identity_warnings(query,record['rit'],record['nombre'])
                if notes:
                    query['warnings'].extend(notes)
                    error=' '.join(notes)
            except (ValueError,KeyError,OSError,TypeError) as exc:
                state='FALLIDA';error=str(exc) if isinstance(exc,ValueError) else 'No se pudo abrir una copia verificable.';query=None
        if all(identity):
            if query is None:
                query={'tribunal_codigo':identity[0],'causa_id':identity[1],'ingreso_id':identity[2],
                    'rit':record.get('rit',''),'persona':record.get('nombre',''),'centro':record.get('programa',''),
                    'coverage':'FALLIDA','error':error or 'No se completó la lectura.', 'entries':[],
                    'stage_code':'','captured_at':'','captures':0,'html_sha256':'','warnings':[],'row_metadata':[]}
            previous=queries.get(identity)
            if previous and previous['coverage']!='FALLIDA' and query['coverage']!='FALLIDA':
                if previous['html_sha256']!=query['html_sha256']:
                    raise ValueError('Hay copias distintas del mismo ingreso. Importa lotes separados para comparar.')
                previous['captures']+=1
            elif previous is None or previous['coverage']=='FALLIDA':queries[identity]=query
        ledger.append([record.get(k,'') for k in ('tribunal_nombre','modalidad_nombre','pestana','pagina','fila','rit','nombre','programa','causa_id','ingreso_id')]+[state,error,record.get('archivo',''),record.get('sha256',''),record.get('capturada',''),record.get('diagnostico',{}).get('codigo_extension','')])
    scope_rows=[[s.get(k,'') for k in ('tribunal_nombre','modalidad_nombre','tab','estado','paginas','registros','error')] for s in scopes]
    expected=data.get('seleccion',{})
    planned={(t,mod,tab) for t in expected.get('tribunals',[]) for mod in expected.get('modalities',[]) for tab in expected.get('tabs',[])}
    attempted={(s.get('tribunal'),s.get('modality'),s.get('tab')) for s in scopes}
    for tribunal,modality,tab in sorted(planned-attempted):scope_rows.append([tribunal,modality,tab,'NO_CONSULTADA',0,0,'La ejecución terminó antes de esta consulta.'])
    extra=[('Lecturas',('Tribunal','Modalidad','Pestaña','Página','Fila','RIT','Persona','Centro','Causa RUS','Ingreso RUS','Estado lectura','Detalle','Archivo','SHA-256','Fecha captura','Código extensión'),ledger),
           ('Consultas',('Tribunal','Modalidad','Pestaña','Estado','Páginas recorridas','Registros enumerados','Detalle'),scope_rows),
           ('Lote',('Dato','Valor'),[['Estado del recorrido',data.get('estado','INCOMPLETA')],['Fecha',data.get('fecha','')],
                ['Error del recorrido',data.get('error','')],
                ['Descargador',json.dumps(data.get('origen',{}),ensure_ascii=False)],
                ['Extensión',json.dumps(data.get('extension',{}),ensure_ascii=False)],
                ['Selección',json.dumps(expected,ensure_ascii=False)],['Historial','Copia de la tabla capturada; no acredita historial remoto completo.']])]
    if data.get('anteriores_no_reenumeradas'):
        rows=[[r.get(k,'') for k in ('tribunal_nombre','modalidad_nombre','pestana','rit','nombre','causa_id','ingreso_id','estado','archivo','sha256')]
              +['No se reenumeró en esta ejecución; no se acredita desaparición del registro.'] for r in data['anteriores_no_reenumeradas']]
        extra.append(('Lecturas anteriores',('Tribunal','Modalidad','Pestaña','RIT','Persona','Causa RUS','Ingreso RUS','Estado previo','Archivo previo','SHA-256 previo','Alcance'),rows))
    return {'queries':list(queries.values()),'sources':[{'file':manifest.name,'sha256':sha256(raw).hexdigest(),
        'requests':len(records),'openings':sum(r[10]=='LEIDA' for r in ledger),'detail':data.get('estado','')}],'extra_sheets':extra}

def export_lote(manifest,destination,start,end,*,today=None):
    capture=import_lote(manifest)
    return export_har_audit(capture,destination,start,end,today=today,extra_sheets=capture['extra_sheets'])
