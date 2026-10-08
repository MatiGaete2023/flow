"""Encuentra el contrato común en desarrollo local o en el monorrepositorio."""
import sys
from pathlib import Path

def parser():
    try:
        from nurus.bitacora_html import read_snapshot
    except ModuleNotFoundError as exc:
        if exc.name != 'nurus': raise
        root=Path(__file__).resolve().parent
        source=next((p for p in (root.parent/'csmp'/'src',root.parent/'src')
                     if (p/'nurus'/'bitacora_html.py').is_file()),None)
        if source is None: raise RuntimeError('Falta el contrato común de bitácoras CSMP.') from None
        sys.path.insert(0,str(source))
        from nurus.bitacora_html import read_snapshot
    return read_snapshot
