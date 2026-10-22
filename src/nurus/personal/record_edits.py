"""Validated edits and decisions shared by the UI, workbook and session."""

from copy import deepcopy
import json

MAIL_KINDS = ('programa_espera', 'programa_vencido', 'programa_por_vencer', 'medidas')
RESOLUTIONS = ('auto', 'review', 'none', 'PC_IE', 'PC_INFO', 'NOMENCL')
MAIL_CHOICES = ('auto', 'include', 'omit')
WORD_FIELDS = ('RIT', 'RUT', 'NOMBRE', 'PROGRAMA', 'FECHA', 'FECHA_RESOLUCION', 'DURACION')


def state(row):
    decisions = deepcopy(row.decisions)
    if decisions.get('structured') and 'RES' in row.review:
        decisions['resolution'] = 'review'
    return dict(
        version=1,
        decisions=decisions,
        overrides=row.overrides,
        word_overrides=row.word_overrides,
        excluded=row.excluded,
        resolution_cell=row.review.get('RES', ''),
        proposal=row.observation,
    )


def encode(row):
    result = json.dumps(state(row), ensure_ascii=False, sort_keys=True, default=str)
    if len(result) > 32000:
        raise ValueError('Las correcciones del registro superan la capacidad de una celda Excel.')
    return result


def decode(raw):
    if not raw:
        return None
    try:
        data = json.loads(raw)
        if data.get('version') != 1:
            raise ValueError('versión desconocida')
        decisions = data['decisions']
        if (
            not isinstance(decisions, dict)
            or not isinstance(data['overrides'], dict)
            or not isinstance(data['word_overrides'], dict)
        ):
            raise ValueError('estructura inválida')
        if decisions.get('resolution', 'auto') not in RESOLUTIONS:
            raise ValueError('resolución inválida')
        if not isinstance(decisions.get('mail', {}), dict):
            raise ValueError('gestiones inválidas')
        if any(value not in MAIL_CHOICES for value in decisions.get('mail', {}).values()):
            raise ValueError('gestión inválida')
        if type(data['excluded']) is not bool:
            raise ValueError('inclusión inválida')
        return data
    except (ValueError, TypeError, KeyError, AttributeError) as exc:
        raise ValueError('La columna técnica NURUS_DECISIONES no es válida. No se importaron cambios.') from exc


def restore(row, raw):
    data = decode(raw)
    if data is None:
        return
    row.decisions = data['decisions']
    row.overrides = data['overrides']
    row.word_overrides = data['word_overrides']
    row.excluded = data['excluded']
    row.observation = str(data.get('proposal', row.observation))
    if str(row.review.get('RES', '') or '') != str(data.get('resolution_cell', '') or ''):
        row.decisions['resolution'] = 'review'


def mail_decision(row, kind):
    return row.decisions.get('mail', {}).get(kind, 'auto')


def validate_review(changes):
    result = dict(changes)
    if 'FECHA_OBS' in result:
        raw = str(result['FECHA_OBS'] or '').strip()
        if raw:
            from nurus.rus.util import as_date

            parsed = as_date(raw)
            if parsed is None:
                raise ValueError('FECHA_OBS debe ser una fecha válida dd/mm/aaaa o quedar vacía.')
            result['FECHA_OBS'] = parsed.strftime('%d/%m/%Y')
    for key in ('TT', 'CC'):
        if key in result:
            raw = str(result[key]).strip()
            if raw not in ('', '0', '1'):
                raise ValueError(key + ' debe quedar vacío o ser 0 o 1.')
            result[key] = int(raw) if raw else ''
    if 'RES' in result:
        from .resolutions import resolution_review_issue

        issue = resolution_review_issue(result['RES'])
        if issue:
            raise ValueError(issue)
    return result


def apply(work, record_ids, *, review=None, decisions=None, overrides=None, word_overrides=None, excluded=None):
    ids = set(record_ids)
    if not ids or not ids <= {row.id for row in work.rows}:
        raise ValueError('Selecciona registros del trabajo actual.')
    review = validate_review(review or {})
    if len(ids) > 1 and ('OBSERVACION' in review or overrides or word_overrides):
        raise ValueError('Los datos personales y la observación se editan por registro.')
    if decisions is not None:
        if decisions.get('resolution', 'auto') not in RESOLUTIONS:
            raise ValueError('Tipo de resolución no válido.')
        if any(key not in MAIL_KINDS or value not in MAIL_CHOICES for key, value in decisions.get('mail', {}).items()):
            raise ValueError('Gestión de correo no válida.')
    if word_overrides and not set(word_overrides) <= set(WORD_FIELDS):
        raise ValueError('Variable Word no admitida.')
    if overrides and not set(overrides) <= set(work.mapping):
        raise ValueError('Columna de origen no reconocida.')
    before = {row.id: deepcopy(row) for row in work.rows if row.id in ids}
    updated = []
    for old in work.rows:
        if old.id not in ids:
            updated.append(old)
            continue
        row = deepcopy(old)
        row.review.update(review)
        if decisions is not None:
            row.decisions.update(deepcopy(decisions))
            row.decisions['structured'] = True
            mode = decisions.get('resolution')
            if mode == 'auto':
                row.review.pop('RES', None)
            elif mode in ('none', 'PC_IE', 'PC_INFO', 'NOMENCL'):
                row.review['RES'] = '' if mode == 'none' else mode
        if overrides is not None:
            row.overrides = deepcopy(overrides)
            recalculate(work, row, update_findings=False)
        if word_overrides is not None:
            row.word_overrides = deepcopy(word_overrides)
        if excluded is not None:
            row.excluded = bool(excluded)
        from .work import _sync_resolution_warning

        _sync_resolution_warning(row)
        updated.append(row)
    changed = set(ids)
    if overrides is not None:
        # Una edición puede crear o deshacer una duplicidad (R6) o un acumulado (J2) en otra fila de la causa:
        # esas filas se recalculan con las demás ya editadas, en la misma operación (y se deshacen con ella).
        from copy import copy

        view = copy(work)
        view.rows = updated
        peers = _prepass_peers(work, work.rows, updated, ids)
        for index, peer in enumerate(updated):
            if peer.id in peers:
                before[peer.id] = deepcopy(peer)
                candidate = deepcopy(peer)
                recalculate(view, candidate, update_findings=False)
                updated[index] = candidate
                changed.add(peer.id)
    from .capture_findings import updated_contexts

    # Solo las filas editadas (y sus pares afectados) cambian sus hallazgos; las demás conservan su contexto (EF-02).
    contexts = updated_contexts(work, [row for row in updated if row.id in changed])
    work.rows = updated
    if hasattr(work, 'capture_context'):
        work.capture_context = contexts
    work.revision += 1
    return before


def undo(work, before):
    from .capture_findings import updated_contexts

    rows = [deepcopy(before.get(row.id, row)) for row in work.rows]
    contexts = updated_contexts(work, rows)
    work.rows = rows
    if hasattr(work, 'capture_context'):
        work.capture_context = contexts
    work.revision += 1


def recalculate(work, row, *, update_findings=True):
    from .capture_findings import updated_contexts

    candidate = deepcopy(row)
    _recalculate(work, candidate)
    contexts = updated_contexts(work, [candidate]) if update_findings else None
    row.__dict__.update(candidate.__dict__)
    if update_findings and hasattr(work, 'capture_context'):
        work.capture_context = contexts


def _rows_prepass(work, rows, mode, mapping):
    """Pre-pase RUS (duplicidad R6, informes acumulados J2) sobre las filas de una etapa, con sus correcciones."""
    from nurus.rus.models import Mode, ReadBatch, SourceRecord, SourceReference

    records = []
    for item in rows:
        if getattr(work, 'capture_modes', {}).get(item.id, work.mode) != mode:
            continue
        data = dict(item.values)
        data.update({work.mapping[k]: v for k, v in item.overrides.items()})
        records.append(SourceRecord(data, SourceReference('', '', '', item.source_row), item.id))
    return work._prepass(ReadBatch(Mode(mode), tuple(records)), mapping, mode=mode)


def _prepass_peers(work, before_rows, after_rows, ids):
    """Filas no editadas cuya duplicidad o acumulado cambia por la edición (p. ej. RUT, RIT o vencimiento)."""
    modes = {getattr(work, 'capture_modes', {}).get(row.id, work.mode) for row in after_rows if row.id in ids}
    peers = set()
    for mode in modes:
        if mode not in ('ESPERA', 'CUMPLIMIENTO', 'INFORMES'):
            continue
        old = _rows_prepass(work, before_rows, mode, dict(work.mapping))
        new = _rows_prepass(work, after_rows, mode, dict(work.mapping))
        for table_old, table_new in zip(old, new):
            peers.update(rid for rid in set(table_old) | set(table_new) if table_old.get(rid) != table_new.get(rid))
    return peers - set(ids)


def _recalculate(work, row):
    from datetime import date
    from nurus.rus.util import tribunal, as_date, as_int
    from .motor.contexto import current
    from .motor.composicion import Incidencias
    from .motor.reglas_espera import generar_observacion_espera
    from .motor.reglas_cumplimiento import generar_observacion_cumplimiento
    from .motor.reglas_informes import generar_observacion_informes
    from .work import actions_for

    mode = getattr(work, 'capture_modes', {}).get(row.id, work.mode)
    mapping = dict(work.mapping)
    source = getattr(work, 'capture_sources', {}).get(row.id)
    if source and source['record_type'] not in ('ESPERA', 'CUMPLIMIENTO', 'INFORMES'):
        row.observation = ''
        row.rules = []
        row.actions = []
        row.warnings = ['Tipo de fuente fuera del motor tradicional; no evaluado.']
        return
    if source:
        import json
        from nurus.rus.columns import map_columns

        allowed = map_columns(json.loads(source['normalized_json']), mode)
        mapping = {key: column for key, column in mapping.items() if key in allowed or key in row.overrides}
    values = dict(row.values)
    for key, value in row.overrides.items():
        values[mapping[key]] = value
    from .capture_input import captured_rule_context

    corte = date.fromisoformat(work.as_of)
    history, diary = captured_rule_context(work, source)
    rows = [row if original.id == row.id else original for original in work.rows]
    duplicity, accumulated = _rows_prepass(work, rows, mode, mapping)
    ctx = dict(
        config=work.config,
        events=[],
        date=corte,
        invalid=[],
        marcas=[],
        historia=history.al_corte(corte) if history is not None else None,
        bitacora=tuple(o for o in diary if o.fecha <= corte),
        duplicidad=duplicity.get(row.id),
        acumulado=accumulated.get(row.id),
    )
    token = current.set(ctx)
    incidences = Incidencias()
    warnings = []
    try:
        if mode == 'CUMPLIMIENTO':
            end = as_date(values.get(mapping.get('egreso_proy', '')))
            counts = [as_int(values.get(mapping.get(key, ''))) for key in ('dias_cumpl', 'dias_egresar')]
            if end and end > ctx['date'] and any(number is not None and number < 0 for number in counts):
                warnings.append(
                    'Días negativos y egreso futuro: verifica la contradicción; no se afirma que la medida esté vencida.'
                )
                ctx['invalid'].append('CUMPLIMIENTO.C04_VENCIDA')
        court = tribunal(values.get(mapping.get('tribunal', ''), '')) or ''
        args = dict(incidencias=incidences, fila_excel=row.source_row)
        if mode == 'CUMPLIMIENTO':
            args['fecha_hoja2'] = as_date(getattr(work, 'cross_dates', {}).get(row.id))
        function = {
            'ESPERA': generar_observacion_espera,
            'CUMPLIMIENTO': generar_observacion_cumplimiento,
            'INFORMES': generar_observacion_informes,
        }[mode]
        row.observation = function(values, court, mapping, **args) if court else ''
        if not court:
            warnings.append('Tribunal sin reglas configuradas; requiere revisión manual.')
        warnings.extend(item['MOTIVO'] for item in incidences._items)
    finally:
        current.reset(token)
    row.rules = list(dict.fromkeys(event for event in ctx['events'] if event not in work.config['desactivadas']))
    from datetime import date as _date

    corte = _date.fromisoformat(work.as_of) if getattr(work, 'as_of', '') else None
    row.actions = actions_for(row.rules, as_date(getattr(work, 'cross_dates', {}).get(row.id)), corte)
    row.avisos = list(dict.fromkeys(ctx['marcas']))
    context = getattr(work, 'capture_context', {}).get(row.id)
    if context:
        warnings.append(context['reason'])
    row.warnings = warnings
