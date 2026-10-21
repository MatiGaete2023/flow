"""Paquete local desde originales verificados, sin llamadas de Chrome ni CSMP."""

from datetime import datetime, date
from hashlib import sha256
import json
from pathlib import Path
from csmp_shared import parser

parser()
from nurus.capture.contract import COMMON, TABLES, write_capture
from nurus.capture.diaries import import_lotes, diary_paths
from resultados import tables, table_mode, inside, headers_and_rows, parse_day
import motor as m
from nurus.capture.coverage import aggregate


def calendar_date(normalized):
    """Fecha de vencimiento ISO de una fila de calendario de informes; vacía si no se reconoce."""
    from nurus.rus.columns import map_columns

    column = map_columns(list(normalized), 'INFORMES').get('vencimiento')
    day = parse_day(normalized.get(column, '')) if column else None
    return day.isoformat() if day else ''


def literal(value):
    if value is None:
        return ''
    if isinstance(value, (date, datetime)):
        return value.isoformat()
    return str(value)


def consolidated_workbook(lots):
    """Libro CSMP de los lotes como bytes (D6): solo el XLSX, nunca HTML ni PDF. Falla si el lote no es apto."""
    from tempfile import TemporaryDirectory
    from resultados import export_book

    with TemporaryDirectory(prefix='consolidado-') as folder:
        return export_book(lots, Path(folder), csmp=True).read_bytes()


def export_capture_with_note(lots, destination, *, consolidated=False, **options):
    """Entrega de GUI: genera el consolidado una vez o informa por qué el lote no es apto para CSMP."""
    lots = list(lots)
    data, note = None, ''
    if consolidated:
        try:
            data = consolidated_workbook(lots)
        except m.PocError as exc:
            note = ' Sin XLSX consolidado: ' + str(exc)
        else:
            note = ' Incluye el XLSX consolidado.'
    path = export_capture(lots, destination, _consolidated_data=data, **options)
    return path, note


def export_capture(
    lots,
    destination,
    *,
    diaries=None,
    include_originals=False,
    match_diaries=False,
    consolidated=False,
    _consolidated_data=None,
):
    lots = list(lots)
    if not lots:
        raise ValueError('Selecciona un lote verificado.')
    consolidated_bytes = consolidated_workbook(lots) if consolidated else _consolidated_data
    captured = lots[0].summary.get('fecha_hora') or datetime.now().astimezone().isoformat()
    fingerprint = json.dumps([lot.summary for lot in lots], ensure_ascii=False, sort_keys=True)
    # Hash del control de bitácoras integra la identidad de la captura reexportable.
    if diaries:
        fingerprint += json.dumps(sorted(sha256(p.read_bytes()).hexdigest() for p in diary_paths(diaries)))
        fingerprint += 'MATCH' if match_diaries else 'ALL'
    cid = 'captura-' + sha256(fingerprint.encode()).hexdigest()[:24]
    data = {'consultas': [], 'registros': [], 'incidencias': []}
    originals = {}
    query_index = {}

    def incident(qid, code, message, rid=''):
        data['incidencias'].append(
            dict(
                capture_id=cid,
                query_id=qid,
                record_id=rid,
                incident_id='i' + str(len(data['incidencias']) + 1),
                phase='EXPORTAR',
                code=code,
                severity='ADVERTENCIA',
                message=message,
                recoverable='true',
            )
        )

    def query(qid, selection, state, when, expected=0, saved=0, screen=None, error=''):
        row = {k: '' for k in TABLES['consultas']}
        row.update(
            query_id=qid,
            source_system='RUS',
            screen=screen or selection.get('screen', 'seguimiento'),
            tab=selection.get('tab', ''),
            court_code=literal(selection.get('tribunal', '')),
            court_name=selection.get('tribunal_nombre', ''),
            modality_code=literal(selection.get('modality', '')),
            coverage=state,
            captured_at=when,
            filters_json=json.dumps(selection, ensure_ascii=False, default=literal),
            expected_pages=str(expected),
            saved_pages=str(saved),
            time_zone='DESCONOCIDA',
            history_complete='false',
            institutional_verified='false',
            error=error,
        )
        data['consultas'].append(row)
        return row

    for li, lot in enumerate(lots):
        for qi, (item, manifest) in enumerate(lot.queries):
            qid = f'q{li}-{qi}'
            selection = item.get('seleccion') or manifest.get('seleccion') or {}
            state = {'VALIDADA': 'COMPLETA', 'SIN_RESULTADOS': 'VACIA_COMPROBADA', 'INCOMPLETA': 'PARCIAL'}.get(
                manifest['estado'], 'FALLIDA'
            )
            files = [f for f in manifest.get('archivos', []) if f.get('estado') == 'OK']
            row = query(
                qid,
                selection,
                state,
                lot.summary.get('fecha_hora') or captured,
                manifest.get('paginas_esperadas', len(files)),
                len(files),
            )
            query_index[(li, item['verificacion'])] = row
            if state == 'PARCIAL':
                incident(qid, 'CAPTURA_PARCIAL', 'La consulta no tiene cobertura completa.')
        attempted = [item.get('seleccion', {}) for item, _ in lot.queries]
        for pi, selection in enumerate(lot.summary.get('plan', [])):
            if selection not in attempted:
                qid = f'q{li}-pendiente-{pi}'
                query(qid, selection, 'NO_CONSULTADA', captured)
                incident(qid, 'NO_CONSULTADA', 'Consulta planificada no completada.')
        for item, selection, source, headers, rows in tables(lot):
            q = query_index[(li, item['verificacion'])]
            # La fuente primaria ya validada conserva encabezados/valores propios.
            raw_headers, raw_rows = (
                headers_and_rows(m.read_excel(inside(lot.folder, source['archivo']).read_bytes()))
                if selection.get('screen') != 'carga'
                else (headers, rows)
            )
            raw_index = dict(raw_rows)
            if include_originals:
                name = f'originales/l{li}/' + source['archivo']
                originals[name] = inside(lot.folder, source['archivo']).read_bytes()
            else:
                name = source['archivo']
            mode = table_mode(selection) or ('EGRESADOS' if selection.get('tab') in ('Egresados', 'Egreso') else 'OTRO')
            for number, values in rows:
                normalized = {h: literal(v) for h, v in zip(headers, values)}
                raw = {h: literal(v) for h, v in zip(raw_headers, raw_index[number])}
                common = {k: '' for k in COMMON}
                common.update(
                    capture_id=cid,
                    query_id=q['query_id'],
                    source_system='RUS',
                    court_code=q['court_code'],
                    court_name=q['court_name'],
                    rit_original=normalized.get('RIT', ''),
                    cause_id=normalized.get('SITFA_CAUSA_RUS', ''),
                    ingreso_id=normalized.get('SITFA_INGRESO_RUS', ''),
                    person_id=normalized.get('SITFA_PERSONA_RUS', ''),
                    program_id=normalized.get('SITFA_CENTRO_RUS', ''),
                    modality_code=q['modality_code'],
                    modality_raw=normalized.get('MODALIDAD', ''),
                    tab=q['tab'],
                    captured_at=q['captured_at'],
                    original_file=name,
                    original_sheet='',
                    original_row=str(number),
                    source_sha256=source['sha256'],
                    coverage=q['coverage'],
                )
                rid = sha256(f"{q['query_id']}|{source['sha256']}|{number}".encode()).hexdigest()[:24]
                verified = bool(common['court_code'] and common['cause_id'] and common['ingreso_id'])
                data['registros'].append(
                    {
                        **common,
                        'record_id': rid,
                        'record_type': mode,
                        'identity_status': 'VERIFICADA' if verified else 'INCIERTA',
                        'raw_json': json.dumps(raw, ensure_ascii=False),
                        'normalized_json': json.dumps(normalized, ensure_ascii=False),
                    }
                )
                if not verified:
                    incident(
                        q['query_id'],
                        'IDENTIDAD_INCIERTA',
                        'No hay ingreso remoto verificable; no se relacionan eventos por RIT/nombre.',
                        rid,
                    )
                if selection.get('screen') == 'calendario_informes':
                    data.setdefault('calendarios', []).append(
                        {
                            **common,
                            'calendar_id': 'cal-' + rid,
                            'event_at': calendar_date(normalized),
                            'event_type': 'INFORME',
                            'status': 'SEGUN_FUENTE',
                            'association': 'VERIFICADA' if verified else 'NO_ASOCIADO',
                            'raw_json': json.dumps(raw, ensure_ascii=False),
                            'normalized_json': json.dumps(normalized, ensure_ascii=False),
                        }
                    )
    if diaries:
        data['bitacoras'] = []
        identities = (
            {
                tuple(r[k] for k in ('court_code', 'cause_id', 'ingreso_id'))
                for r in data['registros']
                if r['identity_status'] == 'VERIFICADA'
            }
            if match_diaries
            else None
        )
        batch = import_lotes(diaries, identities=identities)
        if match_diaries:
            incident(
                data['consultas'][0]['query_id'],
                'BITACORAS_REUTILIZADAS',
                f"Bitácoras conservadas por tribunal/causa/ingreso: {batch['reuse']['matched']}; ajenas excluidas: {batch['reuse']['excluded']}. Se conservan fecha y cobertura originales; no se acredita actualización remota.",
            )
        if match_diaries and not batch['queries']:
            query(
                'diary-sin-coincidencia',
                {'capture_tables': ['bitacoras']},
                'IDENTIDAD_INCIERTA',
                captured,
                screen='bitacoras',
                error='Las bitácoras disponibles no tienen correspondencia remota verificada con los registros seleccionados.',
            )
        for qi, snapshot in enumerate(batch['queries']):
            qid = f'diary-{qi}'
            selection = {
                'tribunal': snapshot['tribunal_codigo'],
                'tab': 'BITACORAS',
                'capture_tables': ['bitacoras'],
                'remote_identity': {
                    'source_system': 'RUS',
                    'court_code': snapshot['tribunal_codigo'],
                    'cause_id': snapshot['causa_id'],
                    'ingreso_id': snapshot['ingreso_id'],
                },
            }
            when = snapshot.get('captured_at') or captured
            q = query(qid, selection, snapshot['coverage'], when, 1, 1, screen='bitacoras')
            if snapshot['coverage'] == 'FALLIDA':
                incident(qid, 'BITACORA_FALLIDA', snapshot['error'])
            for warning in snapshot.get('warnings', []):
                incident(qid, 'BITACORA_ADVERTENCIA', warning)
            for en, entry in enumerate(snapshot['entries'], 1):
                common = {k: '' for k in COMMON}
                common.update(
                    capture_id=cid,
                    query_id=qid,
                    source_system='RUS',
                    court_code=snapshot['tribunal_codigo'],
                    rit_original=entry['rit'],
                    cause_id=entry['causa_id'],
                    ingreso_id=entry['ingreso_id'],
                    captured_at=when,
                    original_file=entry['fuente'],
                    original_sheet='TablaInforme',
                    original_row=str(entry['fila_fuente']),
                    source_sha256=snapshot['html_sha256'],
                    coverage=snapshot['coverage'],
                )
                event_at = entry['fecha']
                date_only = event_at.endswith('T00:00:00')
                data['bitacoras'].append(
                    {
                        **common,
                        'event_id': f"{qid}-{entry['entry_id']}",
                        'event_at': event_at[:10] if date_only else event_at,
                        'time_precision': 'DIA' if date_only else 'SEGUNDO',
                        'time_zone': 'DESCONOCIDA',
                        'origin': 'CENTRO',
                        'origin_verified': 'true',
                        'author': entry['autor'],
                        'event_type': entry['tipo'],
                        'text': entry['texto'],
                        'response': entry['respuesta'],
                        'response_at': entry['fecha_respuesta'],
                        'identity_status': 'VERIFICADA',
                    }
                )
        for ci, calendar in enumerate(batch.get('calendars', [])):
            qid = f'calendar-{ci}'
            scope = {
                'source_system': 'RUS',
                'court_code': calendar['tribunal_codigo'],
                'cause_id': calendar['causa_id'],
                'ingreso_id': calendar['ingreso_id'],
            }
            selection = {
                'tribunal': calendar['tribunal_codigo'],
                'tab': 'CALENDARIO',
                'capture_tables': ['calendarios'],
                'remote_identity': scope,
            }
            when = calendar.get('captured_at') or captured
            query(
                qid,
                selection,
                calendar['coverage'],
                when,
                1,
                0 if calendar['coverage'] == 'FALLIDA' else 1,
                screen='calendario_ingreso',
                error=calendar['error'],
            )
            if calendar['coverage'] == 'FALLIDA':
                incident(qid, 'CALENDARIO_FALLIDO', calendar['error'])
                continue
            plazos = {
                'plazo_remision_meses': calendar.get('plazo_remision_meses'),
                'plazo_intervencion_meses': calendar.get('plazo_intervencion_meses'),
            }
            data.setdefault('calendarios', [])
            for projection in calendar['proyecciones']:
                common = {k: '' for k in COMMON}
                common.update(
                    capture_id=cid,
                    query_id=qid,
                    source_system='RUS',
                    court_code=calendar['tribunal_codigo'],
                    cause_id=calendar['causa_id'],
                    ingreso_id=calendar['ingreso_id'],
                    captured_at=when,
                    original_file=calendar['archivo'],
                    original_sheet='Calendario',
                    original_row=str(projection['fila']),
                    source_sha256=calendar['html_sha256'],
                    coverage='COMPLETA',
                )
                values = json.dumps({**projection, **plazos}, ensure_ascii=False)
                data['calendarios'].append(
                    {
                        **common,
                        'calendar_id': f"{qid}-{projection['fila']}",
                        'event_at': projection['fecha_proyeccion'],
                        'event_type': 'INFORME_PROYECTADO',
                        'status': projection['estado'],
                        'association': 'VERIFICADA',
                        'raw_json': values,
                        'normalized_json': values,
                    }
                )
    states = {k: 'NO_SOLICITADA' for k in TABLES}
    states.update(
        consultas=aggregate(q['coverage'] for q in data['consultas']),
        registros=aggregate(
            q['coverage'] for q in data['consultas'] if q['screen'] not in ('bitacoras', 'calendario_ingreso')
        ),
        incidencias='COMPLETA' if data['incidencias'] else 'VACIA_COMPROBADA',
    )
    for name in ('calendarios', 'bitacoras'):
        screens = ('calendario_informes', 'calendario_ingreso') if name == 'calendarios' else ('bitacoras',)
        relevant = [q['coverage'] for q in data['consultas'] if q['screen'] in screens]
        if name in data or relevant:
            state = aggregate(relevant)
            states[name] = 'VACIA_COMPROBADA' if not data.get(name) and state == 'COMPLETA' else state
            data.setdefault(name, [])
    from contrato_integral import capabilities

    build = capabilities()
    metadata = {
        'capture_id': cid,
        'captured_at': captured,
        'downloader': {'version': build['version'], 'commit': build['commit']},
        'contains_sensitive_data': True,
        'originals_authorized': include_originals,
        'queries': data['consultas'],
        'table_states': states,
    }
    return write_capture(destination, metadata, data, originals=originals, consolidated=consolidated_bytes)
