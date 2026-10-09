"""Barra «Siguiente paso»: muestra el recorrido del trabajo y abre cada página. Solo lee el estado."""
import tkinter as tk
from . import ui
from .product_checks import check_products,current_drafts

STEPS=(('revisar','Revisar','Trabajo'),('exportar','Exportar copia','Trabajo'),('correos','Correos','Correos'),
       ('resoluciones','Resoluciones','Resoluciones'),('resultados','Resultados','Resultados'))


def counts(app):
    """Conteos de cada paso con las mismas fuentes que usa cada página."""
    work=getattr(app,'work',None)
    if not work:return {key:0 for key,_,_ in STEPS}
    checks=check_products(work,current_drafts(app),app.projects)
    return {'revisar':sum(1 for row in work.rows if row.warnings and not row.excluded),
            'exportar':0,
            'correos':sum(1 for c in checks if c.kind=='Correo'),
            'resoluciones':sum(1 for c in checks if c.kind=='Word'),
            'resultados':sum(bool(c.issues) for c in checks)}


def labels(values):
    return {key:f'{number} {title}'+('' if key=='exportar' else f' ({values[key]})')
            for number,(key,title,_) in enumerate(STEPS,1)}


def build(app,parent):
    bar=ui.Frame(parent);bar.pack(side='right',padx=8,pady=2)
    app.next_step_buttons={};app.next_step_text={key:tk.StringVar() for key,_,_ in STEPS}
    for index,(key,_,page) in enumerate(STEPS):
        if index:ui.Label(bar,text='→',text_color=ui.MUTED).pack(side='left',padx=2)
        button=ui.Button(bar,textvariable=app.next_step_text[key],width=40,height=24,fg_color='transparent',border_width=1,text_color=ui.TEXT,
                         command=lambda name=page:app.tabs.select(app.pages[name]))
        button.pack(side='left');app.next_step_buttons[key]=button

    def refresh():
        try:values=counts(app)
        except Exception:return
        for key,text in labels(values).items():app.next_step_text[key].set(text)
    app.next_step_refresh=refresh
    original_select=app.tabs.select
    def select(page=None):
        result=original_select(page)
        if page is not None:refresh()
        return result
    app.tabs.select=select
    original_show=app._show_work
    def show_work(*args,**kwargs):
        result=original_show(*args,**kwargs)
        app.after_idle(refresh)
        return result
    app._show_work=show_work
    refresh()
    return bar
