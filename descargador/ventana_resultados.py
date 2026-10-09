"""Resumen e historial: acciones secundarias fuera del formulario de descarga."""
import os
from pathlib import Path
import queue
import subprocess
import threading
import tkinter as tk
from tkinter import filedialog,messagebox,ttk

import motor as m
from resultados import load_lot,export_book,export_csv,export_ics,export_zip,compare_lots,inside,read_json,tables,compatibility


def open_local(path):
    path=Path(path).resolve()
    if not path.exists():raise m.PocError('El archivo fue movido o ya no existe.')
    if os.name=='nt':os.startfile(str(path))
    else:subprocess.Popen(['xdg-open',str(path)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)


class ResultsWindow(tk.Toplevel):
    def __init__(self,app):
        from csmp_shared import parser
        parser()
        super().__init__(app.root);self.app=app;self.title('Resultados y herramientas · rama experimental')
        self.geometry('1100x780');self.minsize(900,650);self.busy=False;self.events=queue.Queue();self.folders=[];self.lot=None
        self.package_folders=[];self.diary_capture=None;self.loaded_kind=None;self.excluded_diaries=set()
        app.result_windows.append(self)
        self.partial=tk.BooleanVar(value=False);self.status=tk.StringVar(value='Selecciona un lote del historial o abre su carpeta.')
        self.history=ttk.Treeview(self,columns=('fecha','tipo','estado','consultas'),show='headings',height=5,selectmode='extended')
        for col,label in [('fecha','Lote'),('tipo','Tipo'),('estado','Estado'),('consultas','Consultas / bitácoras')]:self.history.heading(col,text=label)
        self.history.pack(fill='x',padx=12,pady=12);self.history.bind('<<TreeviewSelect>>',self.select_history)
        controls=ttk.Frame(self);controls.pack(fill='x',padx=12)
        ttk.Button(controls,text='Añadir descarga o lote anterior…',command=self.choose).pack(side='left')
        ttk.Button(controls,text='Añadir bitácoras anteriores…',command=self.add_diaries).pack(side='left',padx=8)
        ttk.Button(controls,text='Quitar lote del paquete',command=self.remove_package_source).pack(side='left',padx=8)
        ttk.Button(controls,text='Actualizar historial',command=self.reload).pack(side='left',padx=8)
        ttk.Checkbutton(controls,text='Permitir resultados INCOMPLETOS',variable=self.partial,command=self.load).pack(side='left')
        self.detail=ttk.Treeview(self,columns=('consulta','estado','filas','aptitud'),show='headings',height=8)
        for col,label in [('consulta','Consulta'),('estado','Resultado'),('filas','Registros'),('aptitud','Compatibilidad CSMP')]:self.detail.heading(col,text=label)
        self.detail.column('consulta',width=350);self.detail.pack(fill='both',expand=True,padx=12,pady=12)
        self.detail.bind('<Double-1>',lambda _:self.open_source())
        row=ttk.Frame(self);row.pack(fill='x',padx=12)
        for text,action in [('Consolidar XLSX',lambda:self.run(lambda:export_book([self.lot]))),
            ('Libro CSMP (Excel)',self.csmp),('Añadir informes al cruce…',self.cross),
            ('Comparar con otro lote…',self.compare)]:ttk.Button(row,text=text,command=action).pack(side='left',padx=(0,6))
        row=ttk.Frame(self);row.pack(fill='x',padx=12,pady=8)
        for text,action in [('CSV',lambda:self.run(lambda:export_csv(self.lot))),('Calendario ICS',lambda:self.run(lambda:export_ics(self.lot))),
            ('ZIP de originales del lote',lambda:self.run(lambda:export_zip(self.lot))),('Abrir carpeta',lambda:self.safe(lambda:open_local(self.lot.folder))),
            ('Preparar pendientes',self.pending)]:ttk.Button(row,text=text,command=action).pack(side='left',padx=(0,6))
        row=ttk.Frame(self);row.pack(fill='x',padx=12,pady=4)
        ttk.Button(row,text='Excel bitácoras autónomo…',command=self.diary_excel).pack(side='left',padx=(0,6))
        ttk.Button(row,text='Crear paquete para Asistente (ZIP)…',command=self.capture_package).pack(side='left',padx=(0,6))
        ttk.Button(row,text='Ver descarga conjunta…',command=self.show_joint).pack(side='left',padx=(0,6))
        ttk.Button(row,text='Abrir solicitud de antecedentes…',command=self.open_request).pack(side='left',padx=(0,6))
        self.collection=tk.StringVar()
        ttk.Label(self,textvariable=self.collection,wraplength=1050).pack(fill='x',padx=12,pady=4)
        ttk.Label(self,text='Añade las carpetas de hojas y los controles bitacoras.json con sus HTML. El ZIP reúne las descargas añadidas; reutiliza bitácoras solo por tribunal, causa e ingreso. Importa ese ZIP en Trabajo → Importar paquete del Asistente.',wraplength=1050).pack(fill='x',padx=12,pady=4)
        ttk.Label(self,textvariable=self.status,wraplength=1050).pack(fill='x',padx=12,pady=8)
        self.protocol('WM_DELETE_WINDOW',self.close);self.reload();self.after(100,self.poll)
    def close(self):
        if self.busy:self.status.set('Espera a que termine la escritura local.');return
        self.destroy()
    def safe(self,action):
        try:
            if self.busy:raise m.PocError('Hay una operación de resultados en curso.')
            if not self.lot:raise m.PocError('Selecciona un lote primero.')
            action()
        except (m.PocError,OSError,ValueError) as exc:self.status.set(str(exc))
    def reload(self):
        if self.busy:return
        self.history.delete(*self.history.get_children());self.folders=[]
        from archivo_local import discover,remember
        for folder,label,state,count in discover(self.app.dest.get(),self.app.prefs,self.app.last_folder):
            iid=str(len(self.folders));self.folders.append(folder)
            self.history.insert('','end',iid=iid,values=(folder.name,label,state,count))
            if label=='Bitácoras':remember(self.app.prefs,folder)
        self.update_collection()
        if getattr(self,'folder',None) or self.app.last_folder:
            self.folder=getattr(self,'folder',None) or self.app.last_folder;self.load()

    def update_collection(self):
        names='; '.join(p.name for p in self.package_folders)
        self.collection.set(f'Paquete: {len(self.package_folders)} descarga(s) añadida(s): {names} · {len(self.package_diaries())} lote(s) de bitácoras disponibles para este paquete. Las fechas originales se conservan; reutilizar no vuelve a consultar RUS.')

    def package_diaries(self):
        return [p for p in self.app.prefs.data.get('historial_bitacoras',[]) if Path(p).resolve() not in self.excluded_diaries]

    def remove_package_source(self):
        if self.busy:return
        if self.loaded_kind=='Bitácoras':self.excluded_diaries.add((self.folder/'bitacoras.json').resolve())
        if getattr(self,'folder',None) in self.package_folders:self.package_folders.remove(self.folder)
        self.update_collection();self.status.set('Lote retirado de este paquete; sus archivos e historial se conservan. Añadir de nuevo lo incorpora otra vez.')
    def select_history(self,event=None):
        selected=self.history.selection()
        if selected and not self.busy:self.folder=self.folders[int(selected[0])];self.load()
    def choose(self):
        if self.busy:return
        folder=filedialog.askdirectory(parent=self,title='Carpeta del lote')
        if folder:
            from archivo_local import remember,kind
            try:
                self.folder=remember(self.app.prefs,folder)
                self.excluded_diaries.discard((self.folder/'bitacoras.json').resolve())
                if kind(self.folder)[1]!='Bitácoras' and self.folder not in self.package_folders:self.package_folders.append(self.folder)
                self.reload();self.load()
            except (m.PocError,OSError,ValueError) as exc:self.status.set(str(exc))

    def add_diaries(self):
        if self.busy:return
        paths=filedialog.askopenfilenames(parent=self,title='Controles bitacoras.json con sus HTML originales',filetypes=[('Control de bitácoras','bitacoras.json')])
        if not paths:return
        from archivo_local import remember
        try:
            for path in paths:
                remember(self.app.prefs,Path(path).parent);self.excluded_diaries.discard(Path(path).resolve())
            self.folder=Path(paths[0]).parent;self.reload();self.update_collection()
        except (m.PocError,OSError,ValueError) as exc:self.status.set(str(exc))
    def load(self):
        if self.busy or not getattr(self,'folder',None):return
        self.lot=None;self.diary_capture=None
        from archivo_local import kind
        try:_,self.loaded_kind=kind(self.folder)
        except m.PocError as exc:self.status.set(str(exc));return
        self.detail.delete(*self.detail.get_children())
        if self.loaded_kind=='Bitácoras':
            from nurus.capture.diaries import import_lotes
            self.start(lambda:import_lotes(self.folder/'bitacoras.json'),'diary_load');return
        if self.loaded_kind=='Conjunto':
            if self.folder not in self.package_folders:self.package_folders.append(self.folder)
            self.update_collection();self.status.set('Conjunto disponible. Crear paquete reúne sus fases; Ver descarga conjunta permite abrir el libro y recuperar pendientes.');return
        if self.folder not in self.package_folders:self.package_folders.append(self.folder)
        self.update_collection()
        # Capturar variables Tk en el hilo de interfaz.
        partial=self.partial.get();folder=self.folder
        def worker_action():
            lot=load_lot(folder,partial);aptitudes={}
            for query,selection,item,headers,rows in tables(lot):
                check=compatibility(headers,selection,rows)
                aptitudes.setdefault(query['consulta'],[]).append(check['estado']+(' · faltan '+', '.join(check['faltan']) if check['faltan'] else ''))
            return lot,aptitudes
        self.start(worker_action,'load')
    def start(self,action,kind='product'):
        if self.busy:return
        self.busy=True;self.status.set('Comprobando archivos y procesando resultados localmente…')
        def work():
            try:self.events.put((kind,action(),None))
            except Exception as exc:self.events.put((kind,None,str(exc) if isinstance(exc,(m.PocError,OSError,ValueError)) else 'E_RESULTADOS: no se pudo generar el producto; los originales se conservan.'))
        threading.Thread(target=work,daemon=True).start()
    def run(self,action):self.safe(lambda:self.start(action))
    def poll(self):
        try:
            while True:
                kind,result,error=self.events.get_nowait();self.busy=False
                if error:self.status.set(error);continue
                if kind=='load':
                    self.lot,aptitudes=result;self.detail.delete(*self.detail.get_children())
                    for i,(q,manifest) in enumerate(self.lot.queries):
                        self.detail.insert('','end',iid=str(i),values=(q['consulta'],manifest['estado'],sum(v['registros'] for v in manifest.get('archivos',[])),
                            '; '.join(dict.fromkeys(aptitudes.get(q['consulta'],['Sin registros'])))))
                    self.status.set(('INCOMPLETO · ' if self.lot.partial else '')+self.lot.folder.name+' · Doble clic en una consulta para abrir su primer archivo verificado.')
                elif kind=='diary_load':
                    self.diary_capture=result;self.detail.delete(*self.detail.get_children())
                    for i,query in enumerate(result['queries']):
                        self.detail.insert('','end',iid=str(i),values=(query['rit']+' · '+query['persona'],query['coverage'],len(query['entries']),'Identidad de tribunal/causa/ingreso conservada'))
                    self.status.set(f'{len(result["queries"])} ingresos recuperados de bitácoras anteriores. Excel bitácoras autónomo genera su consolidado sin Chrome.')
                elif kind=='capture':
                    self.status.set('Paquete creado: '+str(result['path'])+' · '+result['reuse']+' Importa este ZIP en Trabajo → Importar paquete del Asistente.')
                else:
                    self.status.set('Creado: '+('; '.join(str(p) for p in result) if isinstance(result,list) else str(result)))
                    if kind=='csmp':self.launch_csmp(result)
        except queue.Empty:pass
        self.after(100,self.poll)
    def open_source(self):
        if self.diary_capture:
            selected=self.detail.selection()
            if selected:
                query=self.diary_capture['queries'][int(selected[0])]
                try:open_local(inside(Path(query['manifest_path']).parent,query['source']))
                except (m.PocError,OSError,KeyError) as exc:self.status.set('No hay una copia HTML verificable para esta lectura: '+str(exc))
            return
        def action():
            selected=self.detail.selection()
            if not selected:return
            _,manifest=self.lot.queries[int(selected[0])];items=manifest.get('archivos',[]) or [manifest.get('evidencia',{})]
            if items and items[0].get('archivo'):open_local(inside(self.lot.folder,items[0]['archivo']))
        self.safe(action)
    def csmp(self):self.safe(lambda:self.start(lambda:export_book([self.lot],csmp=True)))

    def open_request(self):
        if self.busy:return
        path=filedialog.askopenfilename(parent=self,title='Solicitud declarativa del Asistente',filetypes=[('Solicitud local','*.json')])
        if not path:return
        from solicitud_antecedentes import summarize_request
        try:messagebox.showinfo('Antecedentes solicitados',summarize_request(path),parent=self)
        except (ValueError,OSError) as exc:self.status.set(str(exc))

    def diary_excel(self):
        if self.busy:return
        manifests=[self.folder/'bitacoras.json'] if self.loaded_kind=='Bitácoras' else self.package_diaries()
        if not manifests:
            self.add_diaries();return
        target=filedialog.asksaveasfilename(parent=self,initialfile='Bitacoras_RUS.xlsx',defaultextension='.xlsx')
        if target:
            from nurus.capture.diaries import export_diaries
            self.start(lambda:export_diaries(manifests,target))

    def capture_package(self):
        if self.busy:return
        if not self.package_folders:self.status.set('Añade primero una carpeta de descarga de hojas RUS. Las bitácoras anteriores seguirán disponibles.');return
        target=filedialog.asksaveasfilename(parent=self,initialfile='Captura_CSMP.zip',defaultextension='.zip')
        if not target:return
        folders=list(self.package_folders);diaries=self.package_diaries();partial=self.partial.get()
        from export_csmp_capture import export_capture
        from archivo_local import load_sources
        def export():
            from nurus.capture import read_capture
            path=export_capture(load_sources(folders,partial),target,diaries=diaries or None,match_diaries=True)
            capture=read_capture(path)
            summary=next((i['message'] for i in capture.rows('incidencias') if i['code']=='BITACORAS_REUTILIZADAS'),'Sin bitácoras añadidas.')
            return {'path':path,'reuse':summary}
        self.start(export,'capture')

    def show_joint(self):
        if self.busy:return
        if self.loaded_kind!='Conjunto':self.status.set('Selecciona un conjunto del historial para abrir sus resultados.');return
        from ventana_flujo import JointWindow
        previous=self.app.last_folder;self.app.last_folder=self.folder
        try:JointWindow(self.app)
        finally:self.app.last_folder=previous
    def cross(self):
        def action():
            folder=filedialog.askdirectory(parent=self,title='Lote de informes compatible para el cruce')
            if folder:
                current=self.lot
                def export():
                    other=load_lot(folder)
                    modes={compatibility(h,s,rows)['modo'] for lot in (current,other) for _,s,_,h,rows in tables(lot)}
                    if modes!={'CUMPLIMIENTO','INFORMES'}:raise m.PocError('El cruce requiere un lote de Cumplimiento y otro de Informes, sin otras pantallas.')
                    return export_book([current,other],csmp=True)
                self.start(export)
        self.safe(action)
    def compare(self):
        def action():
            folder=filedialog.askdirectory(parent=self,title='Lote anterior del mismo alcance')
            if folder:
                current=self.lot;self.start(lambda:compare_lots(load_lot(folder),current))
        self.safe(action)
    def pending(self):self.safe(lambda:self.app.prepare_pending(self.lot.folder))
    def launch_csmp(self,path):
        executable=self.app.prefs.data.get('csmp','')
        if not executable:
            if not messagebox.askyesno('Libro preparado','¿Quieres abrirlo en la rama experimental de CSMP? Selecciona csmp-assistant.exe o pythonw.exe de su entorno.',parent=self):return
            executable=filedialog.askopenfilename(parent=self,title='Ejecutable de CSMP o Python de su entorno',filetypes=[('Ejecutable','*.exe')])
            if not executable:return
            self.app.prefs.data['csmp']=executable;self.app.prefs.save()
        try:
            args=[executable]
            if Path(executable).stem.lower() in ('python','pythonw'):args+=['-m','nurus.personal.app']
            from openpyxl import load_workbook
            book=load_workbook(path,read_only=True,data_only=True)
            try:modes=[name for name in ('ESPERA','CUMPLIMIENTO','INFORMES') if name in book.sheetnames]
            finally:book.close()
            extra=['--modo',modes[0],'--hoja',modes[0]] if len(modes)==1 else []
            subprocess.Popen([*args,'--archivo',str(path),*extra],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        except OSError:self.status.set('Libro creado. No se pudo abrir CSMP: revisa su ejecutable configurado.')
