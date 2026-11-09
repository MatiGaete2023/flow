"""Ventana de Meta 4: lotes del Descargador → planilla Meta 4. No se conecta a SITFA ni a RUS."""
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parent / 'scripts'))
from meta4_bitacoras import (  # noqa: E402
    BASE, archivos_bitacoras, buscar_archivos, fecha_lectura, generar, meta, raices, regenerar, tribunales_disponibles,
)
from datetime import date, datetime  # noqa: E402
import os  # noqa: E402
import queue  # noqa: E402
import threading  # noqa: E402
import tkinter as tk  # noqa: E402
from tkinter import ttk, filedialog, messagebox  # noqa: E402,F401

import flowkit as ui  # noqa: E402

EXCEL_RUTA_MAXIMA = 218


def nueva_salida(folder, sufijo=''):
    """Carpeta de resultados con nombre corto: la ruta completa debe caber en el límite de Windows."""
    base = 'Resultado META4 ' + datetime.now().strftime('%Y-%m-%d %H-%M-%S') + sufijo
    output, n = folder / base, 2
    while output.exists():
        output, n = folder / f'{base} ({n})', n + 1
    return output


class App:
    def __init__(self, root):
        self.root = root
        self.events = queue.Queue()
        self.busy = False
        self.output = None
        self.sources = []
        self.lot_count = 0
        self.diaries = tk.StringVar()
        self.folder = tk.StringVar(value=str(BASE / 'Salidas'))
        self.cutoff = tk.StringVar(value=date.today().isoformat())
        self.status = tk.StringVar()
        self.window = ui.Window(root, 'Meta 4', 'Generar Meta 4', self.start, size='820x600', minsize=(680, 480))
        top = self.window.top
        top.add('Agregar lote…', self.pick_source)
        top.add('Quitar lote elegido', self.remove_sources)
        top.add('Vaciar', lambda: self.busy or self.set_sources([]))
        top.add('Bitácoras en otra carpeta…', self.pick_diaries, separator=True)
        top.add('Destino de resultados…', self.destination)
        top.add('Regenerar desde Registros revisado…', self.start_regenerate, separator=True)
        top.add('Abrir resultados', self.open_output)
        self.add_button = self.window.footer.add('Agregar lote…', self.pick_source)
        self.generate = self.window.footer.primary
        form = ui.Form(self.window.body, key_width=11, padding=(16, 14))
        form.pack(fill='both', expand=True)
        self.lots = form.row('Lotes', self._lots_view)
        self.courts = form.row('Tribunales', lambda p: ui.Chips(p, command=self.refresh))
        self.cutoff_entry = form.row('Corte', lambda p: ttk.Entry(p, textvariable=self.cutoff, width=12))
        self.notices = form.row('Avisos', lambda p: ui.ProgressList(p, height=5), name='avisos')
        self.notices.bar.pack_forget()
        form.show('avisos', False)
        self.form = form
        self.fill_courts(meta.TRIBUNALS)
        self.cutoff.trace_add('write', lambda *_: self.refresh())
        self.status.trace_add('write', self._status_changed)
        self.refresh()
        root.protocol('WM_DELETE_WINDOW', self.close)
        root.after(100, self.poll)

    def _lots_view(self, parent):
        tree = ttk.Treeview(parent, columns=('lote', 'detalle'), show='', height=5, selectmode='extended')
        tree.column('lote', width=380)
        tree.column('detalle', width=200, anchor='e')
        tree.tag_configure('falta', foreground=ui.PALETTE['warn'])
        tree.tag_configure('vacio', foreground=ui.PALETTE['muted'])
        return tree

    def _status_changed(self, *_):
        text = self.status.get()
        self.window.banner.show(text) if text else self.window.banner.hide()

    def emit(self, name, value):
        self.events.put((name, value))

    def fill_courts(self, names):
        names = list(names)
        self.courts.set_options([(n, n.title()) for n in names], selected=names)

    def selected_courts(self):
        chosen = list(self.courts.get())
        return None if len(chosen) == len(self.courts.buttons) else chosen

    def refresh(self):
        # Conteo guardado en set_sources: recorrer las carpetas en cada tecla de la fecha era lento con muchos lotes.
        lots = self.lot_count
        courts = self.courts.get()
        partial = '' if len(courts) == len(self.courts.buttons) else ' (parcial)'
        summary = f'{lots} lote{"s" if lots != 1 else ""} · {len(courts)} tribunal{"es" if len(courts) != 1 else ""}{partial} · corte {self.cutoff.get()}'
        self.window.footer.set(summary if self.sources else '', enabled=bool(self.sources and courts) and not self.busy)

    def set_sources(self, paths):
        self.sources = [str(p) for p in raices(paths)]
        self.lot_count = len(buscar_archivos(raices(self.sources), 'resumen.json')) if self.sources else 0
        self.lots.delete(*self.lots.get_children())
        external = bool(self.diaries.get().strip())
        for index, path in enumerate(self.sources):
            found = len(buscar_archivos(raices([path]), 'resumen.json'))
            diaries = external or bool(archivos_bitacoras(path))
            detail = f'{found} lote{"s" if found != 1 else ""} · ' + ('bitácoras ✓' if diaries else 'faltan bitácoras')
            tag = 'vacio' if not found else ('' if diaries else 'falta')
            self.lots.insert('', 'end', iid=str(index), values=(Path(path).name, detail), tags=(tag,))
        # Solo los tribunales consultados en los lotes; sin resumen.json, todos.
        self.fill_courts(tribunales_disponibles(self.sources) or meta.TRIBUNALS)
        self.refresh()

    @property
    def lots_info(self):
        return f'{len(self.sources)} carpeta(s) · {self.lot_count} lote(s)'

    def pick_source(self):
        if self.busy:  # el menú «Más» sigue activo mientras se genera
            return
        path = filedialog.askdirectory(title='Lote del Descargador (o carpeta con varios lotes)', mustexist=True)
        if path:
            self.set_sources(self.sources + [path])
            if not self.diaries.get():
                self.suggest_cutoff(self.sources)

    def remove_sources(self):
        if self.busy:
            return
        chosen = {int(i) for i in self.lots.selection()}
        self.set_sources([p for i, p in enumerate(self.sources) if i not in chosen])

    def pick_diaries(self):
        if self.busy:
            return
        path = filedialog.askdirectory(title='Carpeta de bitácoras del Descargador', mustexist=True)
        if path:
            self.diaries.set(path)
            self.set_sources(self.sources)
            self.suggest_cutoff(path)

    def suggest_cutoff(self, path):
        # El corte no debe superar el día de lectura: lo posterior no está en las bitácoras.
        day = fecha_lectura(path)
        if day and day < date.today():
            self.cutoff.set(day.isoformat())
            self.status.set(f'Corte ajustado al día de lectura de las bitácoras: {day:%d-%m-%Y}.')

    def destination(self):
        path = filedialog.askdirectory(title='Destino de resultados')
        if path:
            self.folder.set(path)

    def parameters(self):
        try:
            cutoff = date.fromisoformat(self.cutoff.get().strip())
        except ValueError:
            raise ValueError('Escribe la fecha de corte como AAAA-MM-DD, por ejemplo 2026-10-16.') from None
        if cutoff > date.today():
            raise ValueError('La fecha de corte no puede estar en el futuro.')
        if not self.sources:
            raise ValueError('Agrega al menos un lote de registros.')
        if not self.courts.get():
            raise ValueError('Marca al menos un tribunal.')
        source = [Path(p) for p in self.sources]
        diaries = Path(self.diaries.get()).expanduser() if self.diaries.get().strip() else None
        return cutoff, source, diaries, Path(self.folder.get()).expanduser().resolve(), self.selected_courts()

    def fail(self, text):
        self.window.banner.show(text, bad=True)

    def start(self):
        if self.busy:
            return
        try:
            cutoff, source, diaries, folder, courts = self.parameters()
        except ValueError as exc:
            self.fail(str(exc))
            return
        output = nueva_salida(folder)
        self.run(output, lambda: generar(source, output, cutoff, bitacoras=diaries, tribunales=courts),
                 'Generando Meta 4 y consolidados…')

    def start_regenerate(self):
        """META 4 y consolidados nuevos desde un Registros.xlsx corregido a mano."""
        if self.busy:
            return
        path = filedialog.askopenfilename(title='Registros.xlsx revisado', filetypes=[('Excel', '*.xlsx')])
        if not path:
            return
        output = nueva_salida(Path(self.folder.get()).expanduser().resolve(), ' regenerado')
        self.run(output, lambda: regenerar(path, output), 'Regenerando Meta 4 desde Registros revisado…')

    def run(self, output, task, message):
        self.busy = True
        self.generate.state(['disabled'])
        self.add_button.state(['disabled'])
        self.form.show('avisos', False)
        self.status.set(message)

        def run():
            try:
                self.emit('complete', (str(output), task()))
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
                self.add_button.state(['!disabled'])
                self.refresh()
            elif name == 'error':
                self.status.set('')
                self.fail(value)
            elif name == 'complete':
                self.complete(*value)
        self.root.after(100, self.poll)

    def complete(self, output, report):
        self.output = output
        lectura = ', '.join(report['lectura_bitacoras'])
        alcance = 'todos los tribunales' if report['tribunales'] == 'TODOS' else 'PARCIAL: ' + ', '.join(report['tribunales'])
        lotes = len(report['lotes_registros'])
        text = ('Regenerado desde Registros revisado. ' if report.get('regenerado_desde') else '') + (
            f"Generados {report['registros']} registros" + (f' de {lotes} lote(s)' if lotes else '')
            + f" ({alcance}). Alertas: {report['alertas']}. Revisar bitácoras: {report['revision_bitacora']}.")
        warnings = []
        if lectura != 'COMPLETA':
            warnings.append('La lectura de bitácoras no quedó COMPLETA.')
        if report['consultas_sin_bitacoras']:
            warnings.append(f"{len(report['consultas_sin_bitacoras'])} consulta(s) sin bitácoras: "
                            + '; '.join(report['consultas_sin_bitacoras'][:5]))
        if report['excluidos_meta4']:
            warnings.append(f"{report['excluidos_meta4']} registro(s) fuera de META 4: su columna TRIBUNAL no "
                            'identifica un tribunal (ver ERRORES_Y_ADVERTENCIAS).')
        warnings.extend(report['avisos_bitacoras'])
        warnings.extend('Lote con filtros: ' + f for f in report['filtros_registros'])
        longest = max((len(str(p)) for p in Path(output).iterdir()), default=0)
        if longest > EXCEL_RUTA_MAXIMA:
            warnings.append(f'La ruta de los archivos tiene {longest} caracteres: Excel podría no abrirlos '
                            f'(máximo {EXCEL_RUTA_MAXIMA}). Usa un destino más corto.')
        self.warnings = warnings
        self.notices.clear()
        for number, warning in enumerate(warnings, 1):
            self.notices.set(number, 'warn', name=warning)
        self.form.show('avisos', bool(warnings))
        self.status.set(text)
        self.window.banner.show(text + (' Las filas afectadas quedan en amarillo.' if warnings else ''),
                                action=self.open_output, action_text='Abrir resultados')

    def open_output(self):
        if not self.output:
            self.status.set('Todavía no hay resultados en esta sesión.')
        elif not Path(self.output).is_dir():
            self.status.set('La carpeta de resultados ya no existe.')
        elif sys.platform == 'win32':
            os.startfile(self.output)
        else:
            self.status.set(self.output)

    def close(self):
        if self.busy:
            self.status.set('Espera a que termine la generación.')
            return
        self.root.destroy()


if __name__ == '__main__':
    root = tk.Tk()
    App(root)
    root.mainloop()
