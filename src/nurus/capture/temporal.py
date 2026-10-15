"""Cronología explicada; no atribuye revisiones locales a actuaciones oficiales."""

from datetime import date, datetime
import json
from calendar import monthrange

RULESET_VERSION = 'baseline-20261008+context-preview-2+historia-3'
# Reglas activas solo para mostrar (D5, WP-50). Debe coincidir con contracts/csmp_context_v1/catalog.json.
DISPLAY_RULES = ('CT-01', 'CT-02')
DISPLAY_VERSION = 'solo-mostrar-3'
CONTEXT_RULES = {
    'CT-01': ('bitacoras', 'Última observación atribuible al centro'),
    'CT-02': ('bitacoras', 'Entradas posteriores y empates inciertos'),
    'CT-03': ('historia_tramites', 'Trámites posteriores; título no acredita contenido'),
    'CT-04': ('historia_tramites', 'Discordancia temporal requiere cotejo'),
    'CT-05': ('bitacoras', 'Respuesta requiere comprobación de contenido'),
    'CT-06': ('consultas', 'Fuente o identidad necesaria no disponible'),
    'CT-07': ('bitacoras', 'Posible reiteración; no suprimir propuesta'),
    'CT-08': ('historia_tramites', 'Firma requiere cobertura e identidad'),
    'CT-09': ('calendarios', 'Calendario no acredita recepción'),
    'CT-10': ('incidencias', 'Contradicciones requieren examen humano'),
}


def identity(row):
    return tuple(row.get(k, '') for k in ('source_system', 'court_code', 'cause_id', 'ingreso_id'))


def moment(row):
    value = row.get('event_at', '')
    if not value:
        return None
    return datetime.fromisoformat(value.replace('Z', '+00:00')) if 'T' in value else date.fromisoformat(value)


def earliest(as_of):
    """Ventana de cuatro meses naturales; no busca antecedentes más antiguos."""
    month = as_of.year * 12 + as_of.month - 1 - 4
    year, zero_month = divmod(month, 12)
    return date(year, zero_month + 1, min(as_of.day, monthrange(year, zero_month + 1)[1]))


def certain_moment(row):
    value = moment(row)
    return (
        row.get('time_precision') == 'SEGUNDO'
        and row.get('time_zone') not in ('', 'DESCONOCIDA')
        and isinstance(value, datetime)
        and value.utcoffset() is not None
    )


def center_entries(events):
    return [
        e
        for e in events
        if e.get('origin') == 'CENTRO'
        and e.get('origin_verified') == 'true'
        and e.get('coverage') in ('COMPLETA', 'PARCIAL')
    ]


def order(related):
    """Última entrada atribuible al centro, posteriores y empates de orden incierto."""
    center = center_entries(related)
    if not center:
        return [], [], []
    last_day = max(e['event_at'][:10] for e in center)
    same_day = [e for e in center if e['event_at'][:10] == last_day]
    if not all(certain_moment(e) for e in same_day):
        last = same_day
    else:
        try:
            maximum = max(moment(e) for e in same_day)
            last = [e for e in same_day if moment(e) == maximum]
        except TypeError:
            last = same_day
    later, uncertain = [], []
    last_ids = {e['event_id'] for e in last}
    for event in related:
        if event['event_id'] in last_ids:
            continue
        if event['event_at'][:10] > last_day:
            later.append(event)
        elif event['event_at'][:10] == last_day:
            if all(certain_moment(e) for e in [event, *last]):
                try:
                    if moment(event) > max(moment(e) for e in last):
                        later.append(event)
                except TypeError:
                    uncertain.append(event)
            else:
                uncertain.append(event)
    return last, later, uncertain


def display_chronology(events, as_of, *, requested=True, read=True):
    """CT-01/CT-02 en modo «solo mostrar» (D5): no genera hallazgos ni cambia observaciones.

    Solo bitácoras del ingreso dentro del horizonte de cuatro meses. Sin origen acreditado del
    centro se informa ORIGEN_DESCONOCIDO; sin entradas capturadas no se afirma ausencia. Si el
    paquete no solicitó bitácoras se informa NO_SOLICITADA. Si la bitácora del ingreso no se leyó
    (límite de lecturas, falla o lote incompleto) se informa NO_LEIDA: nunca «sin entradas». Las
    entradas sin fecha y las posteriores al corte se cuentan aparte: no son «anteriores al período».
    """
    start, end = earliest(as_of).isoformat(), as_of.isoformat()
    dated = [e for e in events if e.get('event_at', '')]
    recent = sorted(
        (e for e in dated if start <= e['event_at'][:10] <= end),
        key=lambda e: (e['event_at'], e.get('event_id', '')),
    )
    last, later, uncertain = order(recent)
    if last:
        status = 'ACREDITADA'
    elif recent:
        status = 'ORIGEN_DESCONOCIDO'
    elif not requested and not events:
        status = 'NO_SOLICITADA'
    elif not read and not events:
        status = 'NO_LEIDA'
    else:
        status = 'SIN_ENTRADAS_CAPTURADAS'
    return {
        'version': DISPLAY_VERSION,
        'period_from': start,
        'period_to': end,
        'status': status,
        'last_center': last,
        'later': later,
        'uncertain': uncertain,
        'unattributed': [] if last else recent,
        'outside_horizon': sum(1 for e in dated if e['event_at'][:10] < start),
        'after_cutoff': sum(1 for e in dated if e['event_at'][:10] > end),
        'undated': len(events) - len(dated),
    }


def attach_displays(capture, contexts, records, as_of):
    """Agrega la vista «solo mostrar» a cada contexto con identidad verificada (no toca hallazgos).

    Con DISPLAY_RULES vacío (rollback documentado) quita la vista que una sesión anterior haya guardado.
    """
    if not DISPLAY_RULES:
        for context in contexts.values():
            context.pop('display', None)
        return
    requested = capture.manifest['table_states'].get('bitacoras') != 'NO_SOLICITADA'
    # Lectura por ingreso: el Descargador deja una consulta de bitácora con su identidad remota.
    # Un paquete sin esas consultas (versiones anteriores, fixtures) no permite distinguir y no se marca.
    read = {}
    for query in capture.rows('consultas'):
        if query.get('screen') != 'bitacoras':
            continue
        try:
            scope = json.loads(query.get('filters_json') or '{}').get('remote_identity')
        except ValueError:
            scope = None
        if isinstance(scope, dict):
            key = identity(scope)
            read[key] = read.get(key, False) or query.get('coverage') not in ('FALLIDA', 'NO_CONSULTADA')
    diaries = {}
    for event in capture.rows('bitacoras'):
        if event.get('identity_status') == 'VERIFICADA' and event.get('scope', 'INGRESO') == 'INGRESO':
            diaries.setdefault(identity(event), []).append(event)
    for record_id, record in records.items():
        if record_id in contexts and all(identity(record)) and record.get('identity_status') == 'VERIFICADA':
            contexts[record_id]['display'] = display_chronology(
                diaries.get(identity(record), ()),
                as_of,
                requested=requested,
                read=read.get(identity(record), False) if read else True,
            )


def chronology(capture, record, as_of, *, fixture_preview=False, event_index=None):
    key = identity(record)
    result = {
        'identity': list(key),
        'last_center': [],
        'later': [],
        'uncertain': [],
        'recent': [],
        'findings': [],
        'class': 'NO_EVALUABLE',
        'reason': 'Interpretación temporal no aceptada institucionalmente; no acredita ausencia de actuaciones.',
    }

    def finding(code, reason, status='NO_EVALUABLE', sources=()):
        result['findings'].append(
            {
                'rule_id': code,
                'rule_version': RULESET_VERSION,
                'source_ids': list(sources),
                'record_identity': list(key),
                'detected_at': as_of.isoformat(),
                'fact': reason,
                'condition': CONTEXT_RULES[code][1],
                'coverage': record['coverage'],
                'status': status,
                'reason': reason,
                'proposed_actions': [],
                'confidence_class': 'INCIERTA',
            }
        )

    if not all(key) or record.get('identity_status') != 'VERIFICADA':
        result['reason'] = 'Identidad de ingreso no verificada; no se relacionan fuentes por RIT o nombre.'
        finding('CT-06', result['reason'])
        return result
    if event_index is None:
        related = [
            event
            for table in ('bitacoras', 'historia_tramites')
            for event in capture.rows(table)
            if identity(event) == key
            and event.get('identity_status') == 'VERIFICADA'
            and event.get('scope', 'INGRESO') == 'INGRESO'
        ]
    else:
        related = event_index.get(key, ())
    start = earliest(as_of)
    result['period_from'] = start.isoformat()
    result['period_to'] = as_of.isoformat()
    result['recent'] = [
        e for e in related if e.get('event_at', '') and start.isoformat() <= e['event_at'][:10] <= as_of.isoformat()
    ]
    if not fixture_preview or capture.manifest['contains_sensitive_data']:
        finding('CT-06', result['reason'], sources=[e['event_id'] for e in result['recent']])
        return result
    # Solo preview ficticio explícito. Garantías del manifiesto no activan reglas.
    last, later, uncertain = order(result['recent'])
    if not last:
        result['reason'] = 'ULTIMA_OBSERVACION_NO_ACREDITADA dentro del período capturado.'
        finding('CT-06', result['reason'])
        return result
    result['last_center'], result['later'], result['uncertain'] = last, later, uncertain
    result['reason'] = 'Preview ficticio acotado a las fuentes capturadas; no acredita historia completa.'
    result['class'] = 'FOCALIZADA' if result['later'] or result['uncertain'] else 'NO_EVALUABLE'
    finding('CT-01', result['reason'], 'APLICA', [e['event_id'] for e in last])
    finding(
        'CT-02',
        'Eventos posteriores o de orden incierto; contenido pendiente de cotejar.',
        'APLICA' if result['later'] else 'NO_EVALUABLE',
        [e['event_id'] for e in [*result['later'], *result['uncertain']]],
    )
    return result
