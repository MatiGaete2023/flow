"""Prevent edits while a worker consumes a stable snapshot."""
import tkinter as tk
import customtkinter as ctk


def lock(app):
    saved=[]
    types=(ctk.CTkButton, ctk.CTkEntry, ctk.CTkTextbox, ctk.CTkComboBox,
           ctk.CTkCheckBox, tk.Listbox)
    def visit(widget):
        if isinstance(widget,types):
            state=widget._textbox.cget('state') if isinstance(widget,ctk.CTkTextbox) else widget.cget('state')
            saved.append((widget,state))
            widget.configure(state='disabled')
            return
        for child in widget.winfo_children():visit(child)
    app._locked_inputs=saved
    try:
        for page in app.pages.values():visit(page)
    except Exception:
        unlock(app)
        raise


def unlock(app):
    for widget,state in getattr(app,'_locked_inputs',[]):
        if widget.winfo_exists():widget.configure(state=state)
    app._locked_inputs=[]
