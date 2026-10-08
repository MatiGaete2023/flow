"""Work page: compact controls and an editor that always has usable space."""
import tkinter as tk
from . import ui
from .widgets import CaseDetailPanel, ScrollPane
from .record_view import RecordForm


def build(app):
    page=app.pages['Trabajo']
    app.resume_frame=ui.Frame(page)
    app.resume_text=tk.StringVar()
    ui.Label(app.resume_frame,textvariable=app.resume_text).pack(side='left',fill='x',expand=True)
    ui.Button(app.resume_frame,text='Continuar',command=lambda:app._guard(app._continue_resume)).pack(side='right')
    ui.Button(app.resume_frame,text='Elegir otro archivo',command=app._choose_other_resume).pack(side='right',padx=5)
    bar=ui.Frame(page);bar.pack(fill='x',pady=(0,4))
    app.mode=tk.StringVar(value='ESPERA');app.file=tk.StringVar();app.sheet=tk.StringVar()
    app.folder=tk.StringVar(value=str(app.cfg.directory/'salidas'))
    mode=ui.Combobox(bar,textvariable=app.mode,values=['ESPERA','CUMPLIMIENTO','INFORMES'],state='readonly',width=18)
    mode.pack(side='left');mode.bind('<<ComboboxSelected>>',app._mode_changed)
    app.work_top=top=ui.Frame(page)
    def toggle_file():
        if top.winfo_manager():top.pack_forget()
        else:top.pack(fill='x',after=bar,pady=5)
    ui.Button(bar,text='Archivo / opciones',width=145,command=toggle_file).pack(side='left',padx=5)
    ui.Button(bar,text='Actualizar Excel · F5',width=145,fg_color='transparent',border_width=1,command=lambda:app._guard(app._refresh)).pack(side='left')
    app.work_primary_button=ui.Button(bar,text='Procesar Excel',command=lambda:app._guard(app._export_current if app.work else app._process))
    app.work_primary_button.pack(side='right')
    app._field(top,'Archivo',app.file,0)
    ui.Button(top,text='Buscar',command=app._choose).grid(row=0,column=2)
    ui.Label(top,text='Hoja · vacía = automática').grid(row=1,column=0,sticky='w')
    app.sheet_box=ui.Combobox(top,textvariable=app.sheet,state='readonly');app.sheet_box.grid(row=1,column=1,sticky='ew',padx=6)
    app._field(top,'Carpeta de salida',app.folder,2)
    ui.Button(top,text='Cambiar',command=lambda:app._select_folder(app.folder)).grid(row=2,column=2)
    extra=ui.Frame(top);extra.grid(row=3,column=0,columnspan=3,sticky='ew',pady=5)
    for index,(label,command) in enumerate([
        ('Procesar archivo',app._process),('Cargar revisada',app._external),('Localizar copia',app._locate),
        ('Abrir Excel',lambda:app._open(app._require_work().output)),('Abrir carpeta',app._open_output_folder),('Resumen',app._export_statistics)]):
        ui.Button(extra,text=label,width=110,fg_color='transparent',border_width=1,command=lambda fn=command:app._guard(fn)).grid(row=index//3,column=index%3,padx=3,pady=3,sticky='ew')
    for index,(label,command) in enumerate([('Descargar y analizar',app._download_joint),('Comprobar columnas',app._inspect_input),('Abrir original SITFA',app._open_sitfa_source)]):
        ui.Button(extra,text=label,width=110,fg_color='transparent',border_width=1,command=lambda fn=command:app._guard(fn)).grid(row=2,column=index,padx=3,pady=3,sticky='ew')
    extra.grid_columnconfigure((0,1,2),weight=1)
    app.summary=tk.StringVar();ui.Label(page,textvariable=app.summary,anchor='w').pack(fill='x')
    app.work_split=split=ui.Panedwindow(page,orient='vertical');split.pack(fill='both',expand=True)
    app.work_table=table=ui.Frame(split);edit=ui.Frame(split);split.add(table,weight=2);split.add(edit,weight=3)
    filters=ui.Frame(table);filters.pack(fill='x',pady=4)
    app.work_search=tk.StringVar();app.work_filter=tk.StringVar(value='Todos');app.incident_text=tk.StringVar(value='0 incidencias')
    app.work_search_entry=ui.Entry(filters,textvariable=app.work_search,width=22,placeholder_text='Buscar persona, RIT o programa');app.work_search_entry.pack(side='left',fill='x',expand=True,padx=(0,6))
    box=ui.Combobox(filters,textvariable=app.work_filter,values=['Todos','Con aviso','Con resolución','Excluidos','Sin incidencias'],state='readonly',width=17)
    box.pack(side='left',padx=4);box.bind('<<ComboboxSelected>>',app._apply_work_filter)
    def schedule(*_):
        if getattr(app,'_search_after',None):app.after_cancel(app._search_after)
        app._search_after=app.after(180,app._apply_work_filter)
    app.work_search.trace_add('write',schedule)
    ui.Button(filters,text='Aviso anterior',width=95,command=lambda:app._next_incident(-1)).pack(side='left',padx=3)
    ui.Button(filters,text='Siguiente aviso',width=110,command=lambda:app._next_incident(1)).pack(side='left')
    app.records=app._tree(table,('Estado','RIT','Nombre','Tribunal','Programa','Observación'))
    app.records.configure(height=5)
    columns=app.cfg.data.get('vista',{}).get('columnas_trabajo')
    if columns and all(c in app.records['columns'] for c in columns):app.records.configure(displaycolumns=columns)
    for name,width in [('Estado',110),('RIT',85),('Nombre',180),('Tribunal',95),('Programa',160),('Observación',220)]:
        app.records.column(name,width=width,minwidth=65)
    app.records.tag_configure('excluded',background='#665220',foreground='#fff2cc')
    app.records.tag_configure('warning',background='#653b29',foreground='#ffe2cd')
    app.records.tag_configure('signed',background='#243c56',foreground='#ddebf7')
    app.records.bind('<<TreeviewSelect>>',lambda event:None if app.busy else app._guard(app._detail))
    footer=ui.Frame(edit);footer.pack(side='bottom',fill='x',pady=(5,0))
    def maximize():
        if str(table) in split.panes():split.forget(table)
        else:
            split.insert(0,table,weight=2)
            app.after(50,lambda:split.sashpos(0,max(120,int(split.winfo_height()*.42))))
    ui.Button(footer,text='Ampliar / reducir editor',width=160,fg_color='transparent',border_width=1,command=maximize).pack(side='left')
    ui.Button(footer,text='Guardar local · Ctrl+S',width=150,command=lambda:app._guard(app._save_session)).pack(side='right')
    ui.Button(footer,text='Aplicar · Ctrl+Enter',width=135,fg_color='transparent',border_width=1,command=lambda:app._guard(app.record_form.save)).pack(side='right',padx=5)
    app.record_tabs=tabs=ui.Notebook(edit);tabs.pack(fill='both',expand=True)
    observation=ui.Frame(tabs);tabs.add(observation,text='Observación')
    app.detail=tk.StringVar();ui.Label(observation,textvariable=app.detail,wraplength=700,text_color=ui.MUTED).pack(fill='x')
    app.observation_editor=ui.Textbox(observation,wrap='word',height=7,font=('Segoe UI',11),undo=True)
    app.observation_editor.pack(fill='both',expand=True)
    app.observation_editor.bind('<Control-Return>',lambda event:(app._guard(app.record_form.save),'break')[1])
    app.record_form=RecordForm(app,tabs)
    detail=ScrollPane(tabs);tabs.add(detail,text='Detalle')
    from .work_tools import show_quality,explain,preferences
    for label,fn in [('Calidad del archivo',show_quality),('Explicar sugerencia',explain),('Preferencias de vista',preferences)]:
        ui.Button(detail.body,text=label,command=lambda action=fn:app._guard(lambda:action(app))).pack(anchor='w',pady=3)
    app.work_case_detail=CaseDetailPanel(detail.body);app.work_case_detail.pack(fill='x',pady=4)
    def fit(event=None):
        height=split.winfo_height()
        if len(split.panes())<2 or height<250:return
        ratio=getattr(app,'work_sash_ratio',.40)
        split.sashpos(0,max(100,min(height-180,int(height*ratio))))
    def remember(event=None):
        if len(split.panes())==2 and split.winfo_height()>250:
            app.work_sash_ratio=split.sashpos(0)/split.winfo_height()
    split.bind('<Configure>',fit)
    split.bind('<Map>',lambda event:app.after_idle(fit))
    split.bind('<ButtonRelease-1>',remember)
