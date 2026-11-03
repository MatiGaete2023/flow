"""SITFA Lite: descargas XLS de RUS (Seguimiento, Busqueda Lit, Informes y Medidas por vencer).

Sin paquetes consolidados, bitacoras, historias de la causa ni calendarios por ingreso.
"""

import calendar
from datetime import date, timedelta
import json
import os
from pathlib import Path
import queue
import re
import subprocess
import sys
import threading
import tkinter as tk
from tkinter import filedialog, messagebox, ttk

import flowkit as ui

import motor as m
from lotes import ALL_MEASURES, Batch, BatchRunner, Dates, MODALITIES, TAB_ORDER
from pausa import PauseGate
from perfiles import SCREENS
from puente import Bridge
from version import VERSION, extension_dir, verify_extension

MONTH_NAMES = (
    'Enero Febrero Marzo Abril Mayo Junio Julio Agosto Septiembre Octubre Noviembre Diciembre'
).split()
PERIODS = ('Sin filtro', 'Por fecha')


def data_directory():
    base = os.environ.get('LOCALAPPDATA') or str(Path.home())
    return Path(base) / 'SITFA_Lite'


class Preferences:
    def __init__(self, path=None):
        self.path = Path(path) if path else data_directory() / 'preferencias.json'
        self.data = {'destino': str(Path.home() / 'Documents' / 'SITFA Lite'), 'pausa': 1.0, 'pdf_sin_registros': True}
        try:
            stored = json.loads(self.path.read_text(encoding='utf-8'))
            if isinstance(stored.get('destino'), str):
                self.data['destino'] = stored['destino']
            if isinstance(stored.get('pausa'), (int, float)) and 0 <= stored['pausa'] <= 30:
                self.data['pausa'] = float(stored['pausa'])
            if isinstance(stored.get('pdf_sin_registros'), bool):
                self.data['pdf_sin_registros'] = stored['pdf_sin_registros']
        except (OSError, ValueError, AttributeError):
            pass

    def save(self):
        try:
            self.path.parent.mkdir(parents=True, exist_ok=True)
            self.path.write_text(json.dumps(self.data, ensure_ascii=False, indent=2), encoding='utf-8')
        except OSError:
            pass


def open_chrome(url):
    candidates = [
        Path(os.environ.get(env, '')) / 'Google/Chrome/Application/chrome.exe'
        for env in ('ProgramFiles', 'ProgramFiles(x86)', 'LOCALAPPDATA')
    ]
    chrome = next((p for p in candidates if p.is_file()), None)
    if chrome is None:
        raise m.PocError('No se encontró Chrome instalado.')
    # Chrome usa el perfil habitual del usuario; no se usa depuración ni copia del perfil.
    subprocess.Popen([str(chrome), url], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def open_path(path):
    if sys.platform == 'win32':
        os.startfile(path)  # noqa: S606 - carpeta local elegida por el usuario
    else:
        subprocess.Popen(['xdg-open', str(path)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def pick_date(root, variable):
    try:
        chosen = date(*reversed([int(x) for x in variable.get().split('/')]))
    except Exception:
        chosen = m.now().date()
    popup = tk.Toplevel(root)
    popup.title('Elegir fecha')
    popup.resizable(False, False)
    popup.transient(root)
    popup.grab_set()
    current = [chosen.year, chosen.month]
    heading = ttk.Frame(popup, padding=10)
    heading.pack(fill='x')
    label = ttk.Label(heading, anchor='center', width=24)
    grid = ttk.Frame(popup, padding=10)
    grid.pack()

    def choose(day):
        variable.set(day.strftime('%d/%m/%Y'))
        popup.destroy()

    def render():
        label.configure(text=f'{MONTH_NAMES[current[1] - 1]} {current[0]}')
        for w in grid.winfo_children():
            w.destroy()
        for column, day in enumerate(['Lu', 'Ma', 'Mi', 'Ju', 'Vi', 'Sa', 'Do']):
            ttk.Label(grid, text=day, anchor='center').grid(row=0, column=column)
        for row, week in enumerate(calendar.Calendar().monthdatescalendar(*current), 1):
            for col, day in enumerate(week):
                b = ttk.Button(grid, text=str(day.day), width=4, command=lambda day=day: choose(day))
                b.grid(row=row, column=col, padx=1, pady=2)
                if day.month != current[1]:
                    b.configure(state='disabled')

    def shift(delta):
        number = current[0] * 12 + current[1] - 1 + delta
        current[:] = [number // 12, number % 12 + 1]
        render()

    ttk.Button(heading, text='‹', width=3, command=lambda: shift(-1)).pack(side='left')
    label.pack(side='left')
    ttk.Button(heading, text='›', width=3, command=lambda: shift(1)).pack(side='right')
    render()


def date_preset(label, today=None):
    today = today or m.now().date()
    if label == 'Este mes':
        start, end = today.replace(day=1), today
    elif label == 'Mes anterior':
        end = today.replace(day=1) - timedelta(days=1)
        start = end.replace(day=1)
    else:
        start, end = today - timedelta(days=29), today
    return start.strftime('%d/%m/%Y'), end.strftime('%d/%m/%Y')


def short_label(label):
    """Nombre de la consulta sin la pantalla ni las fechas (ya visibles en la lista y el resumen)."""
    parts = [x for x in label.split(' - ') if not re.fullmatch(r'\d{2}-\d{2}-\d{4} al \d{2}-\d{2}-\d{4}', x)]
    return ' · '.join(parts[1:] if len(parts) > 1 else parts)


SIDE = [
    ('Seguimiento', [('seguimiento', 'Informe seguimiento'), ('litigantes', 'Búsqueda litigantes')]),
    ('Informes', [('calendario_informes', 'Informes por vencer'), ('calendario_medidas', 'Medidas por vencer')]),
]
LABELS = {code: label for label, code in SCREENS.items()}


class Application:
    def __init__(self, root, start_worker=True, bridge_factory=Bridge, preferences_path=None):
        self.root = root
        self.events = queue.Queue()
        self.commands = queue.Queue()
        self.cancel = PauseGate()
        self.bridge_factory = bridge_factory
        self.bridge = bridge_factory(self.emit)
        self.prefs = Preferences(preferences_path)
        self.catalog = None
        self.connected = False
        self.busy = False
        self.quitting = False
        self.running = False
        self.showing_results = False
        self.last_folder = None
        self.planned = 0
        self.query_index = 0
        self.query_files = {}
        self.screen = tk.StringVar(value=next(iter(SCREENS)))
        today = m.now().strftime('%d/%m/%Y')
        self.from_date = tk.StringVar(value=today)
        self.to_date = tk.StringVar(value=today)
        self.keep_filters = tk.BooleanVar(value=False)
        self.empty_pdf = tk.BooleanVar(value=self.prefs.data['pdf_sin_registros'])
        self.year = tk.StringVar()
        self.dest = tk.StringVar(value=self.prefs.data['destino'])
        self.code = tk.StringVar(value=self.bridge.connection_code)
        self.status = tk.StringVar()
        self.summary = ''

        self.window = ui.Window(root, 'SITFA Lite', 'Descargar', self.primary, size='1000x680', minsize=(820, 560))
        root.title(f'SITFA Lite {VERSION}')
        root.protocol('WM_DELETE_WINDOW', self.close_app)
        top = self.window.top
        top.add('Abrir SITFA en Chrome', lambda: self.enqueue('open'))
        top.add('Instalar extensión…', self.setup)
        top.add('Copiar código de conexión', self.copy_code)
        top.add('Desconectar', lambda: self.enqueue('disconnect'))
        top.add('Carpeta de descarga…', self.browse, separator=True)
        top.add('Abrir carpeta del último lote', self.open_folder)
        top.add_check('Conservar los otros filtros preparados en SITFA', self.keep_filters, self.refresh)
        top.add(f'Versión {VERSION}', lambda: None, separator=True)
        self.folder_button = self.window.footer.add('Abrir carpeta', self.open_folder)
        self.folder_button.pack_forget()
        self.download_button = self.window.footer.primary
        self.status.trace_add('write', self._status_changed)

        self.side, pane = ui.with_sidebar(self.window, SIDE, self.change_screen)
        self.pane = pane
        # Sin conexión: una sola acción.
        self.connect_view = ttk.Frame(pane)
        ttk.Label(self.connect_view, text='Conecta tu Chrome', font=ui.FONT_BOLD).pack(anchor='w')
        self.open_button = ttk.Button(
            self.connect_view, text='Abrir SITFA', style='Primary.TButton', command=lambda: self.enqueue('open')
        )
        self.open_button.pack(anchor='w', pady=(12, 12))
        line = ttk.Frame(self.connect_view)
        line.pack(anchor='w')
        ttk.Entry(line, textvariable=self.code, state='readonly', width=44).pack(side='left')
        self.copy_button = ttk.Button(line, text='Copiar', command=self.copy_code)
        self.copy_button.pack(side='left', padx=(6, 0))
        # Conectado: filtros de la consulta elegida.
        self.form = ui.Form(pane, key_width=12)
        f = self.form
        self.tribunals = f.row('Tribunales', lambda p: ui.Chips(p, command=self.refresh))
        self.tabs = f.row('Pestañas', lambda p: ui.Chips(p, command=self.refresh))
        self.informes = f.row('  Informes', lambda p: ui.Chips(p, command=self.refresh, style='Chip.Toolbutton'),
                              name='informes')
        self.medidas = f.row('  Medidas', lambda p: ui.Chips(p, command=self.refresh), name='medidas')
        self.modalities = f.row('Modalidad', lambda p: ui.Chips(p, command=self.refresh))
        self.states = f.row('Estado', lambda p: ui.Chips(p, command=self.refresh))
        self.months = f.row('Mes', lambda p: ui.Chips(p, command=self.refresh))
        self.year_combo = f.row('Año', lambda p: ttk.Combobox(p, textvariable=self.year, width=8, state='readonly'))
        self.year_combo.bind('<<ComboboxSelected>>', lambda e: self.refresh())
        self.period = f.row('Período', lambda p: ui.Chips(p, [(x, x) for x in PERIODS], multi=False,
                                                         command=self.change_period))
        self.range = f.row('', self._range_row, name='rango')
        self.include = f.row('Incluir', lambda p: ui.Chips(p, [('pdf', 'PDF si no hay registros')],
                                                         command=self.toggle_empty_pdf))
        self.include.set(['pdf'] if self.empty_pdf.get() else [])
        self.results = ui.ProgressList(pane, on_open=self.open_result, height=14)
        self.screen_radios = list(self.side.buttons.values())
        self.change_screen(self.side.get(), initial=True)
        if start_worker:
            threading.Thread(target=self.worker, daemon=True).start()
            root.after(100, self.poll)

    def _range_row(self, parent):
        frame = ttk.Frame(parent)
        self.date_widgets = []
        for label, var in (('Desde', self.from_date), ('Hasta', self.to_date)):
            ttk.Label(frame, text=label, style='Muted.TLabel').pack(side='left', padx=(0, 4))
            entry = ttk.Entry(frame, textvariable=var, width=11)
            entry.pack(side='left')
            entry.bind('<KeyRelease>', lambda e: self.refresh())
            button = ttk.Button(frame, text='Calendario', command=lambda var=var: self.pick(var))
            button.pack(side='left', padx=(2, 14))
            self.date_widgets += [entry, button]
        return frame

    def _status_changed(self, *_):
        text = self.status.get()
        self.window.banner.show(text) if text else self.window.banner.hide()

    # --- selección -------------------------------------------------------------------------------
    @property
    def screen_code(self):
        return SCREENS[self.screen.get()]

    @property
    def use_dates(self):
        return self.period.get() != ('Sin filtro',)

    def change_screen(self, code=None, initial=False):
        if code:
            self.screen.set(LABELS[code])
        self.catalog = None
        for chips in (self.tribunals, self.tabs, self.informes, self.medidas, self.modalities, self.states, self.months):
            chips.set_options([])
        screen = self.screen_code
        shown = {
            'seguimiento': {'Tribunales', 'Pestañas', 'Modalidad', 'Período', 'Incluir'},
            'litigantes': {'Tribunales', 'Estado', 'Período', 'Incluir'},
        }.get(screen, {'Tribunales', 'Mes', 'Año', 'Incluir'})
        for name in self.form.rows:
            self.form.show(name, name in shown)
        if not initial and self.connected and not self.busy:
            self.load_catalog()
        self.refresh()

    def change_period(self):
        self.refresh()

    def apply_catalog(self, catalog):
        if not isinstance(catalog, dict) or catalog.get('pantalla') != self.screen_code:
            raise m.PocError('SITFA devolvió otra pantalla. Elige de nuevo la consulta.')
        self.catalog = catalog
        chosen = catalog.get('seleccion') or {}
        tribunal = chosen.get('tribunal')
        self.tribunals.set_options(list(catalog['tribunales'].items()),
                                   (tribunal,) if tribunal in catalog['tribunales'] else ())
        screen = self.screen_code
        if screen == 'seguimiento':
            tabs = [t for t in TAB_ORDER if t in catalog.get('pestanas', [])]
            self.tabs.set_options([(t, t) for t in tabs], [t for t in ('Espera', 'Cumplimiento') if t in tabs])
            modalities = [(k, MODALITIES[k]) for k in catalog.get('modalidades', {}) if k in MODALITIES]
            self.modalities.set_options(modalities, [k for k, _ in modalities])
            informes = list(catalog.get('informes', {}).items())
            self.informes.set_options(informes, [k for k, _ in informes[:1]])
            self.medidas.set_options([(ALL_MEASURES, 'Todas las medidas'), *catalog.get('medidas', {}).items()],
                                     [ALL_MEASURES])
            if chosen.get('desde') and chosen.get('hasta'):
                self.from_date.set(chosen['desde'])
                self.to_date.set(chosen['hasta'])
        elif screen == 'litigantes':
            states = catalog.get('estados', {})
            self.states.set_options(list(states.items()), [chosen.get('estado')] if chosen.get('estado') in states else [])
        else:
            months = catalog.get('meses', {})
            self.months.set_options(list(months.items()), [chosen.get('mes')] if chosen.get('mes') in months else [])
            years = catalog.get('anios', {})
            self.year_combo.configure(values=list(years.values()))
            self.year.set(years.get(chosen.get('anio'), next(iter(years.values()), '')))
        self.status.set('')
        self.refresh()

    def batch(self):
        if not self.catalog:
            raise m.PocError('Cargando las opciones de SITFA…')
        screen = self.screen_code
        dates = None
        if screen in ('seguimiento', 'litigantes') and self.use_dates:
            dates = Dates(self.from_date.get().strip(), self.to_date.get().strip())
        keep = self.keep_filters.get() and screen in ('seguimiento', 'litigantes')
        tribunals = self.tribunals.get()
        if screen == 'seguimiento':
            tabs = self.tabs.get()
            return Batch(
                screen,
                tribunals,
                modalities=self.modalities.get(),
                tabs=tabs,
                informes=self.informes.get() if 'Informes' in tabs else (),
                medidas=self.medidas.get() if 'Cumplimiento' in tabs else (),
                dates=dates,
                keep_filters=keep,
            )
        if screen == 'litigantes':
            return Batch(screen, tribunals, states=self.states.get(), dates=dates, keep_filters=keep)
        year = next((k for k, v in self.catalog.get('anios', {}).items() if v == self.year.get()), None)
        return Batch(screen, tribunals, months=self.months.get(), year=year)

    def toggle_empty_pdf(self):
        self.empty_pdf.set('pdf' in self.include.get())
        self.prefs.data['pdf_sin_registros'] = self.empty_pdf.get()
        self.prefs.save()
        self.refresh()

    def describe(self, plan):
        parts = [f'{len(plan)} consulta{"s" if len(plan) != 1 else ""}']
        tribunals = len(self.tribunals.get())
        parts.append(f'{tribunals} tribunal{"es" if tribunals != 1 else ""}')
        if self.screen_code in ('seguimiento', 'litigantes'):
            parts.append('sin filtro de fecha' if not self.use_dates else f'{self.from_date.get()} – {self.to_date.get()}')
        return ' · '.join(parts)

    def show(self, view):
        for widget in (self.connect_view, self.form, self.results):
            widget.pack_forget()
        {'connect': self.connect_view, 'form': self.form, 'results': self.results}[view].pack(
            fill='both' if view == 'results' else 'x', expand=view == 'results', anchor='nw'
        )

    def refresh(self):
        idle = not self.busy and not self.quitting
        ready = idle and self.connected and self.catalog is not None
        if self.running or self.showing_results:
            self.show('results')
        elif self.connected:
            self.show('form')
        else:
            self.show('connect')
        self.window.top.set_state('Chrome conectado' if self.connected else 'Sin conexión',
                                  'ok' if self.connected else 'muted')
        self.side.enable(idle)
        if self.screen_code == 'seguimiento':
            tabs = self.tabs.get()
            self.form.show('informes', 'Informes' in tabs)
            self.form.show('medidas', 'Cumplimiento' in tabs)
        if self.screen_code in ('seguimiento', 'litigantes'):
            self.form.show('rango', self.period.get() == ('Por fecha',))
        for chips in (self.tribunals, self.tabs, self.informes, self.medidas, self.modalities, self.states,
                      self.months, self.period, self.include):
            chips.enable(ready)
        self.year_combo.configure(state='readonly' if ready else 'disabled')
        footer = self.window.footer
        if self.running:
            footer.set(self.summary, enabled=not self.quitting, primary_text='Detener')
            return
        if self.showing_results:
            self.folder_button.pack(side='right', padx=(0, 8))
            footer.set(self.summary, enabled=idle, primary_text='Nuevo lote')
            return
        self.folder_button.pack_forget()
        if not self.connected:
            footer.set('', enabled=False, primary_text='Descargar')
            return
        try:
            plan = self.batch().plan(self.catalog)
            footer.set(self.describe(plan), enabled=ready, primary_text='Descargar')
        except m.PocError as exc:
            footer.set(str(exc), enabled=False, primary_text='Descargar')

    # --- acciones ----------------------------------------------------------------------------------
    def emit(self, name, value):
        self.events.put((name, value))

    def enqueue(self, command, arg=None):
        if self.busy and command not in ('exit',):
            return
        self.busy = command not in ('open',)
        self.refresh()
        self.commands.put((command, arg))

    def load_catalog(self):
        self.enqueue('catalog', self.screen_code)

    def pick(self, var):
        pick_date(self.root, var)
        self.root.after(300, self.refresh)

    def quick_dates(self, label):
        """Atajo programático (sin botón): fija Desde/Hasta y activa el filtro por fecha."""
        start, end = date_preset(label)
        self.from_date.set(start)
        self.to_date.set(end)
        self.period.set(['Por fecha'])
        self.change_period()

    def setup(self):
        directory = extension_dir()
        self.root.clipboard_clear()
        self.root.clipboard_append(str(directory))
        self.root.update()
        try:
            open_chrome('chrome://extensions/')
        except m.PocError as exc:
            self.status.set(str(exc))
        messagebox.showinfo(
            'Instalar la extensión (una sola vez)',
            '1. En chrome://extensions activa «Modo de desarrollador» (arriba a la derecha).\n'
            '2. Pulsa «Cargar descomprimida».\n'
            '3. Pega la ruta copiada al portapapeles y elige la carpeta:\n\n'
            f'{directory}\n\n'
            '4. Fija el icono «SITFA Lite» en la barra de Chrome.',
            parent=self.root,
        )

    def copy_code(self):
        self.root.clipboard_clear()
        self.root.clipboard_append(self.code.get())
        self.root.update()
        self.status.set('Código copiado. Pégalo en la pestaña «Conectar con SITFA Lite» de Chrome.')

    def browse(self):
        chosen = filedialog.askdirectory(initialdir=self.dest.get() or str(Path.home()), parent=self.root)
        if chosen:
            self.dest.set(chosen)
            self.prefs.data['destino'] = chosen
            self.prefs.save()

    def primary(self):
        if self.running:
            self.request_cancel()
        elif self.showing_results:
            self.showing_results = False
            self.status.set('')
            self.refresh()
        else:
            self.start()

    def start(self):
        try:
            batch = self.batch()
            plan = batch.plan(self.catalog)
            destination = Path(self.dest.get())
            destination.mkdir(parents=True, exist_ok=True)
        except m.PocError as exc:
            self.window.banner.show(str(exc), bad=True)
            return
        except OSError:
            self.window.banner.show('No se pudo crear la carpeta de descarga. Elige otra en Más › Carpeta de descarga.',
                                    bad=True)
            return
        self.results.clear()
        self.results.bar.configure(maximum=max(1, len(plan)))
        self.query_files = {}
        self.cancel.clear()
        self.running = True
        self.summary = f'Consultando 0 de {len(plan)}'
        self.status.set('')
        self.enqueue('download', (batch, destination))

    def request_cancel(self):
        self.cancel.set()
        self.summary = 'Deteniendo al terminar la página en curso…'
        self.refresh()

    def open_folder(self):
        if self.last_folder:
            open_path(self.last_folder)

    def open_result(self, key):
        files = self.query_files.get(int(key), [])
        if files and self.last_folder and (self.last_folder / files[0]).is_file():
            open_path(self.last_folder / files[0])

    def close_app(self):
        if self.busy and not messagebox.askyesno(
            'Salir', 'Hay una descarga en curso. ¿Cancelarla y salir? Los archivos ya verificados se conservan.'
        ):
            return
        self.quitting = True
        self.cancel.set()
        self.commands.put(('exit', None))
        self.root.after(3000, self.root.destroy)

    # --- hilo de trabajo -------------------------------------------------------------------------
    def worker(self):
        while True:
            command, arg = self.commands.get()
            try:
                if command == 'open':
                    open_chrome(m.START)
                    self.emit('status', 'SITFA abierto. Inicia sesión y pulsa el icono de la extensión SITFA Lite.')
                elif command == 'catalog':
                    self.emit('catalog', self.bridge.call('catalog', {'screen': arg}))
                elif command == 'download':
                    verify_extension(self.bridge)
                    BatchRunner(
                        self.bridge,
                        self.catalog,
                        self.emit,
                        self.cancel,
                        pause=self.prefs.data['pausa'],
                        evidencia=self.prefs.data['pdf_sin_registros'],
                    ).run(*arg)
                elif command in ('disconnect', 'exit'):
                    if self.bridge.connected:
                        try:
                            self.bridge.call('unlock', timeout=8)
                        except Exception:
                            pass
                    self.bridge.close()
                    if command == 'exit':
                        self.emit('exit', None)
                        return
                    self.bridge = self.bridge_factory(self.emit)
                    self.emit('disconnected', self.bridge.connection_code)
            except m.PocError as exc:
                self.emit('error', str(exc))
            except Exception:
                self.emit('error', 'No se pudo completar la operación. Revisa la conexión, SITFA y la carpeta.')
            finally:
                self.emit('idle', command)

    def process_event(self, name, value):
        if name == 'connected':
            self.connected = True
            self.status.set('')
            if not self.busy:
                self.load_catalog()
        elif name == 'disconnected':
            self.connected = False
            self.catalog = None
            self.code.set(value)
            self.change_screen(initial=True)
        elif name == 'status':
            self.status.set(value)
        elif name == 'catalog':
            try:
                self.apply_catalog(value)
            except (m.PocError, KeyError, TypeError) as exc:
                self.catalog = None
                self.window.banner.show(
                    str(exc) if isinstance(exc, m.PocError) else 'SITFA entregó opciones incompletas.', bad=True
                )
        elif name == 'folder':
            self.last_folder = Path(value)
        elif name == 'plan':
            self.planned = value
        elif name == 'query':
            self.query_index = value['index']
            self.summary = f'Consultando {value["index"]} de {value["total"]}'
            self.results.set(value['index'], 'run', name=short_label(value['label']))
        elif name == 'total':
            self.results.set(self.query_index, 'run', f'{value} página{"s" if value != 1 else ""}')
        elif name == 'progress':
            self.query_files.setdefault(self.query_index, []).append(value['archivo'])
            self.results.set(self.query_index, 'run', f'página {value.get("pagina", "")} · {value["registros"]}')
        elif name == 'evidence':
            self.query_files.setdefault(self.query_index, []).append(value['archivo'])
        elif name == 'query_done':
            empty = value['estado'] == 'SIN_RESULTADOS'
            detail = ('sin registros' + (' · sin PDF' if value.get('evidencia_omitida') else ' · PDF')
                      if empty else f'{value["registros"]} registros')
            self.results.set(self.query_index, 'skip' if empty else 'ok', detail)
        elif name == 'warning':
            self.status.set(value)
        elif name == 'done':
            self.status.set('')
            self.running = False
            self.showing_results = True
            self.summary = (
                f'{value["queries"]} consultas · {value["records"]} registros · {value["files"]} XLS · '
                f'{value["pdfs"]} PDF · {value.get("empty", 0)} sin registros'
            )
            self.root.bell()
        elif name == 'error':
            if self.running:
                self.running = False
                self.showing_results = True
                self.summary = 'Detenido'
                if self.query_index:
                    self.results.set(self.query_index, 'bad', 'detenida')
            if not self.quitting:
                self.window.banner.show(value, bad=True)
        elif name == 'idle':
            self.busy = False
            if value == 'download' and self.running:
                self.running = False
                self.showing_results = True
        elif name == 'exit':
            self.root.destroy()
            return False
        self.refresh()
        return True

    def poll(self):
        try:
            while True:
                name, value = self.events.get_nowait()
                if self.process_event(name, value) is False:
                    return
        except queue.Empty:
            pass
        linked = self.bridge.connected
        if linked != self.connected and not self.busy:
            self.connected = linked
            if linked:
                self.load_catalog()
            elif self.catalog is not None:
                self.change_screen(initial=True)
                self.window.banner.show('Se perdió la conexión con Chrome. Ábrela de nuevo desde el icono SITFA Lite.',
                                        bad=True)
            self.refresh()
        self.root.after(400, self.poll)


def main():
    root = tk.Tk()
    Application(root)
    root.mainloop()


if __name__ == '__main__':
    main()
