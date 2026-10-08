"""Resolve just the fields that conflict, without changing Work until acceptance."""
import tkinter as tk
import customtkinter as ctk
from . import ui


def resolve(app, conflict):
    window = ctk.CTkToplevel(app)
    window.title('Cambios en Excel y en esta sesión')
    window.geometry('820x600')
    window.transient(app)
    window.grab_set()
    current = tk.IntVar(value=0)
    answers = {}
    title = tk.StringVar()
    ui.Label(window, textvariable=title, wraplength=760).pack(fill='x', padx=14, pady=12)
    tabs = ui.Notebook(window)
    tabs.pack(fill='both', expand=True, padx=14)
    boxes = {}
    for key, label in (('local', 'Edición aquí'), ('incoming', 'Excel'), ('base', 'Última sincronización')):
        panel = ui.Frame(tabs)
        tabs.add(panel, text=label)
        box = ui.Textbox(panel, wrap='word')
        box.pack(fill='both', expand=True)
        boxes[key] = box
    ui.Label(window, text='Valor a conservar (puedes combinar o corregir):').pack(anchor='w', padx=14, pady=6)
    chosen = ui.Textbox(window, height=5, wrap='word')
    chosen.pack(fill='x', padx=14)
    buttons = ui.Frame(window)
    buttons.pack(fill='x', padx=14, pady=10)

    def show():
        item = conflict.conflicts[current.get()]
        row = next(row for row in app.work.rows if row.id == item.record_id)
        from .outputs import value
        title.set(f'{current.get()+1}/{len(conflict.conflicts)} · {value(app.work,row,"rit")} · {value(app.work,row,"nombre")} · {item.field}')
        for key, box in boxes.items():
            box.configure(state='normal')
            box.delete('1.0', 'end')
            box.insert('1.0', str(getattr(item, key)))
            box.configure(state='disabled')
        choose(item.local)

    def choose(value):
        chosen.delete('1.0', 'end')
        chosen.insert('1.0', str(value))

    def accept():
        item = conflict.conflicts[current.get()]
        from .record_edits import validate_review
        value = chosen.get('1.0', 'end-1c')
        answers[(item.record_id, item.field)] = validate_review({item.field: value})[item.field]
        if current.get() + 1 < len(conflict.conflicts):
            current.set(current.get() + 1)
            show()
        else:
            window.destroy()
            app._refresh_resolved(answers,path=getattr(conflict,'path',None))

    ui.Button(buttons, text='Usar Excel', command=lambda: choose(conflict.conflicts[current.get()].incoming)).pack(side='left')
    ui.Button(buttons, text='Usar mi edición', command=lambda: choose(conflict.conflicts[current.get()].local)).pack(side='left', padx=6)
    ui.Button(buttons, text='Cancelar', command=window.destroy).pack(side='right')
    ui.Button(buttons, text='Aceptar y continuar', command=lambda: app._guard(accept)).pack(side='right', padx=6)
    show()
