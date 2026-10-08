"""Lectura local de capturas RUS. No reproduce peticiones ni guarda en el sitio."""
import base64
from datetime import datetime
from hashlib import sha256
import json
from pathlib import Path
from urllib.parse import parse_qsl, urlsplit

from .bitacoras import Observation, export_audit, cc_for_type, response_state, EXCEL_CELL_TEXT_LIMIT, _utf16_units

from nurus.bitacora_html import ORIGIN, POPUP_PATH, HEADERS, read_snapshot

def read_popup(text, params, *, source="", captured_at=""):
    query=read_snapshot(text,params,source=source,captured_at=captured_at)
    query["entries"]=[Observation(**entry) for entry in query["entries"]]
    return query


def import_har(paths):
    """Importa varias capturas. No accede a cookies, cabeceras de sesión ni red."""
    queries = {}; sources = []
    for path in map(Path, paths):
        raw = path.read_bytes(); document = json.loads(raw.decode('utf-8-sig'))
        records = document.get('log', {}).get('entries', [])
        if not isinstance(records, list): raise ValueError('HAR sin lista de peticiones válida.')
        found = 0
        for number, record in enumerate(records):
            request = record.get('request', {}); url = urlsplit(request.get('url', ''))
            if url.path != POPUP_PATH: continue
            if f'{url.scheme}://{url.netloc}' != ORIGIN or request.get('method') != 'GET':
                raise ValueError('Origen o método inesperado para la apertura de bitácora.')
            pairs = parse_qsl(url.query, keep_blank_values=True)
            params = dict(pairs)
            if len(params) != len(pairs): raise ValueError('La consulta tiene parámetros repetidos.')
            if params.get('tipo_popUp') != '12': continue
            response = record.get('response', {})
            if response.get('status') != 200: raise ValueError('Una apertura de bitácora no respondió correctamente.')
            content = response.get('content', {}); text = content.get('text', '')
            encoding = content.get('encoding')
            if encoding == 'base64': text = base64.b64decode(text, validate=True).decode('utf-8-sig')
            elif encoding: raise ValueError('Codificación HAR desconocida.')
            capture_time = record.get('startedDateTime', '')
            try: datetime.fromisoformat(capture_time.replace('Z', '+00:00'))
            except (ValueError, AttributeError): raise ValueError('La apertura no identifica la fecha de la captura.')
            query = read_popup(text, params, source=f'{path.name} / respuesta {number}', captured_at=capture_time)
            found += 1
            key = tuple(query[k] for k in ('tribunal_codigo', 'causa_id', 'ingreso_id'))
            if key in queries:
                old = queries[key]
                if old['html_sha256'] != query['html_sha256']:
                    raise ValueError('Dos capturas del mismo ingreso difieren. Importa por separado para comparar las versiones.')
                old['captures'] += 1
            else: queries[key] = query
        sources.append({'file': path.name, 'sha256': sha256(raw).hexdigest(), 'requests': len(records),
                        'openings': found, 'detail': '' if found else 'Sin respuestas de bitácora recuperables.'})
    if not queries: raise ValueError('No hay respuestas de bitácora recuperables en los HAR seleccionados.')
    return {'queries': list(queries.values()), 'sources': sources}


def export_har_audit(capture, destination, start, end, *, today=None, extra_sheets=()):
    sources = [(s['file'], s['sha256'], s['requests'], s['openings'], s['detail']) for s in capture['sources']]
    provenance = []; copy_rows = []
    for query in capture['queries']:
        provenance.append([query['tribunal_codigo'], query['causa_id'], query['ingreso_id'], query['rit'],
            query['persona'], query['centro'], query['stage_code'], query['captured_at'], query['captures'],
            len(query['entries']), query['html_sha256'], query['error'], '\n'.join(query['warnings'])])
        for entry, meta in zip(query['entries'], query['row_metadata']):
            if any(_utf16_units(text)>EXCEL_CELL_TEXT_LIMIT for text in (entry.texto,entry.respuesta)):
                raise ValueError('Un texto excede el límite de celda de Excel; no se generará una copia recortada.')
            copy_rows.append([entry.tribunal_codigo,entry.causa_id,entry.ingreso_id,entry.rit,entry.persona,
                entry.centro,entry.entry_id,entry.fecha,entry.autor,entry.tipo,cc_for_type(entry.tipo),entry.etapa,
                entry.texto,entry.fecha_respuesta,meta['tribunal_user'],entry.respuesta,response_state(entry),
                meta['state'],meta['send_to_tribunal'],query['captured_at']])
    return export_audit(capture['queries'], destination, start, end, today=today, extra_sheets=[
        ('Copia íntegra', ('Tribunal código','Causa RUS','Ingreso RUS','RIT','Persona','Centro','Entrada RUS',
         'Fecha centro','Usuario centro','Tipo','CC','Etapa','Texto centro','Fecha tribunal','Usuario tribunal',
         'Texto tribunal','Estado respuesta','Estado RUS código','HFLG_Envio RUS','Fecha captura'),copy_rows),
        ('Capturas', ('Tribunal código', 'Causa RUS', 'Ingreso RUS', 'RIT', 'Persona', 'Centro',
         'Etapa código', 'Fecha captura', 'Aperturas idénticas', 'Entradas tabla capturada', 'SHA-256 HTML',
         'Alcance', 'Advertencias'), provenance),
        ('Fuentes', ('Archivo', 'SHA-256', 'Peticiones', 'Aperturas', 'Detalle'), sources), *extra_sheets])
