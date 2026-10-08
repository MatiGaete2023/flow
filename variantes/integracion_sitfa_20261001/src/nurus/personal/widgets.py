"""Controles pequeños para el escritorio Windows del Asistente."""
import tkinter as tk
import customtkinter as ctk
from . import ui as ttk
from collections import Counter


class ScrollPane(ctk.CTkFrame):
    """Panel CTk desplazable con rueda, incluido sobre sus controles hijos."""
    def __init__(self,parent,**kwargs):
        super().__init__(parent,fg_color=ttk.PANEL,**kwargs)
        self.body=ctk.CTkScrollableFrame(self,fg_color=ttk.PANEL)
        self.body.pack(fill='both',expand=True)



class NamedChoice(ttk.Combobox):
    """Muestra nombres editables conservando IDs estables para reglas y plantillas."""
    def __init__(self, parent, *, keyvariable, names, **kwargs):
        self.keyvariable = keyvariable
        self.names = names
        self.label = tk.StringVar(parent)
        super().__init__(parent, textvariable=self.label, **kwargs)
        self.keyvariable.trace_add('write', self._sync_label)
        super().bind('<<ComboboxSelected>>', self._selected)
        self.set_keys(list(names()))

    def bind(self, sequence=None, func=None, add=None):
        # El llamador recibe el ID actualizado antes de cargar el formulario.
        return super().bind(sequence, func, '+' if sequence == '<<ComboboxSelected>>' else add)

    def set_keys(self, keys):
        names = self.names()
        titles = [names.get(key, key) for key in keys]
        counts = Counter(titles)
        self.keys = list(keys)
        self.labels = [title if counts[title] == 1 else f'{title} [{key}]' for key, title in zip(keys, titles)]
        super().configure(values=self.labels)
        self._sync_label()

    def configure(self, cnf=None, **kwargs):
        if 'values' in kwargs:
            self.set_keys(kwargs.pop('values'))
        return super().configure(cnf, **kwargs)

    def _sync_label(self, *args):
        key = self.keyvariable.get()
        self.label.set(self.labels[self.keys.index(key)] if key in self.keys else key)

    def _selected(self, event=None):
        index = self.current()
        if index >= 0:
            self.keyvariable.set(self.keys[index])
