"""Lectura local de capturas RUS. No reproduce peticiones ni guarda en el sitio."""

from .bitacoras import Observation, export_audit, cc_for_type, response_state, EXCEL_CELL_TEXT_LIMIT, _utf16_units

from nurus.bitacora_html import HEADERS as HEADERS, read_snapshot


def read_popup(text, params, *, source="", captured_at=""):
    query = read_snapshot(text, params, source=source, captured_at=captured_at)
    query["entries"] = [Observation(**entry) for entry in query["entries"]]
    return query


def export_har_audit(capture, destination, start, end, *, today=None, extra_sheets=()):
    sources = [(s['file'], s['sha256'], s['requests'], s['openings'], s['detail']) for s in capture['sources']]
    provenance = []
    copy_rows = []
    for query in capture['queries']:
        provenance.append(
            [
                query['tribunal_codigo'],
                query['causa_id'],
                query['ingreso_id'],
                query['rit'],
                query['persona'],
                query['centro'],
                query['stage_code'],
                query['captured_at'],
                query['captures'],
                len(query['entries']),
                query['html_sha256'],
                query['error'],
                '\n'.join(query['warnings']),
            ]
        )
        for entry, meta in zip(query['entries'], query['row_metadata']):
            if any(_utf16_units(text) > EXCEL_CELL_TEXT_LIMIT for text in (entry.texto, entry.respuesta)):
                raise ValueError('Un texto excede el límite de celda de Excel; no se generará una copia recortada.')
            copy_rows.append(
                [
                    entry.tribunal_codigo,
                    entry.causa_id,
                    entry.ingreso_id,
                    entry.rit,
                    entry.persona,
                    entry.centro,
                    entry.entry_id,
                    entry.fecha,
                    entry.autor,
                    entry.tipo,
                    cc_for_type(entry.tipo),
                    entry.etapa,
                    entry.texto,
                    entry.fecha_respuesta,
                    meta['tribunal_user'],
                    entry.respuesta,
                    response_state(entry),
                    meta['state'],
                    meta['send_to_tribunal'],
                    query['captured_at'],
                ]
            )
    return export_audit(
        capture['queries'],
        destination,
        start,
        end,
        today=today,
        extra_sheets=[
            (
                'Copia íntegra',
                (
                    'Tribunal código',
                    'Causa RUS',
                    'Ingreso RUS',
                    'RIT',
                    'Persona',
                    'Centro',
                    'Entrada RUS',
                    'Fecha centro',
                    'Usuario centro',
                    'Tipo',
                    'CC',
                    'Etapa',
                    'Texto centro',
                    'Fecha tribunal',
                    'Usuario tribunal',
                    'Texto tribunal',
                    'Estado respuesta',
                    'Estado RUS código',
                    'HFLG_Envio RUS',
                    'Fecha captura',
                ),
                copy_rows,
            ),
            (
                'Capturas',
                (
                    'Tribunal código',
                    'Causa RUS',
                    'Ingreso RUS',
                    'RIT',
                    'Persona',
                    'Centro',
                    'Etapa código',
                    'Fecha captura',
                    'Aperturas idénticas',
                    'Entradas tabla capturada',
                    'SHA-256 HTML',
                    'Alcance',
                    'Advertencias',
                ),
                provenance,
            ),
            ('Fuentes', ('Archivo', 'SHA-256', 'Peticiones', 'Aperturas', 'Detalle'), sources),
            *extra_sheets,
        ],
    )
