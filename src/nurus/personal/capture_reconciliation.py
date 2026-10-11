"""Conciliación local sin asociar personas por nombre/RIT ni borrar conflictos."""

from collections import defaultdict
from copy import deepcopy
from dataclasses import dataclass, field
import json
from nurus.capture.temporal import identity

EDIT_FIELDS = ('review', 'decisions', 'overrides', 'word_overrides', 'excluded')


@dataclass(frozen=True)
class CaptureConflict:
    record_id: str
    previous_ids: tuple[str, ...]
    reason: str
    can_keep: bool
    changed_fields: tuple[str, ...] = ()
    previous_values: dict = field(default_factory=dict)
    incoming_values: dict = field(default_factory=dict)


class ReconciliationRequired(ValueError):
    def __init__(self, conflicts):
        self.conflicts = tuple(conflicts)
        super().__init__(
            f'Hay {len(conflicts)} conflictos entre capturas; revise las decisiones. El trabajo anterior sigue intacto.'
        )


def _normalized(source):
    return json.loads(source['normalized_json'])


def _key(source):
    scope = tuple(source.get(k, '') for k in ('record_type', 'modality_code', 'tab', 'person_id', 'program_id'))
    if source['identity_status'] == 'VERIFICADA' and all(identity(source)):
        return ('REMOTE', *identity(source), *scope)
    # Una fila física íntegra idéntica puede recuperarse sin inferir identidad remota.
    return (
        'PHYSICAL',
        *identity(source),
        *scope,
        source['source_sha256'],
        source['original_sheet'],
        source['original_row'],
        json.dumps(_normalized(source), sort_keys=True, ensure_ascii=False),
    )


def _decided(decisions):
    """analyze siempre deja {'structured': True, 'mail': {}, 'resolution': 'auto'}: eso no es una edición."""
    return (
        decisions.get('resolution', 'auto') != 'auto'
        or any(value != 'auto' for value in decisions.get('mail', {}).values())
        or any(value for key, value in decisions.items() if key not in ('structured', 'mail', 'resolution'))
    )


def _edited(row):
    return any(_decided(row.decisions) if field == 'decisions' else getattr(row, field) for field in EDIT_FIELDS)


def _remap_session(session, id_map):
    def visit(value, key=''):
        if isinstance(value, dict):
            return {k: visit(v, k) for k, v in value.items()}
        if isinstance(value, list):
            if key in ('record_ids', 'records', 'prepared_selection'):
                return [id_map.get(v, v) for v in value]
            return [visit(v) for v in value]
        if key == 'id' and isinstance(value, str):
            return id_map.get(value, value)
        return value

    saved = visit(deepcopy(session))
    for key in ('drafts', 'projects'):
        for product in saved.get(key, []):
            product['dependency_hash'] = product.get('dependency_hash') or 'REVISAR_CAPTURA_ANTERIOR'
    return saved


def reconcile(candidate, previous, *, resolutions=None):
    """Confirmar solo un plan completo; RESET es siempre una decisión explícita."""
    if not previous or not getattr(previous, 'capture_digest', ''):
        candidate.capture_reconciliation = {'continued': False, 'id_map': {}}
        return
    resolutions = resolutions or {}
    old_rows = {row.id: row for row in previous.rows}
    old_sources = previous.capture_sources
    old_by_key = defaultdict(list)
    new_by_key = defaultdict(list)
    for rid, source in old_sources.items():
        old_by_key[_key(source)].append(rid)
    for rid, source in candidate.capture_sources.items():
        new_by_key[_key(source)].append(rid)
    same_bytes = previous.capture_digest == candidate.capture_digest
    matches = {}
    conflicts = []
    consumed = set()
    resolved = set()
    for row in candidate.rows:
        source = candidate.capture_sources[row.id]
        key = _key(source)
        prior_ids = [row.id] if same_bytes and row.id in old_rows else old_by_key.get(key, [])
        if not prior_ids:
            continue
        unique = len(prior_ids) == 1 and (same_bytes or len(new_by_key[key]) == 1)
        changed = ()
        if unique:
            old_data = _normalized(old_sources[prior_ids[0]])
            new_data = _normalized(source)
            changed = tuple(sorted(k for k in old_data.keys() | new_data.keys() if old_data.get(k) != new_data.get(k)))
        has_edits = any(_edited(old_rows[rid]) for rid in prior_ids)
        conflict = (not unique or bool(changed)) and has_edits
        if conflict:
            answer = resolutions.get(row.id)
            if answer == 'RESET':
                consumed.update(prior_ids)
                resolved.add(row.id)
                continue
            if answer != 'KEEP' or not unique:
                conflicts.append(
                    CaptureConflict(
                        row.id,
                        tuple(prior_ids),
                        'Datos fuente modificados.' if unique else 'Correspondencia ambigua.',
                        unique,
                        changed,
                        {k: old_data.get(k) for k in changed} if unique else {},
                        {k: new_data.get(k) for k in changed} if unique else {},
                    )
                )
                consumed.update(prior_ids)
                continue
            resolved.add(row.id)
        if unique and (not changed or resolutions.get(row.id) == 'KEEP'):
            matches[row.id] = prior_ids[0]
        consumed.update(prior_ids)
    for rid, row in old_rows.items():
        if rid not in consumed and _edited(row):
            conflict_id = 'missing:' + rid
            if resolutions.get(conflict_id) != 'RESET':
                conflicts.append(CaptureConflict(conflict_id, (rid,), 'Registro anterior no presente.', False))
            else:
                resolved.add(conflict_id)
    if set(resolutions) - resolved:
        raise ValueError('Respuesta de conciliación ajena o innecesaria; trabajo anterior intacto.')
    if conflicts:
        raise ReconciliationRequired(conflicts)
    for row in candidate.rows:
        if row.id not in matches:
            continue
        prior = old_rows[matches[row.id]]
        for field in EDIT_FIELDS:
            setattr(row, field, deepcopy(getattr(prior, field)))
        if same_bytes:
            for field in ('sync_base', 'sync_local', 'sync_state'):
                setattr(row, field, deepcopy(getattr(prior, field)))
        if row.overrides:
            from .record_edits import recalculate

            recalculate(candidate, row, update_findings=False)
    id_map = {old: new for new, old in matches.items()}
    candidate.capture_reconciliation = {
        'continued': same_bytes or bool(matches),
        'id_map': id_map,
        'resolved': sorted(resolved),
    }
    if same_bytes or matches:
        candidate.session = _remap_session(previous.session, id_map)
        candidate.receipts = deepcopy(previous.receipts)
        candidate.revision = previous.revision + 1
    if same_bytes:
        candidate.output = previous.output
        candidate.output_hash = previous.output_hash
