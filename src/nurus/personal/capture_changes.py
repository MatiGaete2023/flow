"""Comparación anónima entre dos capturas de un trabajo."""

from __future__ import annotations

NUEVO = 'NUEVO'
CAMBIO_FUENTE = 'CAMBIO_FUENTE'
CAMBIO_REGLAS = 'CAMBIO_REGLAS'
SIN_CAMBIO = 'SIN_CAMBIO'


def _visible(work, row, key):
    try:
        from .outputs import value

        return value(work, row, key)
    except (AttributeError, KeyError):
        return str(getattr(row, 'values', {}).get(key, '') or '')


def compare(candidate, previous, matches, changed_by_row):
    if not previous or not getattr(previous, 'capture_digest', ''):
        return None
    old = {row.id: row for row in previous.rows}
    rows = {}
    for row in candidate.rows:
        prior = old.get(matches.get(row.id))
        changed = tuple(changed_by_row.get(row.id, ()))
        if prior is None:
            kind, before, after = NUEVO, [], []
        else:
            rules_changed = list(getattr(prior, 'rules', ())) != list(getattr(row, 'rules', ())) or (
                getattr(prior, 'observation', '') != getattr(row, 'observation', '')
            )
            before = list(getattr(prior, 'rules', ())) if rules_changed else []
            after = list(getattr(row, 'rules', ())) if rules_changed else []
            kind = CAMBIO_FUENTE if changed else CAMBIO_REGLAS if rules_changed else SIN_CAMBIO
        rows[row.id] = {
            'tipo': kind,
            'campos': list(changed) if kind == CAMBIO_FUENTE else [],
            'reglas_antes': before,
            'reglas_despues': after,
        }
    matched_old = set(matches.values())
    egresados = []
    for row in previous.rows:
        if row.id in matched_old:
            continue
        egresados.append(
            {
                'id_anterior': row.id,
                'rit': _visible(previous, row, 'rit'),
                'nombre': _visible(previous, row, 'nombre'),
                'tribunal': _visible(previous, row, 'tribunal'),
                'programa': _visible(previous, row, 'programa'),
            }
        )
    return {'version': 1, 'base_digest': previous.capture_digest, 'filas': rows, 'egresados': egresados}


def summary(changes):
    counts = {key: 0 for key in (NUEVO, CAMBIO_FUENTE, CAMBIO_REGLAS, SIN_CAMBIO)}
    if changes:
        for item in changes.get('filas', {}).values():
            if item.get('tipo') in counts:
                counts[item['tipo']] += 1
        counts['EGRESADOS'] = len(changes.get('egresados', ()))
    else:
        counts['EGRESADOS'] = 0
    return counts


def label(changes, row_id):
    item = (changes or {}).get('filas', {}).get(row_id, {})
    return {NUEVO: 'Nueva', CAMBIO_FUENTE: 'Fuente', CAMBIO_REGLAS: 'Reglas'}.get(item.get('tipo'), '')
