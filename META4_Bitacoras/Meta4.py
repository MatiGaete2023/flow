"""Ventana de descarga y generación de Meta 4."""
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parent / 'scripts'))
from meta4_bitacoras import BASE, generar, meta
from descarga_meta4 import descargar
from puente import Bridge
from motor import PocError
from datetime import date, datetime
import os
import queue
import threading
import tkinter as tk
from tkinter import ttk, filedialog, messagebox


class App:
    def __init__(self, root):
        self.root = root
        root.title('Meta 4 · Registros y bitácoras')
        root.geometry('850x690')
        root.minsize(760, 620)
        self.events = queue.Queue()
        self.cancel = threading.Event()
        self.busy = False
        self.catalog = None
        self.court_ids = []
        self.output = None
        self.read_count = 0
        self.bridge = Bridge(self.emit)
        self.folder = tk.StringVar(value=str(BASE / 'Salidas'))
        self.cutoff = tk.StringVar(value=date.today().isoformat())
        self.status = tk.StringVar(value='Conecta Chrome o procesa una carpeta ya descargada.')
        frame = ttk.Frame(root, padding=24)
        frame.pack(fill='both', expand=True)
        ttk.Label(frame, text='Meta 4', font=('Segoe UI', 24, 'bold')).pack(anchor='w')
        ttk.Label(frame, text='Espera y Cumplimiento · FAE y Residencia · Bitácoras del último mes').pack(anchor='w', pady=(0, 16))
        box = ttk.LabelFrame(frame, text='1. Conectar el descargador', padding=12)
        box.pack(fill='x')
        ttk.Label(box, text='Entra a SITFA → Seguimiento y conecta la extensión incluida usando este código:').pack(anchor='w')
        code = ttk.Entry(box, font=('Consolas', 12))
        code.insert(0, self.bridge.connection_code)
        code.configure(state='readonly')
        code.pack(fill='x', pady=8)
        self.load = ttk.Button(box, text='Cargar tribunales de SITFA', command=lambda: self.work(self.load_catalog))
        self.load.pack(anchor='w')
        box = ttk.LabelFrame(frame, text='2. Elegir tribunales y fecha de corte', padding=12)
        box.pack(fill='both', expand=True, pady=12)
        self.courts = tk.Listbox(box, selectmode='extended', exportselection=False, height=8)
        self.courts.pack(fill='both', expand=True)
        line = ttk.Frame(box)
        line.pack(fill='x')
        ttk.Label(line, text='Ctrl + clic para seleccionar varios tribunales. Se incluyen Espera y Cumplimiento.').pack(side='left')
        ttk.Button(line, text='Seleccionar todos', command=lambda: self.courts.select_set(0, 'end')).pack(side='right')
        line = ttk.Frame(box)
        line.pack(fill='x', pady=(10, 0))
        ttk.Label(line, text='Fecha de corte (AAAA-MM-DD):').pack(side='left')
        ttk.Entry(line, textvariable=self.cutoff, width=16).pack(side='left', padx=8)
        box = ttk.LabelFrame(frame, text='3. Generar archivos', padding=12)
        box.pack(fill='x')
        line = ttk.Frame(box)
        line.pack(fill='x')
        ttk.Entry(line, textvariable=self.folder).pack(side='left', fill='x', expand=True)
        ttk.Button(line, text='Destino…', command=self.destination).pack(side='left', padx=(8, 0))
        self.download = ttk.Button(box, text='Descargar y generar Meta 4', command=self.start)
        self.download.pack(side='left', pady=(10, 0))
        self.offline = ttk.Button(box, text='Usar carpeta ya descargada…', command=self.offline_start)
        self.offline.pack(side='left', padx=8, pady=(10, 0))
        self.stop = ttk.Button(box, text='Detener', command=self.request_stop, state='disabled')
        self.stop.pack(side='right', pady=(10, 0))
        ttk.Label(frame, textvariable=self.status, wraplength=780).pack(fill='x', pady=12)
        ttk.Button(frame, text='Abrir resultados', command=self.open_output).pack(anchor='e')
        root.protocol('WM_DELETE_WINDOW', self.close)
        root.after(100, self.poll)

    def emit(self, name, value):
        self.events.put((name, value))

    def work(self, action):
        if self.busy:
            return
        self.busy = True
        self.cancel.clear()
        self.read_count = 0
        for button in (self.load, self.download, self.offline):
            button.configure(state='disabled')
        self.stop.configure(state='normal')

        def run():
            try:
                action()
            except (PocError, ValueError, OSError) as exc:
                self.emit('error', str(exc))
            except Exception as exc:
                # Error no previsto: el tipo orienta el soporte sin exponer datos.
                self.emit('error', f'Error inesperado ({type(exc).__name__}). Los archivos verificados se conservan.')
            finally:
                self.emit('idle', None)
        threading.Thread(target=run, daemon=True).start()

    def request_stop(self):
        self.cancel.set()
        self.status.set('Deteniendo al terminar la lectura en curso…')

    def load_catalog(self):
        self.emit('catalog', self.bridge.call('catalog', {'screen': 'seguimiento'}))

    def destination(self):
        path = filedialog.askdirectory(title='Carpeta de destino')
        if path:
            self.folder.set(path)

    def parameters(self):
        try:
            cutoff = date.fromisoformat(self.cutoff.get().strip())
        except ValueError:
            raise ValueError('Escribe la fecha de corte como AAAA-MM-DD, por ejemplo 2026-10-16.') from None
        if cutoff > date.today():
            raise ValueError('La fecha de corte no puede estar en el futuro.')
        return cutoff, Path(self.folder.get()).expanduser().resolve()

    def start(self):
        try:
            cutoff, folder = self.parameters()
            if not self.catalog:
                raise ValueError('Conecta la extensión y carga los tribunales.')
            ids = [self.court_ids[i] for i in self.courts.curselection()]
            if not ids:
                raise ValueError('Selecciona al menos un tribunal.')
        except ValueError as exc:
            messagebox.showerror('Revisar selección', str(exc))
            return

        def action():
            output, report = descargar(self.bridge, self.catalog, ids, folder, cutoff, self.emit, self.cancel)
            self.emit('complete', (str(output), report))
        self.work(action)

    def offline_start(self):
        try:
            cutoff, folder = self.parameters()
        except ValueError as exc:
            messagebox.showerror('Revisar fecha', str(exc))
            return
        source = filedialog.askdirectory(title='Carpeta con Excel originales y bitácoras del descargador')
        if not source:
            return
        output = folder / ('Resultado META4 ' + datetime.now().strftime('%Y-%m-%d %H-%M-%S-%f'))

        def action():
            self.emit('status', 'Generando Meta 4 y consolidados…')
            report = generar(source, output, cutoff)
            self.emit('complete', (str(output), report))
        self.work(action)

    def poll(self):
        while True:
            try:
                name, value = self.events.get_nowait()
            except queue.Empty:
                break
            if name == 'idle':
                self.busy = False
                for button in (self.load, self.download, self.offline):
                    button.configure(state='normal')
                self.stop.configure(state='disabled')
            elif name == 'catalog':
                self.catalog = value
                self.courts.delete(0, 'end')
                self.court_ids = [k for k, label in value['tribunales'].items() if meta.tribunal_name(label)]
                for k in self.court_ids:
                    self.courts.insert('end', value['tribunales'][k])
                self.status.set('Tribunales de Meta 4 cargados. Selecciona los que deseas incluir.' if self.court_ids else 'No hay tribunales reconocidos dentro del alcance de la planilla Meta 4.')
            elif name == 'error':
                self.status.set('El proceso requiere revisión.')
                messagebox.showerror('Meta 4', value)
            elif name == 'complete':
                self.output, report = value
                lectura = report.get('lectura') or ', '.join(report.get('lectura_bitacoras', []))
                text = (f"Generados {report['registros']} registros. Alertas: {report['alertas']}. "
                        f"Revisar bitácoras: {report['revision_bitacora']}. Lectura de bitácoras: {lectura}.")
                self.status.set(text)
                if lectura != 'COMPLETA':
                    messagebox.showwarning('Archivos generados con lectura incompleta',
                                           text + '\n\nLas filas sin lectura quedan en amarillo con FECHA OBS. vacía.\n\n' + self.output)
                else:
                    messagebox.showinfo('Archivos generados', text + '\n\n' + self.output)
            elif name == 'query':
                self.status.set(f"Consulta {value['index']}/{value['total']}: {value['label']}")
            elif name == 'diary_progress':
                self.read_count += 1
                self.status.set(f'Leyendo bitácoras del mes… {self.read_count} procesadas.')
            elif name in ('status', 'warning', 'connected'):
                self.status.set(str(value))
        self.root.after(100, self.poll)

    def open_output(self):
        if not self.output:
            messagebox.showinfo('Resultados', 'Todavía no hay resultados en esta sesión.')
        elif sys.platform == 'win32':
            os.startfile(self.output)
        else:
            messagebox.showinfo('Resultados', self.output)

    def close(self):
        self.cancel.set()
        if self.busy:
            self.status.set('Deteniendo el proceso; se conservarán las descargas comprobadas…')
            self.root.after(500, self.close)
            return
        self.bridge.close()
        self.root.destroy()


if __name__ == '__main__':
    root = tk.Tk()
    App(root)
    root.mainloop()
