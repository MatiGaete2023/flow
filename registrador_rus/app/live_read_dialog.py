"""Conexión local de validación de un ingreso. No escritura ni persistencia."""
import tkinter as tk
from tkinter import ttk
from .bridge import BridgeError
from .live_read import open_live_read_bridge

class LiveReadDialog:
    def __init__(self,preview):
        self.preview=preview
        batch=preview.revalidate_confirmed()
        if len(batch.operations)!=1:
            raise BridgeError('VALIDACION_REQUIERE_UN_INGRESO')
        self.op=batch.operations[0]
        self.bridge=None
        self.closed=False
        self.window=tk.Toplevel(preview.window)
        self.window.title('Validar ingreso en RUS — solo lectura')
        self.window.transient(preview.window)
        self.window.protocol('WM_DELETE_WINDOW',self.close)
        self.window.bind('<Destroy>',self._destroyed,add='+')
        self.window.grab_set()
        body=ttk.Frame(self.window,padding=12);body.pack(fill='both',expand=True)
        ttk.Label(body,text='Abre listado y bitácora del ingreso confirmado. No se guardará nada.',wraplength=550).pack(anchor='w')
        self.origin=tk.StringVar(master=self.window)
        self.credentials=tk.StringVar(master=self.window)
        self.status=tk.StringVar(master=self.window,value='Sin conexión.')
        ttk.Label(body,text='Origen chrome-extension:// de la extensión de validación:').pack(anchor='w')
        ttk.Entry(body,textvariable=self.origin,width=60).pack(fill='x')
        self.start_button=ttk.Button(body,text='Abrir puente de lectura',command=self.start)
        self.start_button.pack(anchor='w',pady=4)
        ttk.Entry(body,textvariable=self.credentials,state='readonly',width=80).pack(fill='x')
        ttk.Label(body,text='Copia las credenciales solo a la extensión local.',wraplength=550).pack(anchor='w')
        ttk.Label(body,textvariable=self.status,wraplength=550).pack(anchor='w',pady=6)
        ttk.Button(body,text='Cerrar',command=self.close).pack(anchor='e')
    def start(self):
        try:
            if self.bridge is not None:raise BridgeError('PUENTE_YA_ABIERTO')
            if self.preview.revalidate_confirmed().operations!=(self.op,):
                raise BridgeError('APROBACION_CAMBIADA')
            self.bridge=open_live_read_bridge(self.origin.get().strip(),self.op)
            self.credentials.set(f'{self.bridge.server.server_port} | {self.bridge.mailbox.session_id} | {self.bridge.token}')
            self.start_button.configure(state='disabled')
            self.status.set('Puente local abierto. Pulsa Validar en la extensión.')
            self.window.after(200,self.poll)
        except (ValueError,RuntimeError,OSError):
            self.status.set('No se pudo abrir el puente local.')
    def poll(self):
        if self.closed or self.bridge is None:return
        result=self.bridge.mailbox.take_result()
        if result is None:
            self.window.after(200,self.poll);return
        self.bridge.close();self.bridge=None;self.credentials.set('')
        try:
            if self.preview.revalidate_confirmed().operations!=(self.op,):raise BridgeError('APROBACION_CAMBIADA')
            self.status.set(result['status']+(f" | Entradas: {result['entries']}" if 'entries' in result else '')+' | Sin registro.')
        except (ValueError,RuntimeError):
            self.status.set('La revisión cambió; lectura no utilizable.')
    def _destroyed(self,event):
        if event.widget is self.window and not self.closed:
            self.closed=True
            if self.bridge:self.bridge.close();self.bridge=None
    def close(self):
        if self.closed:return
        self.closed=True
        if self.bridge:self.bridge.close();self.bridge=None
        self.credentials.set('')
        self.window.destroy()
        if self.preview.window.winfo_exists():self.preview.window.grab_set()
