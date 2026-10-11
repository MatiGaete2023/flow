"""Encuentra el contrato común en desarrollo local o en el monorrepositorio."""

import sys
from pathlib import Path


def parser():
    # La instalación desde ZIP debe usar su contrato, también al recuperar
    # datos offline, aunque haya otra entrega instalada en el entorno Python.
    root = Path(__file__).resolve().parent
    source = next(
        (
            p
            for p in (root.parent / 'csmp' / 'src', root.parent / 'src')
            if (p / 'nurus' / 'bitacora_html.py').is_file()
        ),
        None,
    )
    if source is not None and str(source) not in sys.path:
        sys.path.insert(0, str(source))
    try:
        from nurus.bitacora_html import read_snapshot
    except ModuleNotFoundError as exc:
        if exc.name != 'nurus':
            raise
        raise RuntimeError('Falta el contrato común de bitácoras CSMP.') from None
    return read_snapshot
