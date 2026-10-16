"""Ventana de generación de Meta 4 desde las carpetas del Descargador SITFA.

No se conecta a SITFA ni a RUS: solo lee lotes ya descargados."""
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parent / 'scripts'))
from meta4_bitacoras import BASE, fecha_lectura, generar, meta, tribunales_disponibles
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
        root.geometry('820x720')
        root.minsize(720, 640)
        self.events = queue.Queue()
        self.busy = False
        self.output = None
        self.source = tk.StringVar()
        self.diaries = tk.StringVar()
        self.folder = tk.StringVar(value=str(BASE / 'Salidas'))
        self.cutoff = tk.StringVar(value=date.today().isoformat())
        self.status = tk.StringVar(value='Elige el lote de registros y las bitácoras descargadas con el Descargador SITFA.')
        frame = ttk.Frame(root, padding=24)
        frame.pack(fill='both', expand=True)
        ttk.Label(frame, text='Meta 4', font=('Segoe UI', 24, 'bold')).pack(anchor='w')
        ttk.Label(frame, text='Espera y Cumplimiento · FAE y Residencia · Bitácoras del último mes').pack(anchor='w', pady=(0, 16))

        box = ttk.LabelFrame(frame, text='1. Carpetas del Descargador', padding=12)
        box.pack(fill='x')
        self.folder_row(box, 'Registros (lote con resumen.json):', self.source, self.pick_source)
        self.folder_row(box, 'Bitácoras (bitacoras.json y HTML):', self.diaries, self.pick_diaries)
        ttk.Label(box, text='Si las bitácoras están dentro de la carpeta de registros, deja el segundo campo vacío.').pack(anchor='w')

        box = ttk.LabelFrame(frame, text='2. Tribunales', padding=12)
        box.pack(fill='both', expand=True, pady=(12, 0))
        self.courts = tk.Listbox(box, selectmode='extended', exportselection=False, height=6)
        self.courts.pack(fill='both', expand=True)
        self.fill_courts(meta.TRIBUNALS)
        line = ttk.Frame(box)
        line.pack(fill='x', pady=(6, 0))
        ttk.Label(line, text='Sin selección se incluyen todos. Con selección, la planilla se marca como parcial.').pack(side='left')
        ttk.Button(line, text='Todos', command=lambda: self.courts.selection_clear(0, 'end')).pack(side='right')

        box = ttk.LabelFrame(frame, text='3. Período y destino', padding=12)
        box.pack(fill='x', pady=12)
        line = ttk.Frame(box)
        line.pack(fill='x')
        ttk.Label(line, text='Fecha de corte (AAAA-MM-DD):').pack(side='left')
        ttk.Entry(line, textvariable=self.cutoff, width=16).pack(side='left', padx=8)
        self.folder_row(box, 'Destino de resultados:', self.folder, self.destination)

        line = ttk.Frame(frame)
        line.pack(fill='x')
        self.generate = ttk.Button(line, text='Generar Meta 4', command=self.start)
        self.generate.pack(side='left')
        ttk.Button(line, text='Abrir resultados', command=self.open_output).pack(side='right')
        ttk.Label(frame, textvariable=self.status, wraplength=760).pack(fill='x', pady=12)
        root.protocol('WM_DELETE_WINDOW', self.close)
        root.after(100, self.poll)

    @staticmethod
    def folder_row(parent, label, variable, command):
        line = ttk.Frame(parent)
        line.pack(fill='x', pady=(0, 8))
        ttk.Label(line, text=label, width=34).pack(side='left')
        ttk.Entry(line, textvariable=variable).pack(side='left', fill='x', expand=True)
        ttk.Button(line, text='Elegir…', command=command).pack(side='left', padx=(8, 0))

    def emit(self, name, value):
        self.events.put((name, value))

    def fill_courts(self, names):
        self.courts.delete(0, 'end')
        for name in names:
            self.courts.insert('end', name)

    def pick_source(self):
        path = filedialog.askdirectory(title='Lote de registros del Descargador', mustexist=True)
        if path:
            self.source.set(path)
            # Solo los tribunales consultados en el lote; sin resumen.json, todos.
            self.fill_courts(tribunales_disponibles(path) or meta.TRIBUNALS)
            if not self.diaries.get():
                self.suggest_cutoff(path)

    def pick_diaries(self):
        path = filedialog.askdirectory(title='Carpeta de bitácoras del Descargador', mustexist=True)
        if path:
            self.diaries.set(path)
            self.suggest_cutoff(path)

    def suggest_cutoff(self, path):
        # El corte no debe superar el día de lectura: lo posterior no está en las bitácoras.
        day = fecha_lectura(path)
        if day and day < date.today():
            self.cutoff.set(day.isoformat())
            self.status.set(f'Fecha de corte ajustada al día de lectura de las bitácoras: {day:%d-%m-%Y}.')

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
        if not self.source.get().strip():
            raise ValueError('Elige la carpeta del lote de registros.')
        source = Path(self.source.get()).expanduser()
        diaries = Path(self.diaries.get()).expanduser() if self.diaries.get().strip() else None
        courts = [self.courts.get(i) for i in self.courts.curselection()] or None
        return cutoff, source, diaries, Path(self.folder.get()).expanduser().resolve(), courts

    def start(self):
        if self.busy:
            return
        try:
            cutoff, source, diaries, folder, courts = self.parameters()
        except ValueError as exc:
            messagebox.showerror('Revisar datos', str(exc))
            return
        output = folder / ('Resultado META4 ' + datetime.now().strftime('%Y-%m-%d %H-%M-%S-%f'))
        self.busy = True
        self.generate.configure(state='disabled')
        self.status.set('Generando Meta 4 y consolidados…')

        def run():
            try:
                self.emit('complete', (str(output), generar(source, output, cutoff, bitacoras=diaries, tribunales=courts)))
            except (ValueError, OSError) as exc:
                self.emit('error', str(exc))
            except Exception as exc:
                self.emit('error', f'Error inesperado ({type(exc).__name__}). No se generaron archivos.')
            finally:
                self.emit('idle', None)
        threading.Thread(target=run, daemon=True).start()

    def poll(self):
        while True:
            try:
                name, value = self.events.get_nowait()
            except queue.Empty:
                break
            if name == 'idle':
                self.busy = False
                self.generate.configure(state='normal')
            elif name == 'error':
                self.status.set('El proceso requiere revisión.')
                messagebox.showerror('Meta 4', value)
            elif name == 'complete':
                self.output, report = value
                lectura = ', '.join(report['lectura_bitacoras'])
                alcance = 'todos los tribunales' if report['tribunales'] == 'TODOS' else 'PARCIAL: ' + ', '.join(report['tribunales'])
                text = (f"Generados {report['registros']} registros ({alcance}). Alertas: {report['alertas']}. "
                        f"Revisar bitácoras: {report['revision_bitacora']}. Lectura de bitácoras: {lectura}.")
                warnings = []
                if lectura != 'COMPLETA':
                    warnings.append('La lectura de bitácoras no quedó COMPLETA.')
                if report['consultas_sin_bitacoras']:
                    warnings.append(f"{len(report['consultas_sin_bitacoras'])} consulta(s) sin bitácoras: "
                                    + '; '.join(report['consultas_sin_bitacoras'][:5]))
                warnings.extend(report['avisos_bitacoras'])
                self.status.set(text)
                if warnings:
                    messagebox.showwarning('Archivos generados con avisos', text + '\n\n' + '\n'.join(warnings)
                                           + '\n\nLas filas afectadas quedan en amarillo.\n\n' + self.output)
                else:
                    messagebox.showinfo('Archivos generados', text + '\n\n' + self.output)
        self.root.after(100, self.poll)

    def open_output(self):
        if not self.output:
            messagebox.showinfo('Resultados', 'Todavía no hay resultados en esta sesión.')
        elif sys.platform == 'win32':
            os.startfile(self.output)
        else:
            messagebox.showinfo('Resultados', self.output)

    def close(self):
        if self.busy:
            self.status.set('Espera a que termine la generación.')
            return
        self.root.destroy()


if __name__ == '__main__':
    root = tk.Tk()
    App(root)
    root.mainloop()
