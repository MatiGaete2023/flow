"""Métrica local opcional: solo conteos y marcas de tiempo, nunca datos de casos."""

from __future__ import annotations

from datetime import datetime, timezone
import json
from pathlib import Path

from .config import atomic_json


def _enabled(work):
    return bool(getattr(work, 'config', {}).get('metricas_locales', False) and getattr(work, 'storage_directory', None))


def _now():
    return datetime.now(timezone.utc).isoformat(timespec='seconds')


def _read(path):
    try:
        data = json.loads(Path(path).read_text(encoding='utf-8'))
        return data if isinstance(data, dict) and isinstance(data.get('ciclos'), list) else {'version': 1, 'ciclos': []}
    except (OSError, ValueError, TypeError):
        return {'version': 1, 'ciclos': []}


def _write(work, data):
    try:
        atomic_json(Path(work.storage_directory) / 'metricas.json', data)
    except (OSError, ValueError, TypeError):
        pass


def _edited(row):
    from .capture_reconciliation import _edited as is_edited

    try:
        return bool(is_edited(row))
    except (AttributeError, TypeError):
        return False


def record_import(work):
    if not _enabled(work):
        return
    from .capture_changes import summary

    data = _read(Path(work.storage_directory) / 'metricas.json')
    changes = summary(getattr(work, 'capture_changes', None))
    rows = list(getattr(work, 'rows', ()))
    reviewed = [row for row in rows if _edited(row)]
    data['ciclos'].append(
        {
            'importado': _now(),
            'exportado': '',
            'primer_borrador': '',
            'ultimo_borrador': '',
            'filas': len(rows),
            'nuevas': changes['NUEVO'],
            'cambio_fuente': changes['CAMBIO_FUENTE'],
            'cambio_reglas': changes['CAMBIO_REGLAS'],
            'egresadas': changes['EGRESADOS'],
            'revisadas': len(reviewed),
            'revisadas_sin_cambio': sum(
                (getattr(work, 'capture_changes', None) or {}).get('filas', {}).get(row.id, {}).get('tipo')
                == 'SIN_CAMBIO'
                for row in reviewed
            ),
            'ediciones_despues_de_exportar': 0,
        }
    )
    data['ciclos'] = data['ciclos'][-60:]
    _write(work, data)


def _latest(work):
    data = _read(Path(work.storage_directory) / 'metricas.json')
    return data, (data['ciclos'][-1] if data['ciclos'] else None)


def record_export(work):
    if not _enabled(work):
        return
    data, cycle = _latest(work)
    if cycle is None:
        return
    cycle['exportado'] = _now()
    _write(work, data)


def record_draft(work):
    if not _enabled(work):
        return
    data, cycle = _latest(work)
    if cycle is None:
        return
    stamp = _now()
    cycle['primer_borrador'] = cycle.get('primer_borrador') or stamp
    cycle['ultimo_borrador'] = stamp
    _write(work, data)


def summary(path):
    """Devuelve el documento sin añadir rutas ni información de personas."""
    data = _read(path)
    return {'version': 1, 'ciclos': data.get('ciclos', [])[-60:]}
