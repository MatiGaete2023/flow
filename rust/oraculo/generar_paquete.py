"""Oráculo del paquete: arma un ZIP CSMP_CAPTURE_V1 ficticio con historia y bitácora, lo importa con el Asistente
Python y guarda lo que este propone, para que Rust lo compare.

Uso (desde la raíz del repo):
    python3 rust/oraculo/generar_paquete.py

Escribe rust/crates/flow-trabajo/tests/datos/paquete_historia.zip y paquete_historia.json.
"""

import json
import sys
from datetime import date
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(RAIZ / 'src'))
sys.path.insert(0, str(RAIZ / 'tests'))

from historia_fixtures import captura_con_historia  # noqa: E402
from nurus.capture import write_capture  # noqa: E402
from nurus.personal.capture_input import import_capture  # noqa: E402
from nurus.personal.config import defaults  # noqa: E402

CORTE = date(2026, 8, 20)
SERVICIO = 'Ing.Serv. protección especializada y otras redes'


def main():
    tramites = [
        (SERVICIO, '2026-08-01', 7),
        ('Orden de búsqueda', '2026-08-05', 9),
        ('Pide cuenta búsqueda', '2026-08-10', 10),
        ('Designa abogado', '2026-08-11', 11),
        ('Resp. continuidad de atención', '2026-08-12', 12),
    ]
    metadata, tablas = captura_con_historia(tramites, registros=4)
    destino = RAIZ / 'rust/crates/flow-trabajo/tests/datos'
    destino.mkdir(parents=True, exist_ok=True)
    paquete = destino / 'paquete_historia.zip'
    paquete.unlink(missing_ok=True)
    write_capture(paquete, metadata, tablas)
    work = import_capture(paquete, defaults(), as_of=CORTE)
    filas = [
        {'id': r.id, 'observacion': r.observation, 'reglas': list(r.rules), 'acciones': list(r.actions),
         'marcas': [{'nivel': n, 'codigo': c, 'texto': t} for n, c, t in r.avisos]}
        for r in work.rows
    ]
    (destino / 'paquete_historia.json').write_text(
        json.dumps({'corte': CORTE.isoformat(), 'filas': filas}, ensure_ascii=False, indent=1), encoding='utf-8')
    for f in filas:
        print(f['id'], '|', f['observacion'][:150], '|', f['acciones'])


if __name__ == '__main__':
    main()
