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


class CaseDetailPanel(ctk.CTkFrame):
    """Panel reutilizable y compacto de contexto de causa."""
    def __init__(self,parent,title='Detalle de causa',wraplength=560,**kwargs):
        super().__init__(parent,fg_color=ttk.FIELD,corner_radius=8,border_width=1,border_color='#394451',**kwargs)
        self.grid_columnconfigure(0,weight=1)
        ctk.CTkLabel(self,text=title,font=('Segoe UI',13,'bold'),anchor='w').grid(row=0,column=0,sticky='ew',padx=10,pady=(7,2))
        self.variable=tk.StringVar(value='Selecciona un registro.')
        self.body_label=ctk.CTkLabel(self,textvariable=self.variable,anchor='w',justify='left',wraplength=wraplength,text_color=ttk.MUTED)
        self.body_label.grid(row=1,column=0,sticky='ew',padx=10,pady=(0,8))
        self.bind('<Configure>',lambda event:self.body_label.configure(wraplength=max(120,event.width-30)),add='+')

    def set_text(self,text):
        self.variable.set(str(text or 'Selecciona un registro.'))


class Tooltip:
    """Tooltip mínimo; no altera el foco ni la acción del control."""
    def __init__(self,widget,text):
        self.widget=widget;self.text=text;self.tip=None
        widget.bind('<Enter>',self._show,add='+');widget.bind('<Leave>',self._hide,add='+')

    def _show(self,event=None):
        if self.tip or not self.text:return
        try:
            x=self.widget.winfo_rootx()+16;y=self.widget.winfo_rooty()+self.widget.winfo_height()+4
            self.tip=tk.Toplevel(self.widget);self.tip.wm_overrideredirect(True);self.tip.wm_geometry(f'+{x}+{y}')
            tk.Label(self.tip,text=self.text,justify='left',background='#fffbe6',foreground='#202020',relief='solid',borderwidth=1,font=('Segoe UI',9)).pack(ipadx=6,ipady=4)
        except tk.TclError:
            self.tip=None

    def _hide(self,event=None):
        if self.tip:
            try:self.tip.destroy()
            except tk.TclError:pass
            self.tip=None
