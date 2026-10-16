"""Hallazgos derivados: una sola construcción para edición, undo y sincronía."""

from copy import deepcopy
from typing import TypedDict
from nurus.capture.temporal import identity


class Finding(TypedDict):
    rule_id: str
    rule_version: str
    source_ids: list[str]
    record_identity: list[str]
    detected_at: str
    fact: str
    condition: str
    coverage: str
    status: str
    reason: str
    proposed_actions: list[str]
    confidence_class: str


def traditional_findings(work, row) -> list[Finding]:
    source = work.capture_sources[row.id]
    return [
        Finding(
            rule_id=rule,
            rule_version=work.ruleset_version,
            source_ids=[source['source_sha256']],
            record_identity=list(identity(source)),
            detected_at=work.as_of,
            fact=row.observation,
            condition='Regla tradicional conservada; no acredita actuaciones remotas.',
            coverage=source['coverage'],
            status='APLICA',
            reason='Evaluación del motor tradicional.',
            proposed_actions=list(row.actions),
            confidence_class='PARCIAL',
        )
        for rule in row.rules
    ]


def updated_contexts(work, rows):
    """Candidato independiente; quien modifica Work decide cuándo confirmarlo."""
    contexts = dict(getattr(work, 'capture_context', {}))
    for row in rows:
        if row.id not in contexts:
            continue
        context = deepcopy(contexts[row.id])
        contextual = [
            f for f in context['findings'] if not f['rule_id'].startswith(('ESPERA.', 'CUMPLIMIENTO.', 'INFORMES.'))
        ]
        context['findings'] = contextual + traditional_findings(work, row)
        contexts[row.id] = context
    return contexts


def _day(iso):
    """aaaa-mm-dd → dd/mm/aaaa."""
    return '/'.join(reversed(iso[:10].split('-')))


def _when(event):
    value = event.get('event_at', '')
    return _day(value) + (' ' + value[11:16] if 'T' in value else ' (sin hora)')


def display_of(work, row):
    return getattr(work, 'capture_context', {}).get(row.id, {}).get('display')


def last_center_label(work, row):
    """Columna «Última obs. centro»: solo muestra lo capturado; vacío si no hay paquete."""
    display = display_of(work, row)
    if not display:
        return ''
    if display['status'] == 'ORIGEN_DESCONOCIDO':
        return 'ORIGEN_DESCONOCIDO'
    if display['status'] == 'NO_SOLICITADA':
        return 'Bitácoras no solicitadas'
    if display['status'] == 'NO_LEIDA':
        return 'Bitácora no leída'
    if display['status'] != 'ACREDITADA':
        # Distingue «no hay bitácora» de «la bitácora solo tiene entradas fuera de la ventana de 4 meses».
        if display.get('outside_horizon'):
            return f"Sin entradas en 4 meses · {display['outside_horizon']} anterior" + (
                'es' if display['outside_horizon'] > 1 else ''
            )
        if display.get('after_cutoff'):
            return f"Sin entradas en el período · {display['after_cutoff']} tras el corte"
        return 'Sin entradas capturadas'
    label = _when(display['last_center'][0])[:10]
    pending = len(display['later']) + len(display['uncertain'])
    return label + (f' · {pending} posterior' + ('es' if pending > 1 else '') if pending else '')


def novelties_text(work, row):
    """Panel «Novedades posteriores»: texto íntegro, autor y origen; no acredita historia completa."""
    display = display_of(work, row)
    if not display:
        return 'Novedades posteriores: no disponibles (trabajo sin paquete de captura o identidad no verificada).'
    period = _day(display['period_from']) + ' a ' + _day(display['period_to'])
    lines = [f'Novedades posteriores · solo mostrar · período {period}']
    if display['status'] == 'NO_SOLICITADA':
        lines.append('El paquete no solicitó bitácoras: no hay entradas que mostrar.')
    elif display['status'] == 'NO_LEIDA':
        lines.append(
            'La bitácora de este ingreso no se leyó (límite de lecturas, falla o lote incompleto). '
            'En el Descargador usa «Retomar» sobre la carpeta de bitácoras y vuelve a crear el paquete.'
        )
    elif display['status'] == 'SIN_ENTRADAS_CAPTURADAS':
        lines.append('No hay entradas capturadas en el período; no acredita ausencia de actuaciones.')
    elif display['status'] == 'ORIGEN_DESCONOCIDO':
        lines.append('ORIGEN_DESCONOCIDO: ninguna entrada del período tiene origen del centro acreditado.')
        lines += _entries(display['unattributed'])
    else:
        lines.append('Última observación atribuible al centro:')
        lines += _entries(display['last_center'])
        lines.append('Entradas posteriores:' if display['later'] else 'Entradas posteriores: ninguna capturada.')
        lines += _entries(display['later'])
        if display['uncertain']:
            lines.append('Mismo día, orden incierto (sin hora comparable):')
            lines += _entries(display['uncertain'])
    if display['outside_horizon']:
        lines.append(f"{display['outside_horizon']} entrada(s) anteriores al período no se consideran.")
    if display.get('after_cutoff'):
        lines.append(f"{display['after_cutoff']} entrada(s) posteriores a la fecha de corte no se consideran.")
    if display.get('undated'):
        lines.append(f"{display['undated']} entrada(s) sin fecha no se pueden ubicar en la cronología.")
    return '\n'.join(lines)


def _entries(events):
    return [
        f"- {_when(e)} · {e.get('origin') or 'DESCONOCIDO'} · {e.get('author') or 'autor no informado'}: "
        f"{e.get('text', '')}"
        for e in events
    ]
