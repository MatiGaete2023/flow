"""Crea una copia anónima de un Excel RUS/SITFA para compartir un caso de prueba.

Uso (en el PC autorizado):
    python tools/anonimizar_excel.py original.xlsx copia_anonima.xlsx [--desplazar-fechas DIAS]

La lógica vive en nurus.personal.anonymize (la misma que usa el botón de Configuración).
Nunca sobrescribe archivos. Revisa la copia antes de compartirla.
"""
import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from nurus.personal.anonymize import *  # noqa: E402,F401,F403
from nurus.personal.anonymize import export_copy  # noqa: E402


def main(argv):
    parser = argparse.ArgumentParser(description="Copia anónima de un Excel RUS/SITFA.")
    parser.add_argument("origen", type=Path)
    parser.add_argument("destino", type=Path)
    parser.add_argument("--desplazar-fechas", type=int, default=0, metavar="DIAS")
    args = parser.parse_args(argv)
    try:
        changed, sheets = export_copy(args.origen, args.destino, args.desplazar_fechas)
    except ValueError as exc:
        print(exc)
        return 1
    print(f"Copia anónima creada: {args.destino} ({changed} celdas reemplazadas en {sheets} hojas). "
          "Revísala antes de compartirla.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
