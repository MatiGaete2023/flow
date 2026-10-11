"""Decisiones explícitas antes de sustituir el trabajo de una captura."""

import tkinter as tk
import json
from . import ui


def resolve(app, error):
    window = tk.Toplevel(app)
    window.title('Conciliar capturas')
    window.geometry('780x420')
    window.transient(app)
    window.grab_set()
    label = tk.StringVar()
    detail = tk.StringVar()
    answers = {}
    index = 0
    ui.Label(window, textvariable=label, wraplength=740).pack(fill='x', padx=18, pady=16)
    ui.Label(window, textvariable=detail, wraplength=740).pack(fill='x', padx=18, pady=16)
    values = ui.Textbox(window, wrap='word', height=8)
    values.pack(fill='both', expand=True, padx=18)
    ui.Label(
        window,
        text='La sesión anterior se archivará. Los productos preparados requieren actualización al cambiar la captura.',
        wraplength=740,
    ).pack(fill='x', padx=18)
    buttons = ui.Frame(window)
    buttons.pack(fill='x', padx=18, pady=16)

    def choose(answer):
        nonlocal index
        answers[error.conflicts[index].record_id] = answer
        index += 1
        if index == len(error.conflicts):
            window.destroy()
            error.retry(answers)
        else:
            show()

    keep = ui.Button(buttons, text='Conservar mis decisiones', command=lambda: choose('KEEP'))
    keep.pack(side='left')
    ui.Button(buttons, text='Usar nueva evaluación', command=lambda: choose('RESET')).pack(side='left', padx=8)
    ui.Button(buttons, text='Cancelar importación', command=window.destroy).pack(side='right')

    def show():
        conflict = error.conflicts[index]
        old = next(r for r in app.work.rows if r.id == conflict.previous_ids[0])
        from .outputs import value

        label.set(
            f'{index + 1}/{len(error.conflicts)} · {value(app.work, old, "rit")} · '
            f'{value(app.work, old, "nombre")} · {conflict.reason}'
        )
        detail.set(
            'Campos distintos: ' + ', '.join(conflict.changed_fields)
            if conflict.changed_fields
            else 'No existe una correspondencia única. No se trasladarán decisiones automáticamente.'
        )
        values.configure(state='normal')
        values.delete('1.0', 'end')
        values.insert(
            '1.0',
            json.dumps(
                {
                    'Fuente anterior': conflict.previous_values,
                    'Fuente nueva': conflict.incoming_values,
                    'Decisiones anteriores': old.review,
                },
                ensure_ascii=False,
                indent=2,
                default=str,
            ),
        )
        values.configure(state='disabled')
        keep.configure(state='normal' if conflict.can_keep else 'disabled')

    show()
