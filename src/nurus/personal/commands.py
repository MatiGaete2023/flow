"""One command registry for discoverable keyboard navigation."""

from dataclasses import dataclass
import tkinter as tk
from tkinter import ttk
from nurus.rus.columns import normalize


@dataclass
class Command:
    name: str
    action: object
    available: object
    reason: str = ''


def find_doc(name):
    """Manual o video junto al programa instalado ({app}/manuales) o en docs/manuales del repositorio."""
    import sys
    from pathlib import Path

    folders = []
    if getattr(sys, 'frozen', False):
        exe = Path(sys.executable).resolve().parent
        folders += [exe / 'manuales', exe.parent / 'manuales']
    folders.append(Path(__file__).resolve().parents[3] / 'docs' / 'manuales')
    return next((folder / name for folder in folders if (folder / name).is_file()), None)


def open_doc(app, name):
    import os
    import subprocess
    import sys

    path = find_doc(name)
    if path is None:
        app.status.set(f'No se encontró {name} junto al programa: búscalo en la carpeta de manuales de la entrega.')
    elif sys.platform == 'win32':
        os.startfile(path)  # noqa: S606 - manual local de la entrega
    else:
        subprocess.Popen(['xdg-open', str(path)])


def commands(app):
    from .mail_controls import mail_block  # misma condición que el botón de Correos

    ready = lambda: bool(app.work and getattr(app.work, 'output', ''))
    items = [
        Command(
            'Siguiente acción del flujo',
            app.next_step.next_action,
            lambda: not app.busy and not getattr(app, 'rendering', False),
        ),
        Command('Elegir Excel · Ctrl+O', app._choose, lambda: True),
        Command('Importar último paquete del Descargador', app._import_latest_capture, lambda: True),
        Command('Guardar trabajo · Ctrl+S', app._save_session, lambda: bool(app.work), 'Carga un trabajo primero.'),
        Command('Buscar registros · Ctrl+F', app._shortcut_find, lambda: bool(app.work), 'Carga un trabajo primero.'),
        Command('Actualizar Excel · F5', app._refresh, ready, 'Genera la copia de trabajo primero.'),
        Command('Abrir carpeta de productos', app._open_output_folder, ready, 'Genera la copia de trabajo primero.'),
        Command('Manual de uso (PDF) · F1', lambda: open_doc(app, 'Manual_Asistente_CSMP.pdf'), lambda: True),
        Command('Video: Asistente CSMP', lambda: open_doc(app, 'Flow_Video_Asistente_CSMP.mp4'), lambda: True),
    ]
    for name, page in app.pages.items():
        items.append(Command('Ir a ' + name, lambda p=page: app.tabs.select(p), lambda: True))
    items.extend(
        [
            Command('Preparar todos los correos', app._prepare_all_mail, ready, 'Carga un trabajo primero.'),
            Command(
                'Revisar y guardar borradores en Outlook',
                app._send_all,
                lambda: not mail_block(app, 'guardar_todos'),
                'Genera la copia de trabajo y prepara los correos primero.',
            ),
            Command('Preparar proyectos Word', app._prepare_words, ready, 'Carga un trabajo primero.'),
            Command(
                'Generar Word revisado',
                app._generate_words,
                lambda: bool(app.projects),
                'Prepara los proyectos primero.',
            ),
        ]
    )
    from .current_tools import deliver
    from .work_tools import show_quality, show_deadlines, preferences

    items.extend(
        [
            Command('Preparar ZIP de productos', lambda: deliver(app), ready, 'Genera productos primero.'),
            Command('Revisar calidad de datos', lambda: show_quality(app), ready, 'Carga un trabajo primero.'),
            Command('Ver vencimientos', lambda: show_deadlines(app), ready, 'Carga un trabajo primero.'),
            # Con el editor plegado, la pestaña Detalle no se ve: la paleta mantiene el acceso.
            Command('Preferencias de vista (columnas de Trabajo)', lambda: preferences(app), lambda: True),
        ]
    )
    return items


def show_palette(app):
    if app.busy or getattr(app, 'rendering', False):
        app.status.set('Espera a que termine la operación actual.')
        return
    previous = getattr(app, 'command_palette', None)
    if previous and previous.winfo_exists():
        previous.lift()
        return
    focus = app.focus_get()
    win = tk.Toplevel(app)
    app.command_palette = win
    win.title('Buscar acción · Ctrl+K')
    win.geometry('640x430')
    win.transient(app)
    query = tk.StringVar(win)
    entry = ttk.Entry(win, textvariable=query)
    entry.pack(fill='x', padx=16, pady=16)
    tree = ttk.Treeview(win, columns=('accion', 'estado'), show='headings', selectmode='browse')
    tree.heading('accion', text='Acción')
    tree.heading('estado', text='Disponible')
    tree.column('accion', width=370)
    tree.column('estado', width=180)
    tree.pack(fill='both', expand=True, padx=16)
    info = tk.StringVar()
    ttk.Label(win, textvariable=info, wraplength=600).pack(fill='x', padx=16, pady=10)
    candidates = []

    def refresh(*_):
        candidates[:] = [c for c in commands(app) if normalize(query.get()) in normalize(c.name)]
        tree.delete(*tree.get_children())
        for i, c in enumerate(candidates):
            tree.insert('', 'end', iid=str(i), values=(c.name, 'Sí' if c.available() else c.reason))
        if candidates:
            tree.selection_set('0')

    def close():
        win.destroy()
        if focus and focus.winfo_exists():
            focus.focus_set()

    def execute(event=None):
        selection = tree.selection()
        if selection:
            command = candidates[int(selection[0])]
            if not command.available():
                info.set(command.reason)
                return 'break'
            close()
            app._guard(command.action)
        return 'break'

    def down(event=None):
        tree.focus_set()
        return 'break'

    query.trace_add('write', refresh)
    refresh()
    entry.bind('<Down>', down)
    entry.bind('<Return>', execute)
    tree.bind('<Return>', execute)
    tree.bind('<Double-1>', execute)
    win.bind('<Escape>', lambda e: close())
    win.protocol('WM_DELETE_WINDOW', close)
    ttk.Button(win, text='Ejecutar acción', command=execute).pack(pady=(0, 12))
    entry.focus_set()
