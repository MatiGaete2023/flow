"""Marca como consignada una constancia de «ingreso efectivo a otro programa» en una sesión guardada del Asistente.

Uso (desde la raíz):
  python tools/consignar.py CARPETA_SESION --listar
  python tools/consignar.py CARPETA_SESION --fila ID --programa "NOMBRE DEL PROGRAMA"

Solo para constancias que YA quedaron registradas en RUS: la propuesta deja de repetirlas. Es idempotente y nunca se llama al
generar o exportar. No escribe en RUS ni envía nada.
"""

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'src'))

from nurus.personal.config import defaults  # noqa: E402
from nurus.personal.motor.evidencia import clave_programa  # noqa: E402
from nurus.personal.work import Work  # noqa: E402


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('sesion')
    ap.add_argument('--listar', action='store_true')
    ap.add_argument('--fila')
    ap.add_argument('--programa')
    a = ap.parse_args(argv)
    trabajo = Work.load(a.sesion, defaults())
    if a.listar:
        for fila in trabajo.rows:
            for propuesta in fila.otros_programas:
                print(f'{fila.id}\t{propuesta["programa"]}')
        return 0
    if not (a.fila and a.programa):
        ap.error('Indica --fila y --programa (o --listar).')
    fila = next((r for r in trabajo.rows if r.id == a.fila), None)
    if fila is None:
        print('Esa fila no existe en la sesión.', file=sys.stderr)
        return 1
    if clave_programa(a.programa) not in {p['clave'] for p in fila.otros_programas}:
        print('El motor no propone esa constancia para la fila (ya consignada o no vigente).', file=sys.stderr)
        return 1
    trabajo.consignar(fila.id, a.programa)
    trabajo.save(a.sesion)
    print('Consignada; la propuesta de la fila ya no la repite.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
