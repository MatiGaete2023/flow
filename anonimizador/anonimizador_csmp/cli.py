"""Línea de comandos. Sin argumentos abre la ventana."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from . import __version__
from .engine import LABELS, report_text, run
from .textengine import Options


def _window_factory():
    try:
        import tkinter as tk
    except ImportError:
        return None

    def create():
        from .gui import App

        root = tk.Tk()
        root.withdraw()
        App(root)
        return root

    return create


def build_parser():
    parser = argparse.ArgumentParser(
        prog="AnonimizadorCSMP",
        description="Anonimiza localmente datos del Descargador/Asistente: Excel, HTML, JSON, CSV, ICS, TXT, Word y ZIP. "
        "Sin argumentos abre la ventana.",
    )
    parser.add_argument("entradas", nargs="*", type=Path, help="archivos o carpetas a anonimizar")
    parser.add_argument("--modo", choices=("estricto", "conservador"), default="estricto")
    parser.add_argument("--salida", type=Path, help="carpeta de destino (por defecto, junto al original)")
    parser.add_argument("--desplazar-fechas", type=int, default=0, metavar="DIAS")
    parser.add_argument(
        "--semilla", default="", help="frase opcional para que los seudónimos sean iguales entre ejecuciones"
    )
    parser.add_argument("--generalizar-nacimiento", action="store_true", help="fechas de nacimiento al 1 de enero")
    parser.add_argument(
        "--rechazar-no-admitidos", action="store_true", help="un ZIP con miembros no admitidos falla completo"
    )
    parser.add_argument("--version", action="version", version=f"%(prog)s {__version__}")
    return parser


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    if len(argv) == 2 and argv[0] == "--self-test":
        from .selftest import run_selftest

        return run_selftest(Path(argv[1]), _window_factory())
    args = build_parser().parse_args(argv)
    if not args.entradas:
        from .gui import main as gui

        return gui()
    options = Options(
        mode=args.modo,
        shift_days=args.desplazar_fechas,
        seed=args.semilla,
        skip_unsupported=not args.rechazar_no_admitidos,
        generalize_birth=args.generalizar_nacimiento,
    )
    missing = [p for p in args.entradas if not p.exists()]
    if missing:
        print("No existe(n) una o más entradas.", file=sys.stderr)
        return 2
    result = run(args.entradas, options, args.salida)
    for item in result.items:
        state = f"OK -> {item.output}" if item.ok else f"NO SE CREÓ: {item.detail}"
        print(f"{item.label} ({LABELS.get(item.kind, item.kind or 'desconocido')}): {state}")
    for out in result.outputs:
        print("Creado:", out)
    for warning in report_text(result, options).split("\n"):
        if warning.startswith("Advertencia:"):
            print(warning)
    return 0 if result.ok else 1
