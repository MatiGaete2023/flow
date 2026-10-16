"""Paquete local desde originales verificados, sin llamadas de Chrome ni CSMP."""

from datetime import datetime
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
from nurus.capture.historias import cargar as cargar_historias, literal


def calendar_date(normalized):
    """Fecha de vencimiento ISO de una fila de calendario de informes; vacía si no se reconoce."""
    from nurus.rus.columns import map_columns

    column = map_columns(list(normalized), 'INFORMES').get('vencimiento')
    day = parse_day(normalized.get(column, '')) if column else None
    return day.isoformat() if day else ''


def asociar_calendarios(data):
    """Calendarios de informes sin ids remotos: se asocian a un ingreso verificado solo si tribunal, RIT, nombre y
    programa coinciden con exactamente un ingreso de las demás tablas de la captura (nunca por RIT solo ni con empate).
    Devuelve los record_id resueltos."""
    from collections import defaultdict
    from nurus.rus.columns import map_columns

    def clave(court, normalized, modo):
        try:
            columnas = map_columns(list(normalized), modo)
        except Exception:
            return None
        partes = [m.normalized(normalized.get(columnas.get(k, ''), '')) for k in ('rit', 'nombre', 'programa')]
        return (court, *partes) if all(partes) else None

    ingresos = defaultdict(set)
    for registro in data['registros']:
        if registro['identity_status'] != 'VERIFICADA' or registro['record_type'] in ('INFORMES', 'OTRO', 'EGRESADOS'):
            continue
        k = clave(registro['court_code'], json.loads(registro['normalized_json']), registro['record_type'])
        if k:
            ingresos[k].add(tuple(registro[f] for f in ('cause_id', 'ingreso_id', 'person_id', 'program_id')))
    por_fisica = {
        (r['query_id'], r['source_sha256'], r['original_row']): r
        for r in data['registros']
        if r['record_type'] == 'INFORMES'
    }
    resueltos = []
    for calendario in data.get('calendarios', []):
        if calendario['association'] == 'VERIFICADA':
            continue
        k = clave(calendario['court_code'], json.loads(calendario['normalized_json']), 'INFORMES')
        candidatos = ingresos.get(k, ())
        if len(candidatos) != 1:
            continue
        ids = next(iter(candidatos))
        registro = por_fisica.get((calendario['query_id'], calendario['source_sha256'], calendario['original_row']))
        for destino in (calendario, registro):
            if destino is None:
                continue
            destino.update(zip(('cause_id', 'ingreso_id', 'person_id', 'program_id'), ids))
        calendario['association'] = 'VERIFICADA'
        if registro is not None:
            registro['identity_status'] = 'VERIFICADA'
            resueltos.append(registro['record_id'])
    return resueltos


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
    historia=None,
    meses=4,
    avisos=(),
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
    if historia:
        fingerprint += 'HISTORIA' + sha256(Path(historia).read_bytes()).hexdigest()
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
    for codigo, mensaje in avisos:
        # Avisos del flujo (p. ej. calendario pedido y no completo): viajan en el paquete, no solo en el Descargador.
        incident(data['consultas'][0]['query_id'] if data['consultas'] else '', codigo, mensaje)
    resueltos = set(asociar_calendarios(data))
    if resueltos:
        data['incidencias'] = [
            i for i in data['incidencias'] if not (i['code'] == 'IDENTIDAD_INCIERTA' and i['record_id'] in resueltos)
        ]
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
        from datetime import date as _date
        from nurus.capture.temporal import earliest

        # Período a recolectar (1 a 4 meses): las entradas anteriores a la ventana no pasan al paquete.
        try:
            corte = _date.fromisoformat(captured[:10])
        except ValueError:
            corte = _date.today()
        primera_fecha = earliest(corte, meses).isoformat()
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
                # Con 4 meses (valor por defecto) el paquete conserva la bitácora completa, como antes: el motor
                # busca ahí la primera observación de una materia para reiterarla. Solo un período menor recorta.
                if meses < 4 and entry['fecha'][:10] < primera_fecha:
                    continue
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
    if historia:
        _historia(historia, data, query, incident, cid, captured)
    states = {k: 'NO_SOLICITADA' for k in TABLES}
    states.update(
        consultas=aggregate(q['coverage'] for q in data['consultas']),
        registros=aggregate(
            q['coverage'] for q in data['consultas'] if q['screen'] not in ('bitacoras', 'historia_tramites')
        ),
        incidencias='COMPLETA' if data['incidencias'] else 'VACIA_COMPROBADA',
    )
    for name in ('calendarios', 'bitacoras'):
        screen = 'calendario_informes' if name == 'calendarios' else 'bitacoras'
        relevant = [q['coverage'] for q in data['consultas'] if q['screen'] == screen]
        if name in data or relevant:
            state = aggregate(relevant)
            states[name] = 'VACIA_COMPROBADA' if not data.get(name) and state == 'COMPLETA' else state
            data.setdefault(name, [])
    if historia:
        states['historia_tramites'] = aggregate(
            q['coverage'] for q in data['consultas'] if q['screen'] == 'historia_tramites'
        )
        if not data.get('historia_tramites') and states['historia_tramites'] == 'COMPLETA':
            states['historia_tramites'] = 'VACIA_COMPROBADA'
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


HISTORIA_COBERTURA = {
    'LEIDA': 'COMPLETA',
    'VACIA_COMPROBADA': 'VACIA_COMPROBADA',
    'FALLIDA': 'FALLIDA',
    'IDENTIDAD_INCIERTA': 'IDENTIDAD_INCIERTA',
    'PENDIENTE': 'CANCELADA',
    'CANCELADA': 'CANCELADA',
}


def _historia(manifest, data, query, incident, cid, captured):
    """Una consulta por causa (alcance CAUSA, sin remote_identity) y una fila por trámite dentro de la ventana."""
    estado, causas = cargar_historias(manifest)
    lote = Path(manifest).resolve().parent.parent.name
    data['historia_tramites'] = []
    vinculos = {}
    for v in estado['vinculos']:
        vinculos.setdefault(v['clave'], v)
    fallos = list(estado.get('errores_listado', []))
    if estado.get('estado') != 'COMPLETA' and not fallos:
        fallos = [{'error': 'HISTORIA_ENUMERACION_INCOMPLETA'}]
    for n, fallo in enumerate(fallos, 1):
        qid = f'hist-listado-{n}'
        query(
            qid,
            {'tab': 'HISTORIA', 'capture_tables': ['historia_tramites']},
            'FALLIDA',
            captured,
            screen='historia_tramites',
            error='HISTORIA_ENUMERACION_FALLIDA',
        )
        incident(qid, 'HISTORIA_ENUMERACION_FALLIDA', 'No se pudo enumerar toda la historia solicitada.')
    if not causas and not fallos:
        query(
            'hist-listado-vacio',
            {'tab': 'HISTORIA', 'capture_tables': ['historia_tramites']},
            'VACIA_COMPROBADA',
            captured,
            screen='historia_tramites',
        )
    for n, item in enumerate(causas, 1):
        causa = item['causa']
        qid = f'hist-{n}'
        cobertura = HISTORIA_COBERTURA.get(causa['estado'], 'FALLIDA')
        primera = vinculos.get(item['clave'], {})
        selection = {
            'tribunal': causa['tribunal'],
            'tribunal_nombre': causa.get('tribunal_nombre', ''),
            'modality': primera.get('modalidad', ''),
            'tab': 'HISTORIA',
            'capture_tables': ['historia_tramites'],
            'historia_causa': {'source_system': 'RUS', 'court_code': causa['tribunal'], 'cause_id': causa['causa_id']},
            'ventana': {
                'criterio': estado.get('criterio_temporal', 'ventana_4_meses'),
                'desde': causa.get('desde', ''),
                'hasta': causa.get('hasta', ''),
            },
        }
        cuando = causa.get('leida') or captured
        lectura_ok = cobertura in ('COMPLETA', 'VACIA_COMPROBADA')
        row = query(
            qid,
            selection,
            cobertura,
            cuando,
            1,
            1 if lectura_ok else 0,
            screen='historia_tramites',
            error=causa.get('error', '') if cobertura == 'FALLIDA' else '',
        )
        if cobertura == 'FALLIDA':
            incident(qid, 'HISTORIA_FALLIDA', causa.get('error') or 'No se pudo leer la historia.')
        elif cobertura == 'IDENTIDAD_INCIERTA':
            incident(qid, 'HISTORIA_IDENTIDAD_INCIERTA', causa.get('error') or 'Identidad de la causa no verificable.')
        elif cobertura == 'CANCELADA':
            incident(qid, 'HISTORIA_NO_LEIDA', 'La historia no se leyó (lote cancelado o con tope).')
        for codigo in causa.get('advertencias', []):
            incident(qid, codigo, 'Advertencia del lector de historia.')
        for fila in item['ventana']:
            common = {k: '' for k in COMMON}
            common.update(
                capture_id=cid,
                query_id=qid,
                source_system='RUS',
                court_code=row['court_code'],
                court_name=row['court_name'],
                rit_original=causa['rit'],
                cause_id=causa['causa_id'],
                modality_code=row['modality_code'],
                tab=primera.get('pestana', ''),
                captured_at=cuando,
                original_file=f'respaldo/{lote}/historia/' + causa['archivo'],
                original_row=str(fila['fila']),
                source_sha256=causa['sha256'],
                coverage='COMPLETA',
            )
            data['historia_tramites'].append(
                {
                    **common,
                    'event_id': f'{qid}-{fila["fila"]}',
                    'event_at': fila['fecha_iso'],
                    'time_precision': 'DIA' if fila['fecha_iso'] else 'DESCONOCIDA',
                    'time_zone': 'DESCONOCIDA',
                    'scope': 'CAUSA',
                    'event_type': 'tramite',
                    'description': fila['descripcion'],
                    'reference': fila['referencia'],
                    'identity_status': 'CAUSA',
                    'folio': fila['folio'],
                    'stage': fila['etapa'],
                    'annex_visible': 'true' if fila['anexo'] else 'false',
                    'raw_json': json.dumps(
                        {
                            'Folio': fila['folio'],
                            'Etapa': fila['etapa'],
                            'Trámite': fila['tramite'],
                            'Desc. Trámite': fila['descripcion'],
                            'Referencia': fila['referencia'],
                            'Fecha Tram': fila['fecha_texto'],
                        },
                        ensure_ascii=False,
                    ),
                }
            )
