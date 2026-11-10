"""SITFA Lite: descargas XLS de RUS (Seguimiento, Busqueda Lit, Informes y Medidas por vencer)."""

import json
import os
from pathlib import Path
import queue
import re
import subprocess
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

PERIODS = ('Sin filtro', 'Por fecha')


def data_directory():
    base = os.environ.get('LOCALAPPDATA') or str(Path.home())
    return Path(base) / 'SITFA_Lite'


class Preferences:
    def __init__(self, path=None):
        self.path = Path(path) if path else data_directory() / 'preferencias.json'
        self.data = {
            'destino': str(Path.home() / 'Documents' / 'SITFA Lite'),
            'pausa': 1.0,
            'pdf_sin_registros': True,
            'selecciones': {},
        }
        try:
            stored = json.loads(self.path.read_text(encoding='utf-8'))
            if isinstance(stored.get('destino'), str):
                self.data['destino'] = stored['destino']
            if isinstance(stored.get('pausa'), (int, float)) and 0 <= stored['pausa'] <= 30:
                self.data['pausa'] = float(stored['pausa'])
            if isinstance(stored.get('pdf_sin_registros'), bool):
                self.data['pdf_sin_registros'] = stored['pdf_sin_registros']
            if isinstance(stored.get('selecciones'), dict):
                self.data['selecciones'] = stored['selecciones']
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


def short_label(label):
    """Nombre de la consulta sin la pantalla ni las fechas (ya visibles en la lista y el resumen)."""
    parts = [x for x in label.split(' - ') if not re.fullmatch(r'\d{2}-\d{2}-\d{4} al \d{2}-\d{2}-\d{4}', x)]
    return ' · '.join(parts[1:] if len(parts) > 1 else parts)


SIDE = [
    ('Seguimiento', [('seguimiento', 'Informe seguimiento'), ('litigantes', 'Búsqueda litigantes')]),
    ('Informes', [('calendario_informes', 'Informes por vencer'), ('calendario_medidas', 'Medidas por vencer')]),
]
LABELS = {code: label for label, code in SCREENS.items()}


DOCS = [
    ('Manual de uso (PDF)', 'SITFA_Lite_Manual_Uso.pdf'),
    ('Manual de instalación (PDF)', 'SITFA_Lite_Manual_Instalacion.pdf'),
    ('Video: instalación y uso', 'SITFA_Lite_Video_Instalacion_y_Uso.mp4'),
    ('Video: descargar Seguimiento de un tribunal', 'SITFA_Lite_Video_Descargar_Seguimiento.mp4'),
]
# Filas que se recuerdan por consulta (preferencias «selecciones»).
REMEMBERED = ('tribunals', 'tabs', 'informes', 'medidas', 'modalities', 'states', 'months', 'period')


class OnlyThese:
    """El mismo lote, limitado a algunas consultas (reintentar las que faltan)."""

    def __init__(self, batch, selections):
        self._batch, self._selections = batch, list(selections)

    def plan(self, catalog):
        return list(self._selections)

    def __getattr__(self, name):
        return getattr(self._batch, name)


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
        self.last_batch = None
        self.last_plan = []
        self.query_index = 0
        self.query_files = {}
        self.steps_done = set()
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
        self.window.title = f'SITFA Lite {VERSION}'
        root.title(self.window.title)
        root.protocol('WM_DELETE_WINDOW', self.close_app)
        self.window.on_escape = lambda: self.request_cancel() if self.running else None
        top = self.window.top
        top.add('Abrir SITFA en Chrome', lambda: self.enqueue('open'), busy_lock=True)
        top.add('Instalar extensión…', self.setup)
        top.add('Copiar código de conexión', self.copy_code)
        top.add('Desconectar', lambda: self.enqueue('disconnect'), busy_lock=True)
        top.add('Carpeta de descarga…', self.browse, separator=True, busy_lock=True)
        top.add('Abrir carpeta del último lote', self.open_folder)
        top.add_check(
            'Conservar los otros filtros preparados en SITFA', self.keep_filters, self.refresh, busy_lock=True
        )
        self.window.add_docs(DOCS, anchor=__file__)
        top.add_note(f'Versión {VERSION}')
        footer = self.window.footer
        self.folder_link = footer.add_link('', self.browse)
        self.retry_button = footer.add('Reintentar las que faltan', self.retry)
        self.retry_button.pack_forget()
        self.folder_button = footer.add('Abrir carpeta', self.open_folder)
        self.folder_button.pack_forget()
        self.download_button = footer.primary
        self.status.trace_add('write', self._status_changed)
        self.dest.trace_add('write', lambda *_: self._show_destination())
        self._show_destination()

        self.side, pane = ui.with_sidebar(self.window, SIDE, self.change_screen)
        self.connect_view = self._connect_view(pane)
        # Conectado: filtros de la consulta elegida.
        self.form = ui.Form(pane, key_width=12)
        f = self.form
        self.tribunals = f.row('Tribunales', lambda p: ui.Chips(p, command=self.refresh, all_label='Todos'))
        self.tabs = f.row('Pestañas', lambda p: ui.Chips(p, command=self.refresh))
        self.informes = f.row('  Informes', lambda p: ui.Chips(p, command=self.refresh), name='informes')
        self.medidas = f.row('  Medidas', lambda p: ui.Chips(p, command=self.refresh), name='medidas')
        self.modalities = f.row('Modalidad', lambda p: ui.Chips(p, command=self.refresh))
        self.states = f.row('Estado', lambda p: ui.Chips(p, command=self.refresh, all_label='Todos'))
        self.months = f.row('Mes', lambda p: ui.Chips(p, command=self.refresh, all_label='Todos'))
        self.year_combo = f.row('Año', lambda p: ttk.Combobox(p, textvariable=self.year, width=8, state='readonly'))
        self.year_combo.bind('<<ComboboxSelected>>', lambda e: self.refresh())
        self.period = f.row(
            'Período', lambda p: ui.Chips(p, [(x, x) for x in PERIODS], multi=False, command=self.refresh)
        )
        f.row('', self._range_row, name='rango')
        self.include = f.row(
            'Incluir', lambda p: ui.Chips(p, [('pdf', 'PDF si no hay registros')], command=self.toggle_empty_pdf)
        )
        self.include.set(['pdf'] if self.empty_pdf.get() else [])
        self.results = ui.ProgressList(
            pane, on_open=self.open_result, height=14, hint='Doble clic en una consulta para abrir su archivo.'
        )
        self.change_screen(self.side.get(), initial=True)
        if start_worker:
            threading.Thread(target=self.worker, daemon=True).start()
            root.after(100, self.poll)

    # --- vistas ----------------------------------------------------------------------------------------------------
    def _connect_view(self, pane):
        """Conexión guiada en tres pasos; cada uno muestra ✓ al cumplirse."""
        view = ttk.Frame(pane)
        ttk.Label(view, text='Conecta tu Chrome', font=ui.FONT_BOLD).pack(anchor='w', pady=(0, 10))
        self.step_marks = {}

        def step(number, text):
            row = ttk.Frame(view)
            row.pack(anchor='w', fill='x', pady=(0, 10))
            mark = ttk.Label(row, text=f'{number}', width=3, style='Step.TLabel')
            mark.pack(side='left')
            ttk.Label(row, text=text).pack(side='left')
            body = ttk.Frame(view)
            body.pack(anchor='w', padx=(28, 0), pady=(0, 12))
            self.step_marks[number] = mark
            return body

        first = step(1, 'Abre SITFA en tu Chrome e inicia sesión.')
        self.open_button = ttk.Button(
            first, text='Abrir SITFA', style='Primary.TButton', command=lambda: self.enqueue('open')
        )
        self.open_button.pack(side='left')
        second = step(2, 'Copia el código de conexión.')
        ttk.Entry(second, textvariable=self.code, state='readonly', width=44).pack(side='left')
        self.copy_button = ttk.Button(second, text='Copiar', command=self.copy_code)
        self.copy_button.pack(side='left', padx=(6, 0))
        step(3, 'En la pestaña de SITFA pulsa el ícono SITFA Lite, pega el código y pulsa «Conectar».')
        ttk.Separator(view).pack(fill='x', pady=(4, 10))
        first_time = ttk.Frame(view)
        first_time.pack(anchor='w')
        ttk.Label(
            first_time, text='¿Primera vez? La extensión de Chrome se instala una sola vez.', style='Muted.TLabel'
        ).pack(side='left')
        self.setup_button = ttk.Button(first_time, text='Instalar extensión…', command=self.setup)
        self.setup_button.pack(side='left', padx=(8, 0))
        return view

    def _mark_step(self, number):
        self.steps_done.add(number)
        self.step_marks[number].configure(text='✓', style='Ok.TLabel')

    def _range_row(self, parent):
        frame = ttk.Frame(parent)
        self.from_field = ui.DateField(frame, self.from_date, self.refresh, label='Desde')
        self.from_field.pack(side='left', padx=(0, 14))
        self.to_field = ui.DateField(frame, self.to_date, self.refresh, label='Hasta')
        self.to_field.pack(side='left', padx=(0, 14))
        self.date_error = ttk.Label(frame, style='Bad.TLabel')
        self.date_error.pack(side='left')
        return frame

    def _status_changed(self, *_):
        text = self.status.get()
        self.window.banner.show(text) if text else self.window.banner.hide()

    def _show_destination(self):
        self.folder_link.configure(text='Carpeta: ' + ui.short_path(self.dest.get()))

    # --- selección ---------------------------------------------------------------------------------------------------
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
        for chips in (
            self.tribunals,
            self.tabs,
            self.informes,
            self.medidas,
            self.modalities,
            self.states,
            self.months,
        ):
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

    def apply_catalog(self, catalog):
        if not isinstance(catalog, dict) or catalog.get('pantalla') != self.screen_code:
            raise m.PocError('SITFA devolvió otra pantalla. Elige de nuevo la consulta.')
        self.catalog = catalog
        chosen = catalog.get('seleccion') or {}
        tribunal = chosen.get('tribunal')
        self.tribunals.set_options(
            list(catalog['tribunales'].items()), (tribunal,) if tribunal in catalog['tribunales'] else ()
        )
        screen = self.screen_code
        if screen == 'seguimiento':
            tabs = [t for t in TAB_ORDER if t in catalog.get('pestanas', [])]
            self.tabs.set_options([(t, t) for t in tabs], [t for t in ('Espera', 'Cumplimiento') if t in tabs])
            modalities = [(k, MODALITIES[k]) for k in catalog.get('modalidades', {}) if k in MODALITIES]
            self.modalities.set_options(modalities, [k for k, _ in modalities])
            informes = list(catalog.get('informes', {}).items())
            self.informes.set_options(informes, [k for k, _ in informes[:1]])
            self.medidas.set_options(
                [(ALL_MEASURES, 'Todas las medidas'), *catalog.get('medidas', {}).items()], [ALL_MEASURES]
            )
            if chosen.get('desde') and chosen.get('hasta'):
                self.from_date.set(chosen['desde'])
                self.to_date.set(chosen['hasta'])
        elif screen == 'litigantes':
            states = catalog.get('estados', {})
            self.states.set_options(
                list(states.items()), [chosen.get('estado')] if chosen.get('estado') in states else []
            )
        else:
            months = catalog.get('meses', {})
            self.months.set_options(list(months.items()), [chosen.get('mes')] if chosen.get('mes') in months else [])
            years = catalog.get('anios', {})
            self.year_combo.configure(values=list(years.values()))
            self.year.set(years.get(chosen.get('anio'), next(iter(years.values()), '')))
        self.restore_selection()
        self.status.set('')
        self.refresh()

    def remember_selection(self):
        """Guarda lo marcado en esta consulta para proponerlo la próxima vez."""
        saved = {name: list(getattr(self, name).get()) for name in REMEMBERED}
        self.prefs.data.setdefault('selecciones', {})[self.screen_code] = saved
        self.prefs.save()

    def restore_selection(self):
        """Vuelve a marcar lo de la última descarga de esta consulta, solo con opciones que SITFA siga ofreciendo."""
        saved = self.prefs.data.get('selecciones', {}).get(self.screen_code)
        if not isinstance(saved, dict):
            return
        for name in REMEMBERED:
            chips = getattr(self, name)
            values = [v for v in saved.get(name, []) if v in chips.buttons]
            if values:
                chips.set(values)

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
            parts.append(
                'sin filtro de fecha' if not self.use_dates else f'{self.from_date.get()} – {self.to_date.get()}'
            )
        return ' · '.join(parts)

    def check_dates(self):
        """Marca en el lugar una fecha mal escrita o un rango invertido; devuelve el motivo o ''."""
        if not (self.screen_code in ('seguimiento', 'litigantes') and self.use_dates):
            reason, bad = '', ()
        else:
            parsed, bad = {}, []
            for field, var in ((self.from_field, self.from_date), (self.to_field, self.to_date)):
                try:
                    parsed[field] = ui.parse_date(var.get())
                except ValueError:
                    bad.append(field)
            if bad:
                reason = 'Fecha DD/MM/AAAA'
            elif parsed[self.from_field] > parsed[self.to_field]:
                bad, reason = [self.from_field, self.to_field], '«Desde» es posterior a «Hasta»'
            else:
                reason = ''
        for field in (self.from_field, self.to_field):
            field.error(field in bad)
        self.date_error.configure(text=reason)
        return reason

    def show(self, view):
        # Solo cambia la vista si hace falta: volver a empaquetar en cada evento hace parpadear la ventana.
        if getattr(self, '_view', None) == view:
            return
        self._view = view
        for widget in (self.connect_view, self.form, self.results):
            widget.pack_forget()
        {'connect': self.connect_view, 'form': self.form, 'results': self.results}[view].pack(
            fill='both' if view == 'results' else 'x', expand=view == 'results', anchor='nw'
        )

    def pending(self):
        """Consultas del último lote que no quedaron listas (detenidas, con error o sin empezar)."""
        if not self.last_batch or self.running:
            return []
        kinds = self.results.kinds()
        return [s for i, s in enumerate(self.last_plan, 1) if kinds.get(str(i), 'wait') not in ('ok', 'skip')]

    def refresh(self):
        idle = not self.busy and not self.quitting
        ready = idle and self.connected and self.catalog is not None
        if self.running or self.showing_results:
            self.show('results')
        elif self.connected:
            self.show('form')
        else:
            self.show('connect')
        self.window.top.set_state(
            'Chrome conectado' if self.connected else 'Sin conexión', 'ok' if self.connected else 'muted'
        )
        self.window.top.lock(not idle)
        self.side.enable(idle)
        if self.screen_code == 'seguimiento':
            tabs = self.tabs.get()
            self.form.show('informes', 'Informes' in tabs)
            self.form.show('medidas', 'Cumplimiento' in tabs)
        if self.screen_code in ('seguimiento', 'litigantes'):
            self.form.show('rango', self.period.get() == ('Por fecha',))
        for chips in (
            self.tribunals,
            self.tabs,
            self.informes,
            self.medidas,
            self.modalities,
            self.states,
            self.months,
            self.period,
            self.include,
        ):
            chips.enable(ready)
        for field in (self.from_field, self.to_field):
            field.enable(ready)
        self.year_combo.configure(state='readonly' if ready else 'disabled')
        self.setup_button.state(['!disabled'] if idle else ['disabled'])
        footer = self.window.footer
        self.folder_link.state(['!disabled'] if idle else ['disabled'])
        if self.running:
            footer.set(self.summary, enabled=not self.quitting, primary_text='Detener')
            return
        if self.showing_results:
            self.folder_button.pack(side='right', padx=(0, 8))
            retry = self.pending() if self.catalog and self.last_batch.screen == self.screen_code else []
            if retry and idle:
                self.retry_button.configure(text=f'Reintentar las que faltan ({len(retry)})')
                self.retry_button.pack(side='right', padx=(0, 8))
            else:
                self.retry_button.pack_forget()
            footer.set(self.summary, enabled=idle, primary_text='Nuevo lote')
            return
        self.folder_button.pack_forget()
        self.retry_button.pack_forget()
        if not self.connected:
            footer.set('', enabled=False, primary_text='Descargar')
            return
        reason = self.check_dates() if self.catalog else ''
        try:
            plan = self.batch().plan(self.catalog)
            footer.set(self.describe(plan), enabled=ready, primary_text='Descargar')
        except m.PocError as exc:
            footer.set(reason or str(exc), enabled=False, primary_text='Descargar')

    # --- acciones ------------------------------------------------------------------------------------------------------
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
            '2. Pulsa «Cargar extensión sin empaquetar».\n'
            '3. Pega la ruta copiada al portapapeles y elige la carpeta:\n\n'
            f'{directory}\n\n'
            '4. Fija el icono «SITFA Lite» en la barra de Chrome.',
            parent=self.root,
        )

    def copy_code(self):
        self.root.clipboard_clear()
        self.root.clipboard_append(self.code.get())
        self.root.update()
        self._mark_step(2)
        self.status.set('Código copiado. Pégalo en la pestaña «Conectar con SITFA Lite» de Chrome.')

    def browse(self):
        if self.busy:
            return
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

    def start(self, retry=None):
        if self.busy:
            # Una operación anterior (p. ej. cargar opciones) sigue en curso: no se marca un lote que no partió.
            self.window.banner.show('Espera a que termine la operación en curso y vuelve a pulsar Descargar.')
            return
        try:
            batch = OnlyThese(self.last_batch, retry) if retry else self.batch()
            plan = batch.plan(self.catalog)
            destination = Path(self.dest.get())
            destination.mkdir(parents=True, exist_ok=True)
        except m.PocError as exc:
            self.window.banner.show(str(exc), bad=True)
            return
        except OSError:
            self.window.banner.show(
                'No se pudo crear la carpeta de descarga. Elige otra en Más › Carpeta de descarga.', bad=True
            )
            return
        if not retry:
            self.last_batch = batch
            self.remember_selection()
        self.last_plan = list(plan)
        self.results.clear()
        self.results.bar.configure(maximum=max(1, len(plan)))
        self.query_files = {}
        self.cancel.clear()
        self.running = True
        self.showing_results = False
        self.summary = f'Consultando 0 de {len(plan)}'
        self.status.set('')
        self.enqueue('download', (batch, destination))

    def retry(self):
        pending = self.pending()
        if pending and not self.busy:
            self.start(retry=pending)

    def request_cancel(self):
        self.cancel.set()
        self.summary = 'Deteniendo al terminar la página en curso…'
        self.refresh()

    def open_folder(self):
        # Sin lote en esta sesión se abre la carpeta de descarga; si ya no existe, se avisa en vez de fallar en silencio.
        folder = self.last_folder or Path(self.dest.get())
        if not ui.open_path(folder):
            self.window.banner.show('La carpeta todavía no existe: se crea con la primera descarga.')

    def open_result(self, key):
        files = self.query_files.get(int(key), [])
        if files and self.last_folder:
            ui.open_path(self.last_folder / files[0])

    def close_app(self):
        if self.busy and not messagebox.askyesno(
            'Salir', 'Hay una descarga en curso. ¿Cancelarla y salir? Los archivos ya verificados se conservan.'
        ):
            return
        self.quitting = True
        self.cancel.set()
        self.commands.put(('exit', None))
        self.root.after(3000, self.root.destroy)

    # --- hilo de trabajo -----------------------------------------------------------------------------------------------
    def worker(self):
        while True:
            command, arg = self.commands.get()
            try:
                if command == 'open':
                    open_chrome(m.START)
                    self.emit('opened', None)
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

    def _finish(self, summary):
        self.running = False
        self.showing_results = True
        self.summary = summary
        self.window.set_progress()

    def process_event(self, name, value):
        if name == 'connected':
            self.connected = True
            self.status.set('')
            if not self.busy:
                self.load_catalog()
        elif name == 'opened':
            self._mark_step(1)
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
        elif name == 'query':
            self.query_index = value['index']
            self.summary = f'Consultando {value["index"]} de {value["total"]}'
            self.window.set_progress(f'{value["index"]} de {value["total"]}')
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
            detail = (
                'sin registros' + (' · sin PDF' if value.get('evidencia_omitida') else ' · PDF')
                if empty
                else f'{value["registros"]} registros'
            )
            self.results.set(self.query_index, 'skip' if empty else 'ok', detail)
        elif name == 'warning':
            self.status.set(value)
        elif name == 'done':
            self.status.set('')
            self._finish(
                f'{value["queries"]} consultas · {value["records"]} registros · {value["files"]} XLS · '
                f'{value["pdfs"]} PDF · {value.get("empty", 0)} sin registros'
            )
            self.window.banner.show(
                'Descarga terminada.', kind='ok', action=self.open_folder, action_text='Abrir carpeta'
            )
            self.root.bell()
        elif name == 'error':
            stopped = self.cancel.is_set()
            if self.running:
                self._finish('Detenido')
                if self.query_index:
                    self.results.set(self.query_index, 'bad', 'detenida')
            if not self.quitting:
                # Detener a pedido no es un error: aviso neutro.
                self.window.banner.show(value, kind='info' if stopped else 'bad')
        elif name == 'idle':
            self.busy = False
            if value == 'download' and self.running:
                self._finish('Detenido')
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
                self.window.banner.show(
                    'Se perdió la conexión con Chrome. Ábrela de nuevo desde el icono SITFA Lite.', bad=True
                )
            self.refresh()
        self.root.after(400, self.poll)


def main():
    root = tk.Tk()
    Application(root)
    root.mainloop()


if __name__ == '__main__':
    main()
