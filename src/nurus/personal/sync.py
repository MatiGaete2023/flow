"""Transactional workbook synchronization by identity, never by row position."""
from copy import deepcopy
from dataclasses import dataclass
from datetime import date, datetime
from hashlib import sha256
from pathlib import Path

from nurus.rus.columns import normalize
from nurus.rus.rules import historical_match
from .importing import read_external

REVIEW_FIELDS = ('OBSERVACION', 'FECHA_OBS', 'TT', 'CC', 'RES')
IDENTITY_FIELDS = ('rit', 'rut', 'nombre', 'tribunal', 'programa')


@dataclass(frozen=True)
class Conflict:
    record_id: str
    field: str
    base: object
    local: object
    incoming: object


class SyncConflict(ValueError):
    def __init__(self, conflicts):
        self.conflicts = conflicts
        super().__init__(f'Hay {len(conflicts)} campos editados aquí y en Excel. '
                         'Resuelve las diferencias; no se aplicó ningún cambio.')


def comparable(value):
    if value is None:
        return ''
    if isinstance(value, datetime):
        return value.isoformat(sep=' ')
    if isinstance(value, date):
        return value.isoformat()
    return value


def associate(work, data):
    """Validate the entire identity set before returning row associations."""
    by_id = {row.id: row for row in work.rows}
    fields = tuple(key for key in IDENTITY_FIELDS if work.mapping.get(key))
    if not all(key in fields for key in ('rit', 'nombre', 'tribunal')):
        raise ValueError('Faltan campos de identidad; no se aplicaron cambios.')

    def identity(values, mapping):
        return tuple(historical_match(values.get(mapping.get(key, ''), '')) for key in fields)

    by_identity = {}
    for row in work.rows:
        by_identity.setdefault(identity(row.values, work.mapping), []).append(row)
    result = {}
    for number, values, review, rules in data['records']:
        id_column = next((key for key in values if normalize(key) == normalize('NURUS_ID_REGISTRO')), None)
        if id_column is not None:
            row = by_id.get(str(values[id_column]).strip())
            if row is None:
                raise ValueError('Hay identidades ajenas o vacías en la copia; no se aplicaron cambios.')
        else:
            candidates = by_identity.get(identity(values, data['mapping']), [])
            if len(candidates) != 1:
                raise ValueError('Falta la columna de identidad y hay registros ambiguos o modificados; no se aplicaron cambios.')
            row = candidates[0]
        if row.id in result:
            raise ValueError('Hay identidades duplicadas en la copia; no se aplicaron cambios.')
        if identity(row.values, work.mapping) != identity(values, data['mapping']):
            raise ValueError('Cambió la identidad de una fila; no se aplicaron cambios.')
        result[row.id] = (number, values, review, rules)
    if set(result) != set(by_id):
        raise ValueError('Faltan registros en la copia; no se aplicaron cambios.')
    return result


def remember_export(work):
    data = read_external(work.output, work.mode, work.sheet)
    records = associate(work, data)
    for row in work.rows:
        row.sync_base = deepcopy(records[row.id][2])
        row.sync_local = deepcopy(row.review)
        row.sync_state = str(records[row.id][1].get('NURUS_DECISIONES', '') or '')


def refresh(work, resolutions=None):
    """Merge into a copy; commit bytes, mapping and positions together.

    resolutions maps (record_id, field) to the user's chosen value. Omitted
    conflicts raise SyncConflict and leave every part of Work unchanged.
    """
    if not work.output:
        raise ValueError('Primero procesa y exporta el trabajo.')
    path = Path(work.output)
    if not path.exists():
        raise ValueError('La copia fue movida. Usa Localizar copia para indicar su nueva ubicación.')
    data = read_external(path, work.mode, work.sheet)
    if data['digest'] == work.output_hash:
        return False
    incoming = associate(work, data)
    from copy import copy
    recalculation=copy(work)
    recalculation.mapping=data['mapping']
    recalculation.header=data['header']
    recalculation.sheet=data['sheet']
    conflicts = []
    updated = deepcopy(work.rows)
    for row in updated:
        number, values, review, _ = incoming[row.id]
        from .record_edits import decode, encode, restore, recalculate
        technical = str(values.get('NURUS_DECISIONES', '') or '')
        technical_data = decode(technical)
        baseline_data = decode(row.sync_state)
        if technical_data is not None and technical_data != baseline_data:
            local_state = encode(row)
            local_data = decode(local_state)
            if local_data != baseline_data and local_data != technical_data:
                key = (row.id, 'NURUS_DECISIONES')
                if resolutions is not None and key in resolutions:
                    technical = resolutions[key]
                else:
                    conflicts.append(Conflict(row.id, key[1], row.sync_state, local_state, technical))
            restore(row, technical)
        merged = deepcopy(row.review)
        for field in REVIEW_FIELDS:
            remote = review.get(field, '')
            base = (row.sync_base or {}).get(field, '')
            local = row.review.get(field, base)
            # Absence of a local review is not a command to erase a source cell.
            local_changed = (field in row.review and
                             comparable(local) != comparable((row.sync_local or {}).get(field, base)))
            if row.sync_base is None:
                # Old sessions have no baseline: never silently overwrite a local value.
                local_changed = field in row.review and comparable(local) != comparable(remote)
            remote_changed = row.sync_base is None or comparable(remote) != comparable(base)
            if local_changed and remote_changed and comparable(local) != comparable(remote):
                key = (row.id, field)
                if resolutions is not None and key in resolutions:
                    merged[field] = resolutions[key]
                else:
                    conflicts.append(Conflict(row.id, field, base, local, remote))
            elif remote_changed:
                merged[field] = remote
        if comparable(review.get('RES','')) != comparable((row.sync_base or {}).get('RES','')):
            row.decisions['resolution']='review'
        row.review = merged
        row.sync_base = deepcopy(review)
        # Track the remote baseline, not the retained local edits.
        row.sync_local = deepcopy(review)
        row.sync_state = str(values.get('NURUS_DECISIONES', '') or '')
        row.source_row = number
        changed_source = any(comparable(row.values.get(column, '')) != comparable(values.get(data['mapping'].get(key, ''), ''))
                             for key, column in work.mapping.items() if key not in IDENTITY_FIELDS)
        row.values = values
        if changed_source and (row.rules or not getattr(work, 'external_input', False)):
            recalculate(recalculation, row)
    if conflicts:
        raise SyncConflict(conflicts)
    from .work import _sync_resolution_warning
    for row in updated:
        _sync_resolution_warning(row)
    if not getattr(work, 'original_hash', ''):
        work.original_hash = work.source_hash
        work.original_content = work.content
    work.rows = updated
    work.content = data['content']
    work.source_hash = data['digest']
    work.output_hash = data['digest']
    work.sheet = data['sheet']
    work.header = data['header']
    work.mapping = data['mapping']
    work.revision = getattr(work, 'revision', 0) + 1
    return True


def require_current_copy(work):
    """Generators consume a stable revision and never import on the caller's behalf."""
    if not getattr(work, 'output', '') or not hasattr(work, 'output_hash'):
        return
    path = Path(work.output)
    if not path.is_file():
        raise ValueError('La copia fue movida. Usa Localizar copia antes de preparar productos.')
    if sha256(path.read_bytes()).hexdigest() != work.output_hash:
        raise ValueError('Excel tiene cambios pendientes. Usa Actualizar desde Excel antes de preparar o guardar productos.')
