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
    return dict(version=1, decisions=decisions, overrides=row.overrides,
                word_overrides=row.word_overrides, excluded=row.excluded,
                resolution_cell=row.review.get('RES', ''), proposal=row.observation)


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
        if not isinstance(decisions, dict) or not isinstance(data['overrides'], dict) or not isinstance(data['word_overrides'], dict):
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
            from nurus.rus.rules import as_date
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
            recalculate(work, row)
        if word_overrides is not None:
            row.word_overrides = deepcopy(word_overrides)
        if excluded is not None:
            row.excluded = bool(excluded)
        from .work import _sync_resolution_warning
        _sync_resolution_warning(row)
        updated.append(row)
    work.rows = updated
    work.revision += 1
    return before


def undo(work, before):
    work.rows = [deepcopy(before.get(row.id, row)) for row in work.rows]
    work.revision += 1


def recalculate(work, row):
    from datetime import date
    from nurus.rus.rules import tribunal, as_date, as_int
    from .motor.contexto import current
    from .motor.composicion import Incidencias
    from .motor.reglas_espera import generar_observacion_espera
    from .motor.reglas_cumplimiento import generar_observacion_cumplimiento
    from .motor.reglas_informes import generar_observacion_informes
    from .work import actions_for
    values = dict(row.values)
    for key, value in row.overrides.items():
        values[work.mapping[key]] = value
    ctx = dict(config=work.config, events=[], date=date.fromisoformat(work.as_of), invalid=[])
    token = current.set(ctx)
    incidences = Incidencias()
    warnings = []
    try:
        if work.mode == 'CUMPLIMIENTO':
            end = as_date(values.get(work.mapping.get('egreso_proy', '')))
            counts = [as_int(values.get(work.mapping.get(key, ''))) for key in ('dias_cumpl', 'dias_egresar')]
            if end and end > ctx['date'] and any(number is not None and number < 0 for number in counts):
                warnings.append('Días negativos y egreso futuro: verifica la contradicción; no se afirma que la medida esté vencida.')
                ctx['invalid'].append('CUMPLIMIENTO.C04_VENCIDA')
        court = tribunal(values.get(work.mapping.get('tribunal', ''), '')) or ''
        args = dict(incidencias=incidences, fila_excel=row.source_row)
        if work.mode == 'CUMPLIMIENTO':
            args['fecha_hoja2'] = as_date(getattr(work, 'cross_dates', {}).get(row.id))
        function = {'ESPERA': generar_observacion_espera, 'CUMPLIMIENTO': generar_observacion_cumplimiento,
                    'INFORMES': generar_observacion_informes}[work.mode]
        row.observation = function(values, court, work.mapping, **args) if court else ''
        if not court:warnings.append('Tribunal sin reglas configuradas; requiere revisión manual.')
        warnings.extend(item['MOTIVO'] for item in incidences._items)
    finally:
        current.reset(token)
    row.rules = list(dict.fromkeys(event for event in ctx['events'] if event not in work.config['desactivadas']))
    row.actions = actions_for(row.rules)
    row.warnings = warnings
