"""Productos de evidencia y solicitudes declarativas; sin consultas institucionales."""

import json


def request_document(work):
    manifest = getattr(work, 'capture_manifest', {})
    states = manifest.get('table_states', {})
    requests = []
    seen = set()
    for rid, record in getattr(work, 'capture_sources', {}).items():
        if record['identity_status'] != 'VERIFICADA':
            continue
        from nurus.capture.coverage import ADEQUATE, ABSENT

        coverage = getattr(work, 'capture_coverage', {}).get(rid, {})
        missing = [
            source
            for source in ('bitacoras', 'historia_tramites')
            if coverage.get(source, states.get(source) if states.get(source) in ABSENT else 'NO_CONSULTADA')
            not in ADEQUATE
        ]
        for source in missing:
            key = (record['source_system'], record['court_code'], record['cause_id'], record['ingreso_id'], source)
            if key in seen:
                continue
            seen.add(key)
            requests.append(
                {
                    'record_id': rid,
                    'source_system': record['source_system'],
                    'court_code': record['court_code'],
                    'cause_id': record['cause_id'],
                    'ingreso_id': record['ingreso_id'],
                    'source': source,
                    'reason': 'Fuente necesaria no disponible con cobertura suficiente.',
                }
            )
    return {
        'schema_id': 'CSMP_REQUEST',
        'schema_version': '1.0.0',
        'capture_sha256': getattr(work, 'capture_digest', ''),
        'as_of': work.as_of,
        'requests': requests,
    }


def export_requests(work, destination):
    from nurus.capture.requests import write_request

    return write_request(destination, request_document(work))


def evidence_text(work, row=None):
    if not getattr(work, 'capture_digest', ''):
        return 'SOLO_PLANILLA. Se conservan las reglas tradicionales. Sin bitácora/historia incorporada no se acredita ausencia de actuaciones ni última observación del centro.'
    manifest = work.capture_manifest
    lines = [
        'Captura: ' + work.capture_id,
        'Huella: ' + work.capture_digest,
        'Corte: ' + work.as_of,
        'Reglas: ' + work.ruleset_version,
        'Cobertura por fuente:',
    ]
    lines += [name + ': ' + state for name, state in manifest['table_states'].items()]
    lines.append(
        'Contexto temporal NO_EVALUABLE: aceptación institucional/funcional pendiente. Los productos requieren revisión humana.'
    )
    incidents = getattr(work, 'capture_incidents', [])
    if row:
        source = work.capture_sources[row.id]
        incidents = [
            i
            for i in incidents
            if i['record_id'] == source['record_id'] or (not i['record_id'] and i['query_id'] == source['query_id'])
        ]
        # Advertencias de otras consultas vinculadas por identidad también importan.
        query_ids = {e['query_id'] for e in work.capture_context[row.id].get('recent', [])}
        from nurus.capture.coverage import remote_identity

        query_ids.update(
            q['query_id']
            for q in manifest['queries']
            if remote_identity(json.loads(q['filters_json']).get('remote_identity', {})) == remote_identity(source)
        )
        incidents += [
            i
            for i in getattr(work, 'capture_incidents', [])
            if not i['record_id'] and i['query_id'] in query_ids and i not in incidents
        ]
        lines += ['', 'Cobertura del ingreso:'] + [
            k + ': ' + v for k, v in getattr(work, 'capture_coverage', {}).get(row.id, {}).items()
        ]
    if incidents:
        lines += ['', 'Incidencias:'] + [i['code'] + ' · ' + i['severity'] + ' · ' + i['message'] for i in incidents]
    if row:
        source = work.capture_sources[row.id]
        context = work.capture_context[row.id]
        lines += [
            '',
            'Procedencia: '
            + source['original_file']
            + ' / '
            + source['original_sheet']
            + ' / fila '
            + source['original_row'],
            'SHA256 fuente: ' + source['source_sha256'],
            'Identidad: ' + source['identity_status'],
            'Motivo: ' + context['reason'],
        ]
        lines += ['', 'Hallazgos:'] + [
            f["rule_id"] + ': ' + f['status'] + ' · ' + f['reason'] for f in context['findings']
        ]
        for event in context['recent']:
            lines += [
                '',
                event['event_at'] + ' · ' + event.get('event_type', '') + ' · ' + event['event_id'],
                event.get('text', event.get('description', '')),
            ]
            if event.get('response'):
                lines.append('Respuesta: ' + event['response'])
        lines += ['', 'Campos originales:', json.dumps(json.loads(source['raw_json']), ensure_ascii=False, indent=2)]
    return '\n'.join(lines)


def show_evidence(app):
    if not app.work:
        raise ValueError('Carga un trabajo primero.')
    import tkinter as tk
    from .selection import selected_ids

    chosen = selected_ids(app, 'records')
    row = next((r for r in app.work.rows if r.id in chosen), None)
    from .ui import Textbox

    window = tk.Toplevel(app)
    window.title('Fuentes, cobertura y antecedentes')
    window.geometry('850x620')
    window.transient(app)
    text = Textbox(window, wrap='word')
    text.pack(fill='both', expand=True, padx=10, pady=10)
    text.insert('1.0', evidence_text(app.work, row))
    text.configure(state='disabled')


def export_evidence(work, destination):
    from .reports import _book

    rows = []
    for row in work.rows:
        source = getattr(work, 'capture_sources', {}).get(row.id, {})
        context = getattr(work, 'capture_context', {}).get(row.id, {})
        rows.append(
            [
                row.id,
                source.get('identity_status', 'SOLO_PLANILLA'),
                context.get('class', 'NO_EVALUABLE'),
                context.get('reason', 'Contexto no disponible.'),
                source.get('original_file', work.path),
                source.get('original_row', row.source_row),
            ]
        )
    from nurus.capture.contract import TABLES

    columns = TABLES['incidencias']
    return _book(
        [
            (
                'Cobertura',
                ('Fuente', 'Estado'),
                list(getattr(work, 'capture_manifest', {}).get('table_states', {'Excel': 'SOLO_PLANILLA'}).items()),
            ),
            ('Registros', ('ID local', 'Identidad', 'Revisión', 'Motivo', 'Archivo', 'Fila'), rows),
            ('Incidencias', columns, [[i[k] for k in columns] for i in getattr(work, 'capture_incidents', [])]),
        ],
        destination,
    )


def institutional_statement_issue(work, draft):
    """No reescribe plantillas personales ni presenta propuestas como inscripción."""
    if not getattr(work, 'capture_digest', ''):
        return ''
    from nurus.rus.columns import normalize

    text = normalize(draft.body)
    phrases = (
        'se registraron observaciones',
        'se ingresaron observaciones en rus',
        'observaciones registradas en rus',
        'se enviaron los correos',
    )
    if any(phrase in text for phrase in phrases):
        return 'El paquete no acredita registro RUS ni envío. Revisa esa afirmación del texto antes de guardar el borrador.'
    return ''
