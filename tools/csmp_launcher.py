"""Arranque Windows y diagnóstico aislado de distribución."""

import sys
from pathlib import Path
import json
import os
import subprocess


def downloader_command(base, *, frozen):
    base = Path(base).resolve()
    if frozen:
        for directory in (base, base.parent):
            binary = directory / 'SITFA_Descargador' / 'SITFA_Descargador.exe'
            if binary.is_file():
                return [str(binary)], binary.parent
    else:
        script = base / 'descargador' / 'descargador.py'
        if script.is_file():
            return [sys.executable, str(script)], script.parent
    raise FileNotFoundError('Falta el Descargador en esta instalación. Reinstala el paquete integral.')


def choose_mode():
    import tkinter as tk
    from tkinter import ttk
    from nurus.ui_theme import install_ttk

    root = tk.Tk()
    root.title('CSMP · Elegir aplicación')
    root.resizable(False, False)
    install_ttk(root, 'Claro')
    result = []

    def choose(mode):
        result.append(mode)
        root.destroy()

    body = ttk.Frame(root, padding=24)
    body.pack()
    ttk.Label(body, text='¿Qué necesitas hacer ahora?', style='Title.TLabel').pack(anchor='w', pady=(0, 16))
    ttk.Label(body, text='Asistente: revisar un paquete local y preparar productos.').pack(anchor='w')
    assistant = ttk.Button(body, text='Abrir Asistente', style='Primary.TButton', command=lambda: choose('asistente'))
    assistant.pack(fill='x', pady=(8, 16))
    ttk.Label(body, text='Descargador: consultar SITFA y crear un paquete.').pack(anchor='w')
    ttk.Button(body, text='Abrir Descargador', command=lambda: choose('descargador')).pack(fill='x', pady=8)
    root.bind('<Alt-a>', lambda _event: choose('asistente'))
    root.bind('<Alt-d>', lambda _event: choose('descargador'))
    root.bind('<Escape>', lambda _event: root.destroy())
    assistant.focus_set()
    root.mainloop()
    return result[0] if result else None


def run(argv=None):
    args = list(sys.argv[1:] if argv is None else argv)
    # Diagnóstico y atajos legados siguen abriendo el Asistente directamente.
    if args:
        if args[0] == '--asistente':
            args.pop(0)
        from nurus.personal.app import main

        return main(args)
    mode = choose_mode()
    if mode is None:
        return None
    if mode == 'asistente':
        from nurus.personal.app import main

        return main([])
    frozen = bool(getattr(sys, 'frozen', False))
    base = Path(sys.executable).parent if frozen else Path(__file__).resolve().parents[1]
    try:
        command, cwd = downloader_command(base, frozen=frozen)
        environment = dict(os.environ)
        if not frozen:
            environment['PYTHONPATH'] = str(base / 'src') + os.pathsep + environment.get('PYTHONPATH', '')
        subprocess.Popen(command, cwd=cwd, env=environment)
    except OSError as exc:
        from tkinter import messagebox

        messagebox.showerror('No se pudo abrir el Descargador', str(exc))


if __name__ == '__main__':
    try:
        run()
    except Exception as exc:
        option = '--verificar-paquete' if '--verificar-paquete' in sys.argv else None
        if option:
            index = sys.argv.index(option) + 1
            if index < len(sys.argv):
                target = Path(sys.argv[index]).resolve()
                target.parent.mkdir(parents=True, exist_ok=True)
                target.with_suffix('.error.json').write_text(
                    json.dumps({'ok': False, 'error': str(exc)}, ensure_ascii=False), encoding='utf-8'
                )
            sys.exit(1)
        raise
