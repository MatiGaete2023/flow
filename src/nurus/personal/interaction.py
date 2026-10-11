"""Prevent edits while a worker consumes a stable snapshot."""

import tkinter as tk
from tkinter import ttk
import customtkinter as ctk


def lock(app):
    saved = []
    seen = set()
    types = (
        ctk.CTkButton,
        ctk.CTkEntry,
        ctk.CTkTextbox,
        ctk.CTkComboBox,
        ctk.CTkCheckBox,
        tk.Button,
        tk.Entry,
        tk.Text,
        tk.Spinbox,
        tk.Checkbutton,
        tk.Radiobutton,
        tk.Listbox,
        tk.Scale,
    )
    themed_types = (ttk.Button, ttk.Entry, ttk.Checkbutton, ttk.Radiobutton, ttk.Scale, ttk.Menubutton)

    def visit(widget):
        if widget in seen:
            return
        seen.add(widget)
        if isinstance(widget, themed_types):
            # ttk guarda readonly/selected como bits independientes: cambiar solo disabled.
            saved.append((widget, widget.instate(['disabled'])))
            widget.state(['disabled'])
            return
        if isinstance(widget, types):
            state = widget._textbox.cget('state') if isinstance(widget, ctk.CTkTextbox) else widget.cget('state')
            saved.append((widget, state))
            widget.configure(state='disabled')
            return
        for child in widget.winfo_children():
            visit(child)

    app._locked_inputs = saved
    try:
        for page in app.pages.values():
            visit(page)
        for child in app.winfo_children():
            if isinstance(child, tk.Toplevel):
                visit(child)
    except Exception:
        unlock(app)
        raise


def unlock(app):
    for widget, state in getattr(app, '_locked_inputs', []):
        if widget.winfo_exists():
            if isinstance(widget, ttk.Widget):
                widget.state(['disabled' if state else '!disabled'])
            else:
                widget.configure(state=state)
    app._locked_inputs = []
    # El estado guardado al bloquear puede ser viejo (p. ej. tras procesar un Excel o
    # restaurar borradores durante el bloqueo): se recalcula con la misma condición.
    from .mail_view import update_mail_actions

    update_mail_actions(app)
