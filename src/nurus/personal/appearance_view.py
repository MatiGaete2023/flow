"""Persisted visual preferences without changing domain configuration."""
from copy import deepcopy
import tkinter as tk
from . import ui


def build(app, notebook):
    page=ui.Frame(notebook);notebook.add(page,text='Apariencia')
    ui.Label(page,text='Comodidad y presentación',font=('Segoe UI',19,'bold')).pack(anchor='w',padx=18,pady=18)
    app.theme_choice=tk.StringVar(value=app.cfg.data.get('vista',{}).get('tema','Sistema'))
    ui.Label(page,text='Tema de la pantalla').pack(anchor='w',padx=18)
    choice=ui.Combobox(page,textvariable=app.theme_choice,values=list(ui.THEMES),state='readonly')
    choice.pack(anchor='w',padx=18,pady=8)
    app.html_choice=tk.BooleanVar(value=app.cfg.data.get('correo_html',True))
    ui.Checkbutton(page,text='Presentación HTML sencilla en borradores',variable=app.html_choice).pack(anchor='w',padx=18,pady=12)
    ui.Label(page,text='La firma de Outlook se conserva. Ctrl+K permite buscar acciones.\nWindows aplica el escalado de pantalla; puedes cambiar el tema cuando lo necesites.',justify='left',wraplength=650).pack(anchor='w',padx=18,pady=12)
    def save():
        cfg=deepcopy(app.cfg.data)
        cfg['vista']={**cfg.get('vista',{}),'tema':app.theme_choice.get()}
        cfg['correo_html']=app.html_choice.get();app.cfg.save(cfg)
        if app.work:app.work.config=deepcopy(cfg)
        ui.set_theme(app,app.theme_choice.get());app.status.set('Preferencias visuales guardadas.')
    ui.Button(page,text='Aplicar y guardar',command=lambda:app._guard(save)).pack(anchor='w',padx=18,pady=10)
    app.apply_appearance=save
