"""Template lifecycle, previews and contact undo controls."""
from copy import deepcopy
import tkinter as tk
from tkinter import simpledialog
import customtkinter as ctk
from . import ui


def template_actions(app,parent):
    bar=ui.Frame(parent);bar.grid(row=8,column=0,columnspan=2,sticky='ew',pady=6)
    def refresh(key):
        app._editing_tpl_key=None
        app.tpl_key.set(key);app._load_tpl();app._save_tpl(notify=False)
    def duplicate():
        from .personalization import duplicate_template
        app._save_tpl(notify=False)
        name=simpledialog.askstring('Duplicar plantilla','Nombre de la copia:',parent=app)
        if name is None:return
        config,key=duplicate_template(app.cfg.data,app.tpl_key.get(),name)
        app.cfg.save(config);refresh(key)
    def archive():
        from .personalization import archive_template
        app._save_tpl(notify=False)
        key=app.tpl_key.get();current=app.cfg.data['correos']['plantillas'][key]
        app.cfg.save(archive_template(app.cfg.data,key,not current.get('archivada',False)))
        refresh(key)
        app.status.set('Plantilla activada.' if current.get('archivada') else 'Plantilla archivada. Puedes volver a activarla aquí.')
    def restore():
        from .personalization import restore_template
        key=app.tpl_key.get()
        app.cfg.save(restore_template(app.cfg.data,key));refresh(key)
    def preview():
        from .template_validation import render
        from .config import VARIABLES
        values={key:'Ejemplo '+key.lower().replace('_',' ') for key in VARIABLES}
        values.update(FECHA='28/09/2026',PERIODO='septiembre de 2026',TABLA_REGISTROS='X-123 · Persona de ejemplo')
        text=render(app.tpl_subject.get(),values)+'\n\n'+render(app.tpl_body.get('1.0','end-1c'),values)
        window=ctk.CTkToplevel(app);window.title('Vista previa · datos ficticios');window.geometry('720x500');window.transient(app)
        box=ui.Textbox(window,wrap='word');box.pack(fill='both',expand=True,padx=12,pady=12);box.insert('1.0',text);box.configure(state='disabled')
    for label,command in [('Duplicar',duplicate),('Archivar / activar',archive),('Restaurar anterior',restore),('Vista previa',preview)]:
        ui.Button(bar,text=label,width=130,command=lambda fn=command:app._guard(fn)).pack(side='left',padx=3)


def contact_actions(app,parent):
    search=tk.StringVar();app.contact_search=search
    bar=ui.Frame(parent);bar.grid(row=6,column=0,columnspan=2,sticky='ew',pady=6)
    ui.Entry(bar,textvariable=search,placeholder_text='Buscar contacto o alias',width=32).pack(side='left',fill='x',expand=True)
    search.trace_add('write',lambda *_:app._list_contacts())
    def new():
        app._editing_contact_name=None
        for variable in (app.contact_name,app.contact_mail,app.contact_alias):variable.set('')
        app.contact_list.selection_clear(0,'end')
    def undo():
        previous=getattr(app,'_contact_undo',None)
        if previous is None:raise ValueError('No hay una modificación de contactos que deshacer.')
        config=deepcopy(app.cfg.data)
        for key in ('contactos','aliases'):config[key]=deepcopy(previous[key])
        config['correos']['tribunales']=deepcopy(previous['correos']['tribunales'])
        app.cfg.save(config);app._contact_undo=None;new();app._list_contacts()
        app.status.set('Última modificación de contactos deshecha.')
    ui.Button(bar,text='Nuevo',width=80,command=new).pack(side='left',padx=4)
    ui.Button(bar,text='Deshacer',width=90,command=lambda:app._guard(undo)).pack(side='left')
