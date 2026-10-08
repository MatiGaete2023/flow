"""Current products, next action and preflight in one place."""
import tkinter as tk
from datetime import date
from tkinter import filedialog
from . import ui
from .widgets import ScrollPane
from .current_tools import deliver,export_diagnosis
from .product_checks import check_products,current_drafts


def build(app):
    pane=ScrollPane(app.pages['Resultados']);pane.pack(fill='both',expand=True)
    app.results_scrollpane=pane;page=pane.body
    header=ui.Frame(page);header.pack(fill='x',pady=8)
    ui.Label(header,text='Resultados del trabajo',font=('Segoe UI',20,'bold')).pack(side='left')
    ui.Button(header,text='Actualizar',width=100,command=lambda:app._guard(lambda:refresh())).pack(side='right')
    app.results_summary=tk.StringVar(value='Carga un Excel para comenzar.')
    ui.Label(page,textvariable=app.results_summary,anchor='w').pack(fill='x',pady=6)
    app.results_empty=ui.Frame(page);app.results_empty.pack(fill='x',pady=8)
    ui.Label(app.results_empty,text='Aquí aparecerán los productos y lo que falta para terminarlos.').pack(anchor='w',padx=12,pady=8)
    ui.Button(app.results_empty,text='Elegir Excel · Ctrl+O',command=app._choose).pack(anchor='w',padx=12,pady=(0,10))
    split=ui.Panedwindow(page,orient='horizontal');split.pack(fill='both',expand=True,pady=8)
    table=ui.Frame(split);detail=ui.Frame(split);split.add(table,weight=3);split.add(detail,weight=2)
    tasks=app._tree(table,('Producto','Tipo','Estado'));tasks.configure(height=9)
    tasks.column('Producto',width=220);tasks.column('Tipo',width=85);tasks.column('Estado',width=185)
    app.results_tree=tasks;app.results_checks={}
    app.results_detail=tk.StringVar(value='Selecciona un producto para revisar el siguiente paso.')
    label=ui.Label(detail,textvariable=app.results_detail,anchor='nw',justify='left',wraplength=270);label.pack(fill='both',expand=True,padx=12,pady=12)
    label.bind('<Configure>',lambda e:label.configure(wraplength=max(150,e.width-16)),add='+')
    app.results_action=ui.Button(detail,text='Abrir producto',command=lambda:app._guard(open_selected));app.results_action.pack(fill='x',padx=12,pady=12)
    actions=ui.Frame(page);actions.pack(fill='x',pady=8)
    ui.Button(actions,text='Preparar ZIP',command=lambda:app._guard(lambda:deliver(app))).pack(side='left')
    ui.Button(actions,text='Abrir carpeta',command=lambda:app._guard(app._open_output_folder)).pack(side='left',padx=8)
    advanced=ui.Frame(page)
    def more():
        if advanced.winfo_manager():advanced.pack_forget()
        else:advanced.pack(fill='x',pady=6)
    app.results_more=more
    ui.Button(actions,text='Más acciones',command=more).pack(side='right')
    # Consulta autónoma, sin activar la escritura de observaciones.
    section=ui.Frame(advanced);section.pack(fill='x',pady=(8,4))
    ui.Label(section,text='Bitácoras RUS · solo lectura',font=('Segoe UI',13,'bold')).pack(anchor='w')
    from .bitacoras import earliest,validate_period
    start_date=tk.StringVar(value=earliest(date.today()).isoformat())
    end_date=tk.StringVar(value=date.today().isoformat())
    dates=ui.Frame(section);dates.pack(fill='x',pady=4)
    for caption,value in (('Desde AAAA-MM-DD',start_date),('Hasta AAAA-MM-DD',end_date)):
        ui.Label(dates,text=caption).pack(side='left',padx=(0,4))
        ui.Entry(dates,textvariable=value,width=116).pack(side='left',padx=(0,8))
    def period():
        start,end=date.fromisoformat(start_date.get()),date.fromisoformat(end_date.get())
        validate_period(start,end)
        return start,end
    def consult():
        from .bitacora_link import start
        a,b=period()
        start(app,a,b)
    def recover():
        from .bitacora_link import recover as recover_request
        recover_request(app)
    def open_har():
        from .bitacora_har import import_har,export_har_audit
        a,b=period()
        selected=filedialog.askopenfilenames(parent=app,title='Capturas HAR de bitácoras',filetypes=[('HAR','*.har')])
        if not selected:return
        target=filedialog.asksaveasfilename(parent=app,defaultextension='.xlsx',initialfile='Bitacoras_RUS.xlsx')
        if not target:return
        app._run('Exportando bitácoras HAR…',lambda:export_har_audit(import_har(selected),target,a,b),
                 lambda value:app.status.set('Excel de bitácoras: '+str(value)))
    def open_lot():
        from .bitacora_lote import export_lote
        a,b=period()
        selected=filedialog.askopenfilename(parent=app,title='Control de bitácoras',filetypes=[('Lote JSON','bitacoras.json')])
        if not selected:return
        target=filedialog.asksaveasfilename(parent=app,defaultextension='.xlsx',initialfile='Bitacoras_RUS.xlsx')
        if not target:return
        app._run('Exportando lote de bitácoras…',lambda:export_lote(selected,target,a,b),
                 lambda value:app.status.set('Excel de bitácoras: '+str(value)))
    buttons=ui.Frame(section);buttons.pack(fill='x',pady=4)
    app.results_bitacora_live_button=ui.Button(buttons,text='Consultar bitácoras en RUS',command=lambda:app._guard(consult))
    app.results_bitacora_live_button.pack(side='left',padx=(0,5))
    ui.Button(buttons,text='Recuperar consulta',command=lambda:app._guard(recover)).pack(side='left',padx=(0,5))
    ui.Button(buttons,text='Importar HAR',command=lambda:app._guard(open_har)).pack(side='left',padx=(0,5))
    ui.Button(buttons,text='Abrir lote',command=lambda:app._guard(open_lot)).pack(side='left')
    ui.Button(advanced,text='Diagnóstico de instalación',command=lambda:app._guard(lambda:export_diagnosis(app))).pack(anchor='w',pady=4)
    from .download_link import resume
    app.results_download_button=ui.Button(advanced,text='Recuperar descarga interrumpida',command=lambda:app._guard(lambda:resume(app)));app.results_download_button.pack(anchor='w',pady=4)
    from .work_tools import show_deadlines
    ui.Button(advanced,text='Vencimientos / egreso proyectado',command=lambda:app._guard(lambda:show_deadlines(app))).pack(anchor='w',pady=4)
    def detail_selected(event=None):
        selected=tasks.selection();check=app.results_checks.get(selected[0]) if selected else None
        if not check:app.results_detail.set('Selecciona un producto.');return
        app.results_detail.set(check.label+'\n\n'+check.state+'\n\n'+('\n'.join('- '+x for x in check.issues) if check.issues else 'Producto disponible para revisar.')+('\n\nArchivo: '+check.path if check.path else ''))
        app.results_action.configure(text='Abrir archivo' if check.path else 'Revisar / corregir')
    def refresh(capture=True):
        if capture and app.work:app._capture_observation();app._capture_mail();app._capture_project()
        checks=check_products(app.work,current_drafts(app),app.projects)
        from .table_update import upsert,prune
        old=list(app.results_checks);app.results_checks={c.id:c for c in checks};prune(tasks,old,app.results_checks)
        for c in checks:upsert(tasks,c.id,(c.label,c.kind,c.state))
        issues=sum(bool(c.issues) for c in checks)
        app.results_summary.set(f'{len(checks)} productos · {len(checks)-issues} sin incidencias · {issues} por corregir' if checks else 'Carga un Excel para comenzar.')
        if app.work:app.results_empty.pack_forget()
        elif not app.results_empty.winfo_manager():app.results_empty.pack(fill='x',before=split,pady=8)
        detail_selected()
    def open_selected(event=None):
        selected=tasks.selection();check=app.results_checks.get(selected[0]) if selected else None
        if not check:return
        if check.path:app._open(check.path);return
        if check.kind=='Correo':
            if not any(d.product_id==check.target.product_id for d in app.drafts):
                app.mail_target.set('todos');app._mail_target_changed()
            index=next(i for i,d in enumerate(app.drafts) if d.product_id==check.target.product_id)
            app.mail_list.selection_clear(0,'end');app.mail_list.selection_set(index);app._select_mail();app.tabs.select(app.pages['Correos'])
        elif check.kind=='Word':
            app.project_list.selection_clear(0,'end');app.project_list.selection_set(check.index);app._select_project();app.tabs.select(app.pages['Resoluciones'])
        else:app.tabs.select(app.pages['Trabajo'])
    tasks.bind('<<TreeviewSelect>>',detail_selected);tasks.bind('<Double-1>',lambda e:app._guard(open_selected));tasks.bind('<Return>',lambda e:app._guard(open_selected))
    app.refresh_pending=refresh
    app.open_result=open_selected
    # Update when entering the page, including edits made outside Resultados.
    original_select=app.tabs.select
    def select(page=None):
        result=original_select(page)
        if page is not None and str(page)==str(app.pages['Resultados']) and not app.busy:refresh()
        return result
    app.tabs.select=select
