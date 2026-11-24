"""E-V1 · La referencia Python es una sola: los árboles `descargador/`, `src/` y `tests/` de HEAD son los del commit de
`app/qa/REFERENCIA_PYTHON.json`, y la CI hace checkout de ese mismo SHA. Sale con 1 ante cualquier diferencia.

Uso (desde la raíz): python tools/verificar_referencia_python.py
"""

import json
import subprocess
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[1]


def main():
    ref = json.loads((RAIZ / 'app' / 'qa' / 'REFERENCIA_PYTHON.json').read_text(encoding='utf-8'))
    errores = []
    for carpeta, esperado in ref['arboles'].items():
        actual = subprocess.run(['git', 'rev-parse', f'HEAD:{carpeta}'], cwd=RAIZ, capture_output=True, text=True).stdout.strip()
        if actual != esperado:
            errores.append(f'{carpeta}/ difiere de la referencia {ref["commit"][:12]} (HEAD {actual[:12]}, referencia {esperado[:12]})')
    flujo = (RAIZ / '.github' / 'workflows' / 'flow-rust-local.yml').read_text(encoding='utf-8')
    if f'ref: {ref["commit"]}' not in flujo:
        errores.append('la CI no hace checkout del SHA de la referencia')
    for e in errores:
        print('DIFERENCIA', e)
    print(f'REFERENCIA_PYTHON: {ref["commit"][:12]}; RESULTADO: {"FALLA" if errores else "PASA"} ({len(errores)} diferencias)')
    return 1 if errores else 0


if __name__ == '__main__':
    sys.exit(main())
