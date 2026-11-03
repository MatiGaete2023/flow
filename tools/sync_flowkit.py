"""Copia ui/flowkit.py (fuente única del kit de interfaz) junto al código de cada producto.

python tools/sync_flowkit.py          # actualiza las copias
python tools/sync_flowkit.py --check  # falla si alguna copia difiere
"""

from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'ui' / 'flowkit.py'
COPIES = (
    ROOT / 'anonimizador' / 'anonimizador_csmp' / 'flowkit.py',
    ROOT / 'META4_Bitacoras' / 'flowkit.py',
    ROOT / 'descargador_lite' / 'app' / 'flowkit.py',
    ROOT / 'descargador' / 'flowkit.py',
)


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    data = SOURCE.read_bytes()
    stale = [c for c in COPIES if not c.is_file() or c.read_bytes() != data]
    if '--check' in argv:
        for c in stale:
            print('Copia desactualizada:', c.relative_to(ROOT))
        return 1 if stale else 0
    for c in stale:
        c.write_bytes(data)
        print('Actualizada:', c.relative_to(ROOT))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
