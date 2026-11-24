"""UI local de piloto unitario real. No se puede abrir en versión no aceptada."""
from datetime import date
from pathlib import Path
import tkinter as tk
from tkinter import ttk,messagebox,filedialog,simpledialog
from .contracts import CAPABILITIES,ContractError
from .pilot_journal import PilotJournal,PilotPolicy
from .pilot_bridge import open_pilot_bridge
from .pilot_controller import PilotController
from .flow_work import REPOSITORY_REPORT_CATALOG
from .private_demo_directory import create_private_demo_directory


class PilotDialog:
    def __init__(self,parent,preview=None,*,recovery=False):
        if CAPABILITIES['write_one'] is not True:raise ContractError('PILOTO_REAL_NO_HABILITADO')
        self.preview=preview;self.recovery=recovery
        self.journal=None;self.bridge=None;self.controller=None;self.op=None;self.closed=False
        self.window=tk.Toplevel(parent);self.window.title('Piloto unitario RUS — recuperación' if recovery else 'Piloto unitario RUS')
        self.window.transient(parent);self.window.grab_set();self.window.minsize(760,540)
        self.window.protocol('WM_DELETE_WINDOW',self.close);self.window.bind('<Destroy>',self._destroyed,add='+')
        body=ttk.Frame(self.window,padding=12);body.pack(fill='both',expand=True)
        self.values={}
        for key,label in (('directory','Carpeta privada acordada'),('retention','Retención acordada (días)'),
            ('acceptance','Referencia de aceptación'),('extension','Origen chrome-extension://'),
            ('actor','Autor exacto RUS'),('stage','Etapa exacta en bitácora'),('day','Fecha servidor AAAA-MM-DD')):
            row=ttk.Frame(body);row.pack(fill='x',pady=2)
            ttk.Label(row,text=label,width=27).pack(side='left')
            v=tk.StringVar(master=self.window);self.values[key]=v
            ttk.Entry(row,textvariable=v).pack(side='left',fill='x',expand=True)
            if key=='directory':
                ttk.Button(row,text='Elegir',command=self.choose).pack(side='left')
                ttk.Button(row,text='Crear privada nueva',command=lambda:self.run(self.create_folder)).pack(side='left')
        self.require_excel=tk.BooleanVar(master=self.window,value=False)
        ttk.Checkbutton(body,text='Conciliar FECHA_OBS y CC en una copia Excel posterior',variable=self.require_excel).pack(anchor='w')
        self.authority=tk.BooleanVar(master=self.window,value=False)
        ttk.Checkbutton(body,text='Confirmo que cuento con autorización aplicable y que ruta/retención están acordadas.',
                        variable=self.authority).pack(anchor='w')
        self.start_button=ttk.Button(body,text='Abrir diario y puente local',command=lambda:self.run(self.start))
        self.start_button.pack(anchor='w',pady=4)
        self.credentials=tk.StringVar(master=self.window)
        ttk.Entry(body,textvariable=self.credentials,state='readonly').pack(fill='x')
        self.text=tk.Text(body,height=6,wrap='word',state='disabled');self.text.pack(fill='both',expand=True,pady=5)
        row=ttk.Frame(body);row.pack(fill='x')
        self.prepare_button=ttk.Button(row,text='Preparar sin guardar',state='disabled',command=lambda:self.run(self.prepare))
        self.prepare_button.pack(side='left')
        self.authorize_button=ttk.Button(row,text='Autorizar esta creación',state='disabled',command=lambda:self.run(self.authorize))
        self.authorize_button.pack(side='left',padx=4)
        self.read_button=ttk.Button(row,text='Releer bitácora',state='disabled',command=lambda:self.run(self.readback))
        self.read_button.pack(side='left')
        self.excel_button=ttk.Button(row,text='Conciliar Excel',state='disabled',command=lambda:self.run(self.excel))
        self.excel_button.pack(side='left',padx=4)
        ttk.Button(row,text='Cerrar',command=self.close).pack(side='right')
        self.status=tk.StringVar(master=self.window,value='Sin conexión. No hay reserva.')
        ttk.Label(body,textvariable=self.status,wraplength=730).pack(anchor='w',pady=4)
    def choose(self):
        chosen=filedialog.askdirectory(parent=self.window,mustexist=True)
        if chosen:self.values['directory'].set(chosen)
    def create_folder(self):
        parent=filedialog.askdirectory(parent=self.window,mustexist=True)
        if not parent:return
        path=Path(parent)/'FlowRegistrador'
        if messagebox.askyesno('Carpeta privada nueva',f'¿Crear esta carpeta nueva con ACL privada? No se modificarán carpetas existentes.\n\n{path}',parent=self.window):
            self.values['directory'].set(str(create_private_demo_directory(path)))
    def value(self,k):return self.values[k].get().strip()
    def run(self,action):
        try:return action()
        except (ValueError,RuntimeError,OSError):
            self.status.set('Operación bloqueada. No repetir una creación reservada.')
    def _current(self):
        batch=self.preview.revalidate_confirmed()
        if len(batch.operations)!=1:raise ContractError('PILOTO_UNITARIO_REQUERIDO')
        return batch.operations[0]
    def start(self):
        if self.journal is not None:raise ContractError('DIARIO_YA_ABIERTO')
        folder=Path(self.value('directory'))
        policy=PilotPolicy(folder,int(self.value('retention')),self.value('acceptance'),self.authority.get())
        # Ruta única: no crear otro diario con nombre aleatorio. El directorio
        # debe existir y cumplir la DACL aprobada; no se cambian ACL aquí.
        path=folder/'registrador.pilot.protected.sqlite'
        if self.recovery and not path.is_file():raise ContractError('DIARIO_NO_ENCONTRADO')
        j=PilotJournal(path,policy=policy)
        bridge=None
        try:
            if self.recovery:op=j.load_unresolved()
            else:
                self._current()
                op=j.freeze_confirmed(self.preview.confirmed_preview,self.preview.work,self.preview.current_requests(),
                                     report_catalog=REPOSITORY_REPORT_CATALOG)
            bridge=open_pilot_bridge(self.value('extension'),op)
            c=PilotController(j,bridge.mailbox,op,revalidate=None if self.recovery else self._current,recovery=self.recovery)
        except BaseException:
            if bridge:bridge.close()
            j.close();raise
        self.journal=j;self.bridge=bridge;self.controller=c;self.op=op
        self.credentials.set(f'{bridge.server.server_port} | {bridge.mailbox.session_id} | {bridge.token}')
        i=op.remote_identity
        self.text.configure(state='normal')
        self.text.insert('1.0',f'Tribunal {i.tribunal_id} / Causa {i.causa_id} / Ingreso {i.ingreso_id}\n{op.origin} | Estado {op.observation_state} | {op.destination} | CC {op.cc}\n{op.approved_text}')
        self.text.configure(state='disabled')
        self.start_button.configure(state='disabled')
        self.refresh()
        self.window.after(200,self.poll)
    def prepare(self):
        if messagebox.askyesno('Preparación sin guardado','¿Preparar exactamente el texto y destino mostrados, sin guardar?',parent=self.window):
            self.controller.prepare(confirmed=True);self.refresh()
    def authorize(self):
        # La autorización es individual y se muestra completa en el panel no editable.
        # No inferir autor/etapa/fecha del servidor desde el reloj o nombre local.
        if messagebox.askyesno('Autorizar una creación real','¿Autorizas exactamente esta observación, Estado y destino mostrados? Solo una creación; no se repetirá automáticamente.',parent=self.window):
            self.controller.authorize_and_verify(author=self.value('actor'),stage=self.value('stage'),
                server_day=date.fromisoformat(self.value('day')),confirmed=True,require_excel=self.require_excel.get())
            self.refresh()
    def readback(self):
        if messagebox.askyesno('Relectura','¿Está abierta la bitácora del mismo ingreso y ya no hay una orden de creación pendiente en Chrome?',parent=self.window):
            self.controller.request_readback(confirmed=True);self.refresh()
    def excel(self):
        from .reconcile_excel import ExcelRequest,reconcile_pilot,_load,_cells,_date,_cc
        if self.controller.phase!='PENDIENTE_EXCEL':raise ContractError('RECIBO_NO_ELEGIBLE')
        source=filedialog.askopenfilename(parent=self.window,filetypes=[('Excel simple','*.xlsx')])
        if not source:return
        sheet=simpledialog.askstring('Hoja','Nombre exacto de hoja:',parent=self.window)
        if not sheet:return
        req=ExcelRequest(self.op,sheet,None,None)
        book=_load(Path(source).read_bytes())
        d,c=_cells(book,req)
        req=ExcelRequest(self.op,sheet,_date(d.value),_cc(c.value))
        destination=filedialog.asksaveasfilename(parent=self.window,initialdir=str(self.journal.policy.directory),defaultextension='.xlsx',filetypes=[('Excel simple','*.xlsx')])
        if not destination:return
        if not messagebox.askyesno('Conciliar copia','¿Crear una copia nueva en la carpeta privada con la fecha y CC del recibo verificado? No se enviará nada a RUS.',parent=self.window):return
        reconcile_pilot(self.journal,req,source,destination)
        self.controller.phase='COMPROBADA';self.refresh()
    def poll(self):
        if self.closed or self.controller is None:return
        try:
            self.controller.poll();self.refresh()
        except (ValueError,RuntimeError,OSError):
            self.status.set('Bloqueada. Si hay reserva, cerrar y recuperar exclusivamente por lectura.')
            for b in (self.prepare_button,self.authorize_button,self.read_button,self.excel_button):b.configure(state='disabled')
            return
        self.window.after(200,self.poll)
    def refresh(self):
        phase=self.controller.phase
        self.prepare_button.configure(state='normal' if phase=='NUEVA' else 'disabled')
        self.authorize_button.configure(state='normal' if phase=='PREPARADA' else 'disabled')
        self.read_button.configure(state='normal' if phase in ('RECUPERACION','INCIERTA','NO_HALLADA') else 'disabled')
        self.excel_button.configure(state='normal' if phase=='PENDIENTE_EXCEL' else 'disabled')
        self.status.set(phase+' | Para ejecutar orden pendiente, pulsa el botón de la extensión local.')
    def _shutdown(self):
        if self.closed:return
        self.closed=True
        try:
            if self.controller:self.controller.stop()
        finally:
            if self.bridge:self.bridge.close()
            if self.journal:self.journal.close()
    def _destroyed(self,event):
        if event.widget is self.window:
            try:self._shutdown()
            except (ValueError,RuntimeError,OSError):pass
    def close(self):
        try:self._shutdown()
        except (ValueError,RuntimeError,OSError):pass
        self.credentials.set('');self.window.destroy()
        if self.preview and self.preview.window.winfo_exists():self.preview.window.grab_set()


if __name__=='__main__':
    # Recuperación de intento original sin depender de que Work siga abierto.
    import argparse
    parser=argparse.ArgumentParser(description='Recuperar piloto unitario por lectura')
    parser.add_argument('--recover',action='store_true',required=True)
    parser.parse_args()
    if CAPABILITIES['write_one'] is not True:parser.exit(2,'Piloto real no habilitado.\n')
    root=tk.Tk();root.withdraw()
    d=PilotDialog(root,recovery=True)
    root.wait_window(d.window);root.destroy()
