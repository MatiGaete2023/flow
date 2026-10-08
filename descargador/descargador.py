"""Ventana local: el Chrome habitual hace las consultas mediante una extension."""
import calendar
from datetime import date
import os
from pathlib import Path
import queue
import subprocess
import threading
import tkinter as tk
from tkinter import filedialog,messagebox,simpledialog,ttk

import motor as m
from lotes import Batch,BatchRunner,Dates,MODALITIES
from puente import Bridge
from perfiles import SCREENS
from pausa import PauseGate
from preferencias import Preferences,date_preset,pending_plan

ALL_TRIBUNALS='Todos los tribunales disponibles'
ALL_MODALITIES='Todas las modalidades'
TAB_OPTIONS={'Espera':('Espera',),'Cumplimiento':('Cumplimiento',),
             'Espera y Cumplimiento':('Espera','Cumplimiento'),'Informes':('Informes',),'Egresados':('Egresados',)}


class TribunalPicker(tk.Toplevel):
    """Las casillas se guardan solo al pulsar Aplicar; cerrar conserva la selección."""
    def __init__(self,root,catalog,selected,apply):
        super().__init__(root);self.title('Elegir tribunales');self.geometry('560x520');self.minsize(430,300)
        self.transient(root);self.grab_set();self.apply_selection=apply
        self.choices={key:tk.BooleanVar(self,value=key in selected) for key in catalog}
        self.labels=catalog;self.search=tk.StringVar();self.selection_count=tk.StringVar()
        ttk.Label(self,text='Marca los tribunales que quieres incluir.',padding=12).pack(anchor='w')
        ttk.Entry(self,textvariable=self.search).pack(fill='x',padx=12)
        ttk.Label(self,textvariable=self.selection_count).pack(anchor='w',padx=12)
        controls=ttk.Frame(self,padding=(12,0));controls.pack(fill='x')
        ttk.Button(controls,text='Marcar todos',command=lambda:self.set_all(True)).pack(side='left',padx=(0,8))
        ttk.Button(controls,text='Desmarcar todos',command=lambda:self.set_all(False)).pack(side='left')
        ttk.Button(controls,text='Marcar visibles',command=lambda:self.set_visible(True)).pack(side='left',padx=8)
        container=ttk.Frame(self,padding=12);container.pack(fill='both',expand=True)
        canvas=tk.Canvas(container,highlightthickness=0,bg='#f3f6fa');canvas.pack(side='left',fill='both',expand=True)
        bar=ttk.Scrollbar(container,orient='vertical',command=canvas.yview);bar.pack(side='right',fill='y');canvas.configure(yscrollcommand=bar.set)
        items=ttk.Frame(canvas);window=canvas.create_window(0,0,anchor='nw',window=items)
        canvas.bind('<Configure>',lambda e:canvas.itemconfigure(window,width=e.width))
        items.bind('<Configure>',lambda e:canvas.configure(scrollregion=canvas.bbox('all')))
        self.checks={}
        for key,label in catalog.items():
            self.checks[key]=ttk.Checkbutton(items,text=label,variable=self.choices[key]);self.checks[key].pack(anchor='w',fill='x',pady=3)
            self.choices[key].trace_add('write',lambda *_:self.filter_choices())
        self.search.trace_add('write',lambda *_:self.filter_choices());self.filter_choices()
        canvas.bind('<MouseWheel>',lambda e:canvas.yview_scroll(-int(e.delta/120),'units'))
        actions=ttk.Frame(self,padding=12);actions.pack(fill='x')
        ttk.Button(actions,text='Cancelar',command=self.destroy).pack(side='right')
        ttk.Button(actions,text='Aplicar',command=self.apply).pack(side='right',padx=8)
    def set_all(self,value):
        for choice in self.choices.values():choice.set(value)
    def set_visible(self,value):
        for key,choice in self.choices.items():
            if m.normalized(self.search.get()) in m.normalized(self.labels[key]):choice.set(value)
    def filter_choices(self):
        query=m.normalized(self.search.get())
        for key,widget in self.checks.items():
            widget.pack_forget()
            if query in m.normalized(self.labels[key]):widget.pack(anchor='w',fill='x',pady=3)
        self.selection_count.set(f'{sum(v.get() for v in self.choices.values())} tribunales seleccionados (se conservan los ocultos)')
    def apply(self):
        self.apply_selection(tuple(k for k,v in self.choices.items() if v.get()));self.destroy()


def open_chrome(url):
    candidates=[Path(os.environ.get(env,''))/'Google/Chrome/Application/chrome.exe'
                for env in ('ProgramFiles','ProgramFiles(x86)','LOCALAPPDATA')]
    chrome=next((p for p in candidates if p.is_file()),None)
    if chrome is None:raise m.PocError('No se encontró Chrome instalado.')
    # Chrome resuelve su perfil habitual. No se usa depuracion ni una copia del perfil.
    subprocess.Popen([str(chrome),url],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)


def pick_date(root,variable):
    try:chosen=date(*reversed([int(x) for x in variable.get().split('/')]))
    except Exception:chosen=m.now().date()
    popup=tk.Toplevel(root);popup.title('Elegir fecha');popup.resizable(False,False)
    popup.transient(root);popup.grab_set()
    current=[chosen.year,chosen.month]
    names=['Enero','Febrero','Marzo','Abril','Mayo','Junio','Julio','Agosto','Septiembre','Octubre','Noviembre','Diciembre']
    heading=ttk.Frame(popup,padding=10);heading.pack(fill='x')
    label=ttk.Label(heading,anchor='center',width=24)
    grid=ttk.Frame(popup,padding=10);grid.pack()
    def choose(day):variable.set(day.strftime('%d/%m/%Y'));popup.destroy()
    def render():
        label.configure(text=f'{names[current[1]-1]} {current[0]}')
        for w in grid.winfo_children():w.destroy()
        for column,day in enumerate(['Lu','Ma','Mi','Ju','Vi','Sa','Do']):ttk.Label(grid,text=day,anchor='center').grid(row=0,column=column)
        for row,week in enumerate(calendar.Calendar().monthdatescalendar(*current),1):
            for col,day in enumerate(week):
                b=ttk.Button(grid,text=str(day.day),width=4,command=lambda day=day:choose(day))
                b.grid(row=row,column=col,padx=1,pady=2)
                if day.month!=current[1]:b.configure(state='disabled')
    def shift(delta):
        number=current[0]*12+current[1]-1+delta
        current[:]=[number//12,number%12+1];render()
    ttk.Button(heading,text='‹',width=3,command=lambda:shift(-1)).pack(side='left')
    label.pack(side='left');ttk.Button(heading,text='›',width=3,command=lambda:shift(1)).pack(side='right')
    render()


class Application:
    def __init__(self,root,start_worker=True,bridge_factory=Bridge,preferences_path=None):
        self.root=root;self.events=queue.Queue();self.commands=queue.Queue();self.cancel=PauseGate()
        self.bridge_factory=bridge_factory;self.bridge=bridge_factory(self.emit)
        self.catalog=None;self.connected=False;self.busy=False;self.quitting=False;self.last_folder=None
        self.prefs=Preferences(preferences_path,memory=not start_worker and preferences_path is None)
        self.recovery=None;self.context=tk.StringVar(value='Sin lote activo');self.query_index=0;self.query_total=0
        self.result_windows=[]
        self.signature_days=tk.IntVar(value=self.prefs.data.get('dias_firmas',60));self.csmp_reply=None;self.csmp_mode=None
        self.bitacoras_reply=None;self.bitacoras_id=None
        self.tribunal=tk.StringVar(value=ALL_TRIBUNALS);self.modality=tk.StringVar(value=ALL_MODALITIES)
        self.selected_tribunals=None if self.prefs.data['tribunales'] is None else tuple(self.prefs.data['tribunales']);self.screen=tk.StringVar(value='Seguimiento')
        self.order_state=tk.StringVar();self.month=tk.StringVar();self.year=tk.StringVar()
        self.tab=tk.StringVar(value='Espera y Cumplimiento');self.report=tk.StringVar()
        self.from_date=tk.StringVar(value=m.now().strftime('%d/%m/%Y'));self.to_date=tk.StringVar(value=self.from_date.get())
        self.use_dates=tk.BooleanVar(value=False);self.keep_filters=tk.BooleanVar(value=False)
        from preferencias import data_directory
        default_folder=data_directory()/'descargas' if getattr(__import__('sys'),'frozen',False) else Path(__file__).resolve().parent/'descargas'
        self.dest=tk.StringVar(value=self.prefs.data['destino'] or str(default_folder))
        self.code=tk.StringVar(value=self.bridge.connection_code)
        self.status=tk.StringVar(value='Abre tu Chrome, entra a Seguimiento y conecta la extensión local.')
        self.preview=tk.StringVar(value='Carga las opciones de tu sesión para preparar el lote.')
        self.count=tk.StringVar(value='Sin descargas en este lote')
        root.title('Descargador SITFA · 2.7.2 integral · CSMP y bitácoras')
        menu=tk.Menu(root);root.configure(menu=menu)
        self.theme=tk.StringVar(value=self.prefs.data.get('tema','Sistema'))
        appearance=tk.Menu(menu,tearoff=False);menu.add_cascade(label='Apariencia',menu=appearance)
        for choice in ('Claro','Oscuro','Sistema'):appearance.add_radiobutton(label=choice,variable=self.theme,value=choice,command=self.apply_theme)
        tools=tk.Menu(menu,tearoff=False);menu.add_cascade(label='Más opciones',menu=tools)
        for label,command in [('Guardar consulta favorita…',self.save_favorite),('Elegir favorita…',self.choose_favorite),
                              ('Resultados e historial',self.show_results),('Limpiar pendientes preparados',self.clear_pending),
                              ('Configurar ejecutable CSMP…',self.configure_csmp)]:tools.add_command(label=label,command=command)
        tools.add_command(label='Retomar descarga conjunta…',command=self.resume_joint)
        self.sound=tk.BooleanVar(value=self.prefs.data['aviso_sonoro'])
        tools.add_checkbutton(label='Aviso sonoro al terminar',variable=self.sound,command=self.save_preferences)
        root.geometry(f'1000x{min(900,max(600,root.winfo_screenheight()-120))}');root.minsize(880,600)
        root.protocol('WM_DELETE_WINDOW',self.close_app)
        style=ttk.Style(root);style.theme_use('clam')
        style.configure('TFrame',background='#f3f6fa');style.configure('TLabel',background='#f3f6fa',foreground='#20324d',font=('Segoe UI',10))
        style.configure('Title.TLabel',font=('Segoe UI',22,'bold'));style.configure('Section.TLabel',font=('Segoe UI',12,'bold'))
        style.configure('TButton',padding=(10,7),font=('Segoe UI',10));style.configure('Treeview',rowheight=26,font=('Segoe UI',10))
        container=ttk.Frame(root);container.pack(fill='both',expand=True)
        self.canvas=tk.Canvas(container,bg='#f3f6fa',highlightthickness=0);self.canvas.pack(side='left',fill='both',expand=True)
        scroll=ttk.Scrollbar(container,orient='vertical',command=self.canvas.yview);scroll.pack(side='right',fill='y');self.canvas.configure(yscrollcommand=scroll.set)
        body=ttk.Frame(self.canvas,padding=20);window=self.canvas.create_window(0,0,anchor='nw',window=body)
        self.canvas.bind('<Configure>',lambda e:self.canvas.itemconfigure(window,width=e.width,height=max(e.height,body.winfo_reqheight())))
        body.bind('<Configure>',lambda e:self.canvas.configure(scrollregion=self.canvas.bbox('all')))
        root.bind('<MouseWheel>',lambda e:self.canvas.yview_scroll(-int(e.delta/120),'units') if e.widget.winfo_class() not in ('TCombobox','Treeview') else None)
        body.columnconfigure(0,weight=1)
        ttk.Label(body,text='Descargas de SITFA',style='Title.TLabel').grid(row=0,column=0,sticky='w')
        ttk.Label(body,text='Tribunales a elección · una o todas las modalidades · todas las páginas').grid(row=1,column=0,sticky='w',pady=(3,15))
        connect=ttk.LabelFrame(body,text='1. Tu Chrome habitual',padding=12);connect.grid(row=2,column=0,sticky='ew',pady=(0,12));connect.columnconfigure(0,weight=1)
        row=ttk.Frame(connect);row.grid(row=0,column=0,sticky='w')
        self.open_button=ttk.Button(row,text='Abrir mi Chrome en SITFA',command=lambda:self.enqueue('open'));self.open_button.pack(side='left',padx=(0,8))
        self.setup_button=ttk.Button(row,text='Instalar extensión…',command=self.setup);self.setup_button.pack(side='left',padx=(0,8))
        self.load_button=ttk.Button(row,text='Cargar opciones',command=lambda:self.enqueue('catalog'));self.load_button.pack(side='left',padx=(0,8))
        self.disconnect_button=ttk.Button(row,text='Desconectar',command=lambda:self.enqueue('disconnect'));self.disconnect_button.pack(side='left')
        code_row=ttk.Frame(connect);code_row.grid(row=1,column=0,sticky='ew',pady=(9,3));code_row.columnconfigure(1,weight=1)
        ttk.Label(code_row,text='Código de conexión:').grid(row=0,column=0,padx=(0,8))
        self.code_entry=ttk.Entry(code_row,textvariable=self.code,state='readonly');self.code_entry.grid(row=0,column=1,sticky='ew',padx=(0,8))
        self.copy_button=ttk.Button(code_row,text='Copiar código',command=self.copy_code);self.copy_button.grid(row=0,column=2)
        ttk.Label(connect,text='En Seguimiento, pulsa el icono de la extensión y pega este código. Mantén abierta su pestaña.',wraplength=910).grid(row=2,column=0,sticky='w',pady=(5,0))
        flow=ttk.Frame(connect);flow.grid(row=3,column=0,sticky='ew',pady=(10,0));flow.columnconfigure(1,weight=1)
        ttk.Label(flow,text='Consulta:').grid(row=0,column=0,padx=(0,8))
        self.screen_combo=ttk.Combobox(flow,textvariable=self.screen,values=tuple(SCREENS),state='disabled')
        self.screen_combo.grid(row=0,column=1,sticky='ew');self.screen_combo.bind('<<ComboboxSelected>>',lambda e:self.change_screen())
        query=ttk.LabelFrame(body,text='2. Elige qué descargar',padding=12);query.grid(row=3,column=0,sticky='ew',pady=(0,12))
        query.columnconfigure(0,weight=3);query.columnconfigure(1,weight=2)
        for column,label in enumerate(['Tribunal','Modalidad']):ttk.Label(query,text=label).grid(row=0,column=column,sticky='w')
        tribunal_row=ttk.Frame(query);tribunal_row.grid(row=1,column=0,sticky='ew',padx=(0,12),pady=(3,8));tribunal_row.columnconfigure(0,weight=1)
        ttk.Label(tribunal_row,textvariable=self.tribunal,wraplength=350).grid(row=0,column=0,sticky='w')
        self.trib_button=ttk.Button(tribunal_row,text='Elegir tribunales…',command=self.choose_tribunals);self.trib_button.grid(row=0,column=1,padx=(8,0))
        self.mod_combo=ttk.Combobox(query,textvariable=self.modality,width=28);self.mod_combo.grid(row=1,column=1,sticky='ew',pady=(3,8))
        ttk.Label(query,text='Registros').grid(row=2,column=0,sticky='w');ttk.Label(query,text='Filtro de cumplimiento / tipo de informe').grid(row=2,column=1,sticky='w')
        self.tab_combo=ttk.Combobox(query,textvariable=self.tab,values=tuple(TAB_OPTIONS));self.tab_combo.grid(row=3,column=0,sticky='ew',padx=(0,12),pady=(3,8))
        self.report_combo=ttk.Combobox(query,textvariable=self.report);self.report_combo.grid(row=3,column=1,sticky='ew',pady=(3,8))
        extras=ttk.Frame(query);extras.grid(row=4,column=0,columnspan=2,sticky='ew',pady=(0,8))
        self.state_combo=ttk.Combobox(extras,textvariable=self.order_state,width=20)
        self.month_combo=ttk.Combobox(extras,textvariable=self.month,width=16)
        self.year_combo=ttk.Combobox(extras,textvariable=self.year,width=8)
        for label,combo in [('Estado de órdenes',self.state_combo),('Mes',self.month_combo),('Año',self.year_combo)]:
            ttk.Label(extras,text=label).pack(side='left',padx=(0,6));combo.pack(side='left',padx=(0,16))
        dates=ttk.Frame(query);dates.grid(row=5,column=0,columnspan=2,sticky='ew')
        self.date_check=ttk.Checkbutton(dates,text='Filtrar por fecha',variable=self.use_dates,command=self.refresh);self.date_check.pack(side='left',padx=(0,15))
        self.date_widgets=[]
        for label,value in [('Desde',self.from_date),('Hasta',self.to_date)]:
            ttk.Label(dates,text=label).pack(side='left',padx=(0,6));entry=ttk.Entry(dates,textvariable=value,width=13);entry.pack(side='left',padx=(0,5))
            button=ttk.Button(dates,text='Calendario',command=lambda value=value:pick_date(root,value));button.pack(side='left',padx=(0,15));self.date_widgets.extend((entry,button))
        self.filter_check=ttk.Checkbutton(query,text='Conservar los otros filtros preparados en SITFA (por ejemplo, litigante o centro)',variable=self.keep_filters,command=self.refresh)
        self.filter_check.grid(row=6,column=0,columnspan=2,sticky='w',pady=(9,4))
        quick=ttk.Frame(query);quick.grid(row=9,column=0,columnspan=2,sticky='w',pady=6)
        for label in ('Este mes','Mes anterior','Últimos 30 días'):
            ttk.Button(quick,text=label,command=lambda label=label:self.quick_dates(label)).pack(side='left',padx=(0,6))
        ttk.Label(query,text='Para una persona concreta, prepara sus filtros en SITFA y marca Conservar los otros filtros. Los calendarios usan mes y año.',wraplength=910).grid(row=7,column=0,columnspan=2,sticky='w')
        ttk.Label(query,textvariable=self.preview,style='Section.TLabel').grid(row=8,column=0,columnspan=2,sticky='w',pady=(9,0))
        destination=ttk.Frame(body);destination.grid(row=4,column=0,sticky='ew',pady=(0,12));destination.columnconfigure(0,weight=1)
        ttk.Label(destination,text='Carpeta de descarga · se creará una subcarpeta con fecha y hora').grid(row=0,column=0,sticky='w')
        ttk.Entry(destination,textvariable=self.dest,state='readonly').grid(row=1,column=0,sticky='ew',padx=(0,8),pady=(4,0))
        self.browse_button=ttk.Button(destination,text='Cambiar…',command=self.browse);self.browse_button.grid(row=1,column=1,pady=(4,0))
        results=ttk.Frame(body);results.grid(row=5,column=0,sticky='ew');results.columnconfigure(0,weight=1)
        actions=ttk.Frame(results);actions.grid(row=0,column=0,sticky='ew');actions.columnconfigure(0,weight=1)
        self.download_button=ttk.Button(actions,text='Descargar el lote completo',command=self.start_download);self.download_button.grid(row=0,column=0,sticky='w')
        self.cancel_button=ttk.Button(actions,text='Cancelar',command=self.request_cancel);self.cancel_button.grid(row=0,column=1,padx=8)
        self.folder_button=ttk.Button(actions,text='Abrir carpeta',command=self.open_folder);self.folder_button.grid(row=0,column=2)
        ttk.Button(actions,text='Resultados…',command=self.show_results).grid(row=0,column=3,padx=8)
        self.joint_button=ttk.Button(actions,text='Descarga conjunta CSMP',command=self.start_joint);self.joint_button.grid(row=1,column=0,sticky='w',pady=5)
        ttk.Label(actions,text='Principal completa + informes del mes actual y siguiente').grid(row=1,column=1,columnspan=3,sticky='w',padx=8)
        self.diary_button=ttk.Button(actions,text='Leer bitácoras del lote',command=self.start_diary)
        self.diary_button.grid(row=2,column=0,sticky='w',pady=5)
        self.diary_retry_button=ttk.Button(actions,text='Reintentar lecturas fallidas…',command=self.resume_diary)
        self.diary_retry_button.grid(row=2,column=1,columnspan=2,sticky='w',padx=8)
        self.pause_button=ttk.Button(actions,text='Pausar / continuar',command=self.toggle_pause)
        self.pause_button.grid(row=3,column=0,sticky='w',pady=5)
        self.progress=ttk.Progressbar(results);self.progress.grid(row=1,column=0,sticky='ew',pady=10)
        table=ttk.Frame(results);table.grid(row=2,column=0,sticky='ew');table.columnconfigure(0,weight=1)
        self.tree=ttk.Treeview(table,columns=('page','rows','file'),show='headings',height=5)
        for col,label,width in [('page','Página',80),('rows','Registros',85),('file','Archivo guardado',690)]:self.tree.heading(col,text=label);self.tree.column(col,width=width,stretch=col=='file')
        self.tree.grid(row=0,column=0,sticky='ew');bar=ttk.Scrollbar(table,orient='vertical',command=self.tree.yview);bar.grid(row=0,column=1,sticky='ns');self.tree.configure(yscrollcommand=bar.set)
        self.tree.bind('<Double-1>',self.open_selected)
        ttk.Label(results,textvariable=self.count).grid(row=3,column=0,sticky='w',pady=(6,0))
        ttk.Label(results,textvariable=self.context).grid(row=4,column=0,sticky='w')
        ttk.Label(body,textvariable=self.status,wraplength=910).grid(row=6,column=0,sticky='ew',pady=(12,0))
        for combo in (self.mod_combo,self.tab_combo,self.report_combo,self.state_combo,self.month_combo,self.year_combo):combo.bind('<<ComboboxSelected>>',lambda e:self.refresh())
        self.refresh();self.apply_theme(save=False)
        if start_worker:
            threading.Thread(target=self.worker,daemon=True).start();root.after(100,self.poll)

    def apply_theme(self,save=True):
        from apariencia import apply
        choice=self.theme.get()
        if choice not in ('Claro','Oscuro','Sistema'):choice='Sistema';self.theme.set(choice)
        apply(self.root,choice)
        if save:self.prefs.data['tema']=choice;self.prefs.save()

    def toggle_pause(self):
        if self.cancel.paused:self.cancel.resume();self.status.set('Descarga reanudada.');self.pause_button.configure(text='Pausar / continuar')
        else:self.cancel.pause();self.status.set('Pausa solicitada: se completará la página en curso.');self.pause_button.configure(text='Continuar descarga')

    def emit(self,name,value):self.events.put((name,value))

    def save_preferences(self):
        self.prefs.data.update(destino=self.dest.get(),tribunales=None if self.selected_tribunals is None else list(self.selected_tribunals),aviso_sonoro=self.sound.get())
        try:
            days=self.signature_days.get()
            if 60<=days<=730:self.prefs.data['dias_firmas']=days
        except tk.TclError:pass
        try:self.prefs.save()
        except OSError:self.status.set('E_CONFIG: no se pudo guardar la configuración local.')
    def configure_csmp(self):
        executable=filedialog.askopenfilename(parent=self.root,title='CSMP experimental: ejecutable o Python de su entorno',filetypes=[('Ejecutables','*.exe')])
        if executable:self.prefs.data['csmp']=executable;self.save_preferences()
    def quick_dates(self,label):
        if self.busy:return
        first,last=date_preset(label,m.now().date())
        if SCREENS[self.screen.get()].startswith('calendario_'):
            if not self.catalog:return
            month=str(first.month);year=str(first.year)
            if month not in self.catalog.get('meses',{}) or year not in self.catalog.get('anios',{}):
                self.status.set('Ese mes/año no está disponible en SITFA.');return
            self.month.set(self.catalog['meses'][month]);self.year.set(self.catalog['anios'][year])
        else:self.from_date.set(first.strftime('%d/%m/%Y'));self.to_date.set(last.strftime('%d/%m/%Y'));self.use_dates.set(True)
        self.clear_pending()
    def save_favorite(self):
        if self.busy:return
        try:
            batch=self.batch();batch.plan(self.catalog)
            name=simpledialog.askstring('Favorito','Nombre del perfil (fechas recalculadas al aplicarlo):',parent=self.root)
            if name:
                offset=0
                if batch.screen.startswith('calendario_'):
                    from datetime import date
                    today=m.now().date();offset=int(batch.year)*12+int(batch.month)-today.year*12-today.month
                    if offset not in (0,1):raise m.PocError('Los perfiles automáticos admiten el mes actual o siguiente.')
                self.prefs.favorite(name,batch,calendar_offset=offset);self.status.set('Perfil guardado; recalcula el período al aplicarlo.')
        except (m.PocError,OSError):self.status.set('Carga las opciones y revisa el alcance; no se guardan filtros privados.')
    def apply_batch(self,batch):
        self.select_tribunals(batch.tribunals);self.modality.set(MODALITIES.get(batch.modalities[0],ALL_MODALITIES) if len(batch.modalities)==1 else ALL_MODALITIES)
        self.tab.set(next((k for k,v in TAB_OPTIONS.items() if v==batch.tabs),'Espera y Cumplimiento'))
        self.order_state.set(self.catalog.get('estados',{}).get(batch.state,''))
        self.month.set(self.catalog.get('meses',{}).get(batch.month,''));self.year.set(self.catalog.get('anios',{}).get(batch.year,''))
        choices=self.catalog.get('informes',{}) if batch.tabs==('Informes',) else self.catalog.get('medidas',{})
        self.report.set(choices.get(batch.report,'Todas las medidas'));self.keep_filters.set(False)
        self.use_dates.set(batch.dates is not None)
        if batch.dates:self.from_date.set(batch.dates.start);self.to_date.set(batch.dates.end)
        self.refresh()
    def choose_favorite(self):
        if self.busy or not self.catalog:return
        popup=tk.Toplevel(self.root);popup.title('Consultas favoritas');popup.transient(self.root)
        choice=tk.StringVar();box=ttk.Combobox(popup,textvariable=choice,values=list(self.prefs.data['favoritos']),state='readonly');box.pack(padx=16,pady=16)
        def apply():
            try:
                name=choice.get();value=self.prefs.data['favoritos'][name]
                if value['screen']!=SCREENS[self.screen.get()]:raise m.PocError('Selecciona la pantalla del favorito y carga sus opciones antes de aplicarlo.')
                batch,missing=self.prefs.restore(name,self.catalog,today=m.now().date());self.clear_pending();self.apply_batch(batch)
                self.status.set('Favorito aplicado; revisa el alcance y las fechas.'+(' Opciones retiradas: '+', '.join(missing) if missing else ''));popup.destroy()
            except (m.PocError,KeyError,ValueError) as exc:messagebox.showerror('Favorito',str(exc),parent=popup)
        ttk.Button(popup,text='Aplicar',command=apply).pack(pady=8)
    def show_results(self):
        if self.busy:self.status.set('Espera a que termine la operación para trabajar con resultados.');return
        if self.last_folder and (self.last_folder/'flujo.json').is_file():
            from ventana_flujo import JointWindow
            JointWindow(self);return
        from ventana_resultados import ResultsWindow
        ResultsWindow(self)
    def open_selected(self,event=None):
        selected=self.tree.selection()
        if selected and self.last_folder:
            from ventana_resultados import open_local
            from resultados import inside
            try:open_local(inside(self.last_folder,self.tree.item(selected[0],'values')[2]))
            except (m.PocError,OSError) as exc:self.status.set(str(exc))
    def clear_pending(self):self.recovery=None;self.download_button.configure(text='Descargar el lote completo');self.refresh()
    def prepare_pending(self,folder):
        from resultados import read_json
        from perfiles import Selection,selection_profile
        planned=pending_plan(folder)
        if not planned:raise m.PocError('No hay consultas pendientes.')
        summary=read_json(Path(folder)/'resumen.json');screen=planned[0]['screen']
        if not self.catalog or SCREENS[self.screen.get()]!=screen:raise m.PocError('Selecciona la pantalla del lote y carga sus opciones primero.')
        keys=Selection.__dataclass_fields__;selections=[Selection(**{k:v for k,v in s.items() if k in keys}) for s in planned]
        for s in selections:selection_profile(s,self.catalog)
        s=selections[0];dates=summary.get('filtro_fecha')
        batch=Batch((s.tribunal,),(s.modality,) if s.modality else (), (s.tab,) if s.tab else (),report=s.report or '0',
                    dates=Dates(dates['desde'],dates['hasta']) if dates else None,screen=screen,state=s.state or '',month=s.month,year=s.year)
        self.recovery=(batch,selections);self.download_button.configure(text='Descargar consultas pendientes')
        self.status.set(f'{len(selections)} consultas pendientes preparadas. Se descargarán desde página 1 en un lote nuevo.');self.refresh()

    def change_screen(self):
        self.catalog=None;self.refresh();self.status.set('Pulsa Cargar opciones para abrir y preparar la consulta elegida en SITFA.')

    def select_tribunals(self,ids):
        if not self.catalog or set(ids)-set(self.catalog['tribunales']):raise m.PocError('La selección contiene tribunales no disponibles.')
        self.selected_tribunals=tuple(k for k in self.catalog['tribunales'] if k in ids)
        self.tribunal.set('Ningún tribunal marcado' if not ids else (self.catalog['tribunales'][ids[0]] if len(ids)==1 else f'{len(ids)} tribunales marcados'))
        self.refresh()

    def choose_tribunals(self):
        if self.catalog and not self.busy:TribunalPicker(self.root,self.catalog['tribunales'],self.selected_tribunals or (),self.select_tribunals)

    def setup(self):
        directory=Path(__file__).resolve().parent/'extension'
        self.root.clipboard_clear();self.root.clipboard_append(str(directory));self.root.update()
        open_chrome('chrome://extensions/')
        messagebox.showinfo('Instalación única en tu Chrome',
            'Activa Modo de desarrollador y pulsa Cargar descomprimida.\n\nSelecciona la carpeta extension de esta herramienta. Su ruta está copiada al portapapeles.\n\nDespués entra a Seguimiento y pulsa el icono de SITFA · Descargador local.',parent=self.root)

    def copy_code(self):
        self.root.clipboard_clear();self.root.clipboard_append(self.code.get());self.root.update()
        self.status.set('Código copiado. Pégalo en la pestaña de conexión de la extensión.')

    def batch(self):
        if not self.catalog:raise m.PocError('Carga las opciones de Seguimiento primero.')
        tribunals=self.selected_tribunals or ()
        screen=SCREENS[self.screen.get()]
        def code(key,variable):return next((k for k,v in self.catalog.get(key,{}).items() if v==variable.get()),None)
        if screen!='seguimiento':
            return Batch(tribunals,(),(),dates=Dates(self.from_date.get(),self.to_date.get()) if screen=='carga' or self.use_dates.get() and screen=='litigantes' else None,
                keep_filters=self.keep_filters.get() if screen=='litigantes' else False,screen=screen,state=code('estados',self.order_state) or '',
                month=code('meses',self.month),year=code('anios',self.year))
        modalities=tuple(k for k in self.catalog['modalidades'] if k in MODALITIES) if self.modality.get()==ALL_MODALITIES else tuple(k for k,v in MODALITIES.items() if v==self.modality.get() and k in self.catalog['modalidades'])
        report_options=self.catalog['informes'] if self.tab.get()=='Informes' else self.catalog.get('medidas',{})
        reports=[k for k,v in report_options.items() if v==self.report.get()]
        return Batch(tribunals,modalities,TAB_OPTIONS[self.tab.get()],reports[0] if reports else ('1' if self.tab.get()=='Informes' else '0'),
                     Dates(self.from_date.get(),self.to_date.get()) if self.use_dates.get() else None,self.keep_filters.get())

    def refresh(self):
        idle=not self.busy and not self.quitting;ready=idle and self.connected and self.catalog is not None
        screen=SCREENS[self.screen.get()];followup=screen=='seguimiento';calendar_screen=screen.startswith('calendario_')
        self.screen_combo.configure(state='readonly' if idle and self.connected else 'disabled')
        for button,enabled in [(self.open_button,idle),(self.setup_button,idle),(self.load_button,idle and self.connected),
             (self.disconnect_button,idle and self.connected),(self.copy_button,idle),(self.browse_button,idle),
             (self.download_button,ready and bool(self.selected_tribunals)),(self.joint_button,ready and followup and bool(self.selected_tribunals)),
             (self.diary_button,ready and followup and bool(self.selected_tribunals)),(self.diary_retry_button,ready and followup),
             (self.trib_button,ready),(self.cancel_button,self.busy and getattr(self,'operation',None) in ('download','joint','resume_joint','diary') and not self.quitting),
             (self.pause_button,self.busy and getattr(self,'operation',None) in ('download','joint','resume_joint','diary') and not self.quitting),
             (self.folder_button,self.last_folder is not None and not self.quitting),
             (self.date_check,ready and not calendar_screen),(self.filter_check,ready and not calendar_screen)]:button.configure(state='normal' if enabled else 'disabled')
        for combo in (self.mod_combo,self.tab_combo):combo.configure(state='readonly' if ready and followup else 'disabled')
        self.state_combo.configure(state='readonly' if ready and screen=='litigantes' else 'disabled')
        for combo in (self.month_combo,self.year_combo):combo.configure(state='readonly' if ready and calendar_screen else 'disabled')
        if self.catalog and followup:
            values=list(self.catalog['informes'].values()) if self.tab.get()=='Informes' else ['Todas las medidas',*self.catalog.get('medidas',{}).values()]
            self.report_combo.configure(values=values)
            if self.report.get() not in values and values:self.report.set(values[0])
        self.report_combo.configure(state='readonly' if ready and followup and self.tab.get()!='Espera' else 'disabled')
        for widget in self.date_widgets:widget.configure(state='normal' if ready and not calendar_screen and (screen=='carga' or self.use_dates.get()) else 'disabled')
        if self.catalog:
            try:self.preview.set(f'{len(self.recovery[1] if self.recovery else self.batch().plan(self.catalog))} consultas · '+('pendientes del lote anterior (desde página 1)' if self.recovery else 'una descarga mensual por tribunal' if calendar_screen else 'todas sus páginas en orden'))
            except Exception:self.preview.set('Revisa las opciones y el rango de fechas.')
        else:self.preview.set('Carga las opciones de tu sesión para preparar el lote.')
        if self.recovery:
            for widget in (self.screen_combo,self.mod_combo,self.tab_combo,self.state_combo,self.month_combo,self.year_combo,self.report_combo,self.trib_button,
                           self.date_check,self.filter_check,*self.date_widgets):widget.configure(state='disabled')

    def enqueue(self,command,arg=None):
        if self.busy or self.quitting:return
        if command=='catalog':arg=SCREENS[self.screen.get()]
        self.busy=True;self.operation=command;self.refresh();self.commands.put((command,arg))

    def browse(self):
        directory=filedialog.askdirectory(parent=self.root,title='Carpeta de descarga')
        if directory:self.dest.set(directory);self.save_preferences()

    def start_download(self):
        try:
            batch=self.recovery[0] if self.recovery else self.batch();batch.plan(self.catalog)
            destination=Path(self.dest.get()).resolve();destination.mkdir(parents=True,exist_ok=True)
        except (m.PocError,OSError,KeyError) as exc:
            messagebox.showerror('Revisa el lote',str(exc) if isinstance(exc,m.PocError) else 'Seleccione una carpeta local accesible.',parent=self.root);return
        self.cancel.clear();self.last_folder=None;self.tree.delete(*self.tree.get_children())
        self.progress.configure(value=0,maximum=1);self.count.set('Preparando el lote…');self.status.set('Descargando en tu sesión Chrome…')
        self.save_preferences();self.enqueue('download',(batch,destination,self.recovery[1] if self.recovery else None))

    def request_cancel(self):self.cancel.set();self.status.set('Se detendrá antes de la siguiente consulta.');self.cancel_button.configure(state='disabled')

    def start_diary(self,resume=None):
        try:
            from lectura_bitacoras import saved_batch,load
            batch=saved_batch(load(Path(resume)/'bitacoras.json')) if resume else self.batch()
            if batch.screen!='seguimiento':raise m.PocError('Selecciona Seguimiento para leer bitácoras.')
            batch.plan(self.catalog)
            destination=Path(self.dest.get()).resolve();destination.mkdir(parents=True,exist_ok=True)
            if self.bitacoras_reply and resume and not Path(resume).resolve().is_relative_to(destination):
                raise m.PocError('Para devolver a CSMP, recupera un lote de la carpeta de esta solicitud.')
        except (m.PocError,OSError,ValueError,KeyError) as exc:
            messagebox.showerror('Revisa el lote de bitácoras',str(exc),parent=self.root);return
        self.cancel.clear();self.tree.delete(*self.tree.get_children());self.last_folder=None
        self.status.set('Leyendo y copiando bitácoras. Las fallas quedan en el control del lote.');self.save_preferences()
        self.enqueue('diary',(batch,destination,resume))

    def resume_diary(self):
        folder=filedialog.askdirectory(parent=self.root,title='Carpeta del lote de bitácoras a reintentar')
        if folder:self.start_diary(folder)

    def start_joint(self):
        try:
            batch=self.batch()
            if batch.screen!='seguimiento' or len(batch.tabs)!=1 or batch.tabs[0] not in ('Espera','Cumplimiento'):
                raise m.PocError('Selecciona Espera o Cumplimiento para la descarga conjunta.')
            self.check_csmp_mode(batch.tabs[0])
            from flujo_csmp import plan
            plan(batch.tribunals,batch.modalities,batch.tabs[0],m.now().date(),self.signature_days.get())
            destination=Path(self.dest.get()).resolve();destination.mkdir(parents=True,exist_ok=True)
        except (m.PocError,ValueError,tk.TclError,OSError) as exc:
            messagebox.showerror('Revisa el flujo conjunto',str(exc),parent=self.root);return
        self.cancel.clear();self.tree.delete(*self.tree.get_children());self.last_folder=None
        self.status.set('Descargando principal completa e informes del mes actual y siguiente. Al terminar se entrega el libro a CSMP.')
        self.save_preferences()
        self.enqueue('joint',(batch.tribunals,batch.modalities,batch.tabs[0],destination,self.signature_days.get()))

    def check_csmp_mode(self,mode):
        if self.csmp_reply and self.csmp_mode and mode.upper()!=self.csmp_mode:
            raise m.PocError('CSMP solicitó '+self.csmp_mode+'. Selecciona ese registro o inicia una nueva descarga desde CSMP con el modo que necesitas.')

    def resume_joint(self,folder=None):
        if self.busy or not self.connected:self.status.set('Conecta Chrome y termina la operación actual antes de retomar un flujo.');return
        folder=folder or filedialog.askdirectory(parent=self.root,title='Elegir carpeta de descarga conjunta')
        if not folder:return
        try:
            from resultados import read_json
            data=read_json(Path(folder)/'flujo.json');s=data['seleccion']
            self.check_csmp_mode(s['modo'])
            from flujo_csmp import plan
            cutoff=date.fromisoformat(s['corte']);plan(tuple(s['tribunales']),tuple(s['modalidades']),s['modo'],cutoff,s['dias_firmas'])
        except (m.PocError,ValueError,KeyError,OSError) as exc:self.status.set('No se puede recuperar el flujo: '+str(exc));return
        self.cancel.clear();self.tree.delete(*self.tree.get_children())
        self.enqueue('resume_joint',(s,Path(folder)))
    def open_folder(self):
        if self.last_folder and self.last_folder.exists():
            from ventana_resultados import open_local
            open_local(self.last_folder)
    def close_app(self):
        if self.quitting:return
        if any(window.winfo_exists() and window.busy for window in self.result_windows):
            self.status.set('Espera a que termine la escritura de resultados antes de cerrar.');return
        self.quitting=True;self.cancel.set();self.status.set('Terminando la página en curso y cerrando la conexión local…');self.refresh();self.commands.put(('exit',None))

    def worker(self):
        while True:
            command,arg=self.commands.get()
            try:
                if command in ('download','joint','resume_joint'):
                    from contrato_integral import verify_extension
                    verify_extension(self.bridge)
                if command=='open':open_chrome(m.START);self.emit('status','SITFA abierto en tu Chrome. Entra a Seguimiento y conecta su extensión.')
                elif command=='catalog':self.emit('catalog',self.bridge.call('catalog',{'screen':arg}))
                elif command=='download':BatchRunner(self.bridge,self.catalog,self.emit,self.cancel).run(*arg)
                elif command=='diary':
                    from lectura_bitacoras import DiaryRunner
                    DiaryRunner(self.bridge,self.catalog,self.emit,self.cancel).run(*arg)
                elif command=='joint':
                    from flujo_csmp import JointRunner
                    JointRunner(self.bridge,self.emit,self.cancel).run(*arg)
                elif command=='resume_joint':
                    from flujo_csmp import JointRunner
                    selection,folder=arg
                    JointRunner(self.bridge,self.emit,self.cancel).run(tuple(selection['tribunales']),tuple(selection['modalidades']),selection['modo'],folder.parent,
                        selection['dias_firmas'],today=date.fromisoformat(selection['corte']),resume=folder)
                elif command in ('disconnect','exit'):
                    if self.bridge.connected:
                        try:self.bridge.call('unlock',timeout=8)
                        except Exception:pass
                    self.bridge.close()
                    if command=='exit':self.emit('exit',None);return
                    self.bridge=self.bridge_factory(self.emit);self.emit('disconnected',self.bridge.connection_code)
            except m.PocError as exc:self.emit('error','E_'+command.upper()+': '+str(exc))
            except Exception:self.emit('error','E_'+command.upper()+': No se pudo completar la operación. Revisa la conexión, SITFA y la carpeta. Se conservan los archivos ya verificados.')
            finally:self.emit('idle',None)

    def process_event(self,name,value):
        if name=='diary_done':
            self.last_folder=Path(value['folder']);self.count.set(f"{value['leidas']} lecturas verificadas · {value['fallidas']} filas fallidas o sin vínculo · {value.get('consultas_fallidas',0)} consultas fallidas")
            self.context.set('Bitácoras · '+value['estado'])
            self.status.set(('Copias guardadas. ' if value['leidas'] else 'No se guardaron copias de bitácoras. ')+
                (value.get('detalle') or 'CSMP recibe el control del lote. Revisa Lecturas y Consultas en el Excel.'))
            if self.bitacoras_reply:
                from lectura_bitacoras import write
                write(Path(self.bitacoras_reply),{**value,'id':self.bitacoras_id})
            self.refresh();return True
        if name=='diary_progress':
            self.tree.insert('','end',values=(value['pagina'],value['registros'],value['rit']+' · '+value['estado']))
            self.tree.yview_moveto(1);self.count.set(f'{len(self.tree.get_children())} filas revisadas en el lote')
            self.status.set('Bitácora '+value['rit']+' · '+value['estado']);return True
        if name=='joint_done':
            self.catalog=None;self.last_folder=Path(value['folder']);self.tree.delete(*self.tree.get_children())
            self.tree.insert('','end',values=('Final','',Path(value['archivo']).name))
            self.status.set('Libro preparado para CSMP: principal completa e informes de ambos meses. No se consultaron resoluciones firmadas.')
            self.count.set('Descarga conjunta terminada');self.context.set(value['estado'])
            if self.csmp_reply:
                from flujo_csmp import write_state
                import hashlib
                path=Path(value['archivo'])
                write_state(Path(self.csmp_reply),{'archivo':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'modo':value['modo']})
            return True
        if name=='connected':self.connected=True;self.catalog=None;self.status.set(value)
        elif name=='disconnected':self.connected=False;self.catalog=None;self.code.set(value);self.status.set('Desconectado. Tu Chrome sigue abierto; usa el nuevo código para reconectar.')
        elif name=='catalog':
            self.catalog=value
            self.screen.set(next(k for k,v in SCREENS.items() if v==value.get('pantalla','seguimiento')))
            self.select_tribunals(tuple(value['tribunales']) if self.selected_tribunals is None else tuple(k for k in self.selected_tribunals if k in value['tribunales']))
            for key,combo,variable,selection_key in [('estados',self.state_combo,self.order_state,'estado'),('meses',self.month_combo,self.month,'mes'),('anios',self.year_combo,self.year,'anio')]:
                choices=value.get(key,{});combo.configure(values=list(choices.values()))
                if variable.get() not in choices.values():
                    current=m.now().date()
                    preferred=str(current.month) if key=='meses' else str(current.year) if key=='anios' else value.get('seleccion',{}).get(selection_key)
                    variable.set(choices.get(preferred,choices.get(value.get('seleccion',{}).get(selection_key),next(iter(choices.values()),''))))
            self.mod_combo.configure(values=[ALL_MODALITIES,*[MODALITIES[k] for k in value['modalidades'] if k in MODALITIES]])
            self.report_combo.configure(values=list(value['informes'].values()))
            if value['informes']:self.report.set(next(iter(value['informes'].values())))
            if self.modality.get() not in self.mod_combo['values']:self.modality.set(ALL_MODALITIES)
            self.status.set('Opciones cargadas. Elige el alcance del lote y, si corresponde, las fechas.')
        elif name=='folder':self.last_folder=Path(value)
        elif name=='query':
            self.query_index=value['index'];self.query_total=value['total'];self.context.set(f'Consulta {self.query_index}/{self.query_total} · {value["label"]}')
            self.status.set('Consultando SITFA…');self.count.set('Consultando…');self.progress.configure(value=0,maximum=1)
        elif name=='total':self.progress.configure(maximum=value,value=0)
        elif name=='progress':
            self.tree.insert('','end',values=(f'{value["pagina"]}/{value["total"]}',value['registros'],value['archivo']))
            self.tree.yview_moveto(1);self.progress.configure(value=value['pagina']);self.count.set(f'{len(self.tree.get_children())} archivos guardados en el lote')
            self.status.set(f'Consulta {self.query_index}/{self.query_total} · Página {value["pagina"]}/{value["total"]} verificada · {value["registros"]} registros')
        elif name=='evidence':
            self.tree.insert('','end',values=('PDF',0,value['archivo']));self.tree.yview_moveto(1)
            self.count.set(f'{len(self.tree.get_children())} archivos guardados en el lote')
        elif name=='done':
            self.status.set(f'Lote completo: {value["queries"]} consultas, {value["files"]} Excel, {value.get("pdfs",0)} PDF y {value["records"]} registros.');self.count.set('Completado. Puedes abrir Resultados o iniciar otro lote.')
            self.context.set(self.last_folder.name if self.last_folder else 'Lote completo')
            self.recovery=None;self.download_button.configure(text='Descargar el lote completo')
            if self.sound.get():self.root.bell()
        elif name in ('status','error','warning'):
            self.status.set(value)
            if name=='error' and self.sound.get():self.root.bell()
        elif name=='idle':self.busy=False;self.operation=None
        elif name=='exit':
            try:
                if self.root.clipboard_get()==self.code.get():self.root.clipboard_clear()
            except tk.TclError:pass
            self.root.destroy();return False
        self.refresh();return True

    def poll(self):
        while True:
            try:event=self.events.get_nowait()
            except queue.Empty:break
            if not self.process_event(*event):return
        self.root.after(100,self.poll)


def main(argv=None):
    import argparse
    parser=argparse.ArgumentParser(description='Descargador SITFA · prototipo integral')
    parser.add_argument('--csmp-reply',type=Path);parser.add_argument('--modo',choices=('ESPERA','CUMPLIMIENTO'))
    parser.add_argument('--destino',type=Path)
    parser.add_argument('--bitacoras-reply',type=Path);parser.add_argument('--bitacoras-id')
    parser.add_argument('--verificar-paquete',type=Path,help=argparse.SUPPRESS)
    parser.add_argument('--capacidades-integrales',type=Path,help=argparse.SUPPRESS)
    parser.add_argument('--generar-lote-prueba',type=Path,help=argparse.SUPPRESS)
    args=parser.parse_args(argv)
    if args.capacidades_integrales:
        import json
        from contrato_integral import capabilities
        path=args.capacidades_integrales.resolve();path.parent.mkdir(parents=True,exist_ok=True)
        path.write_text(json.dumps(capabilities(),ensure_ascii=False),encoding='utf-8');return
    if args.generar_lote_prueba:
        import json
        from prueba_bitacoras_paquete import produce
        path=args.generar_lote_prueba.resolve();path.mkdir(parents=True,exist_ok=True)
        result=produce(path)
        (path/'prueba.json').write_text(json.dumps(result,ensure_ascii=False),encoding='utf-8');return
    if args.verificar_paquete:
        from verificar_paquete import check
        try:check(args.verificar_paquete)
        except Exception as exc:
            import json,traceback
            destination=args.verificar_paquete.resolve()
            destination.parent.mkdir(parents=True,exist_ok=True)
            destination.write_text(json.dumps({'ok':False,'tipo':type(exc).__name__,
                'ubicacion':[{'modulo':Path(t.filename).name,'linea':t.lineno}
                             for t in traceback.extract_tb(exc.__traceback__)]},indent=2),encoding='utf-8')
            raise SystemExit(1)
        return
    root=tk.Tk();app=Application(root)
    app.csmp_reply=args.csmp_reply
    app.csmp_mode=args.modo
    if bool(args.bitacoras_reply)!=bool(args.bitacoras_id):parser.error('La lectura de bitácoras requiere respuesta e identificador.')
    app.bitacoras_reply=args.bitacoras_reply;app.bitacoras_id=args.bitacoras_id
    if args.modo:app.tab.set('Espera' if args.modo=='ESPERA' else 'Cumplimiento')
    if args.destino:app.dest.set(str(args.destino.resolve()))
    app.refresh();root.mainloop()


if __name__=='__main__':main()
