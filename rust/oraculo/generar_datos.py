"""Copia los textos y umbrales del Asistente Python a los datos que incrusta el motor Rust.

Uso (desde la raíz del repo):
    python3 rust/oraculo/generar_datos.py

Escribe rust/datos/textos.json (aplanado «GRUPO.CLAVE» → plantilla, desde src/nurus/personal/textos_base.json) y
rust/datos/umbrales.json (desde nurus.personal.config.UMBRALES). Solo textos y números de configuración: sin datos.
"""

import json
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(RAIZ / 'src'))

from nurus.personal.config import UMBRALES  # noqa: E402


def main():
    base = json.loads((RAIZ / 'src/nurus/personal/textos_base.json').read_text(encoding='utf-8'))
    textos = {f'{grupo}.{clave}': dato['texto'] for grupo, claves in base.items() if grupo != '_meta'
              for clave, dato in claves.items()}
    destino = RAIZ / 'rust' / 'datos'
    (destino / 'textos.json').write_text(json.dumps(textos, ensure_ascii=False, indent=1), encoding='utf-8')
    (destino / 'umbrales.json').write_text(json.dumps(dict(UMBRALES), ensure_ascii=False, indent=1), encoding='utf-8')
    print(f'{len(textos)} textos ({base["_meta"]["version"]}) y {len(UMBRALES)} umbrales copiados del Python.')


if __name__ == '__main__':
    main()
