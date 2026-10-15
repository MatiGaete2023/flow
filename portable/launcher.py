"""Entrada portable: reutiliza la GUI y los motores de flow sin modificarlos."""
from pathlib import Path
import sys

if not getattr(sys, 'frozen', False):
    ROOT = Path(__file__).resolve().parents[1]
    sys.path[:0] = [str(ROOT / 'src'), str(ROOT / 'tools')]

from anonimizar_local import App
import tkinter as tk


def create_app():
    root = tk.Tk()
    app = App(root)
    root.title('Anonimizador CSMP portable')
    root.update_idletasks()
    # La ventana original de 315 px recorta estados largos y advertencias.
    root.geometry(f'{max(640, root.winfo_reqwidth())}x{max(430, root.winfo_reqheight() + 100)}')
    return root, app


def main():
    if len(sys.argv) == 3 and sys.argv[1] == '--self-test':
        from selftest import run
        return run(Path(sys.argv[2]), create_app)
    if len(sys.argv) != 1:
        return 2
    root, _ = create_app()
    root.mainloop()
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
