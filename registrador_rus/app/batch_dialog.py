"""Diálogo de revisión de un lote exclusivamente ficticio; no autoriza intentos."""
import tkinter as tk
from tkinter import ttk, messagebox
from .batch_store import selection_digest


class BatchReviewDialog:
    def __init__(self, parent, operations, on_confirm):
        self.digest = selection_digest(operations)
        self.on_confirm = on_confirm
        self.window = tk.Toplevel(parent)
        self.window.title('Revisión exacta del lote — SIMULACIÓN')
        self.window.transient(parent)
        self.window.minsize(720, 420)
        self.accepted = tk.BooleanVar(master=self.window, value=False)
        body = ttk.Frame(self.window, padding=12); body.pack(fill='both', expand=True)
        ttk.Label(body, text=f'{len(operations)} operaciones ficticias. Revisar identidad, texto, Estado y destino.').pack(anchor='w')
        panel = ttk.Frame(body); panel.pack(fill='both', expand=True, pady=6)
        self.text = tk.Text(panel, wrap='word', height=14)
        scroll = ttk.Scrollbar(panel, orient='vertical', command=self.text.yview)
        self.text.configure(yscrollcommand=scroll.set)
        scroll.pack(side='right', fill='y'); self.text.pack(side='left', fill='both', expand=True)
        for op in operations:
            i = op.remote_identity
            self.text.insert('end', f'{op.operation_id} | {op.origin} | Tribunal {i.tribunal_id} / Causa {i.causa_id} / Ingreso {i.ingreso_id}\nDestino: {op.destination} | Estado: {op.observation_state}\n{op.approved_text}\n\n')
        self.text.configure(state='disabled')
        ttk.Checkbutton(body, text='Confirmo la revisión de este conjunto exacto (solo simulación).', variable=self.accepted,
                        command=self._toggle).pack(anchor='w')
        buttons = ttk.Frame(body); buttons.pack(fill='x', pady=6)
        self.confirm_button = ttk.Button(buttons, text='Confirmar revisión', command=self.confirm, state='disabled')
        self.confirm_button.pack(side='right')
        ttk.Button(buttons, text='Cancelar', command=self.window.destroy).pack(side='right', padx=4)
        self.window.grab_set()

    def _toggle(self):
        self.confirm_button.configure(state='normal' if self.accepted.get() else 'disabled')

    def confirm(self):
        if not self.accepted.get():
            return
        try:
            self.on_confirm(self.digest)
        except (ValueError, RuntimeError) as exc:
            messagebox.showwarning('Revisión no registrada', str(exc), parent=self.window)
            return
        self.window.destroy()
