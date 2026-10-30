"""SITFA Lite: descargas XLS de RUS (Seguimiento, Busqueda Lit, Informes y Medidas por vencer).

Sin paquetes consolidados, bitacoras, historias de la causa ni calendarios por ingreso.
"""

import calendar
from datetime import date, timedelta
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import threading
import tkinter as tk
from tkinter import filedialog, messagebox, ttk

import motor as m
from lotes import ALL_MEASURES, Batch, BatchRunner, Dates, MODALITIES, TAB_ORDER
from pausa import PauseGate
from perfiles import SCREENS
from puente import Bridge
from version import VERSION, extension_dir, verify_extension

MONTH_NAMES = (
    'Enero Febrero Marzo Abril Mayo Junio Julio Agosto Septiembre Octubre Noviembre Diciembre'
).split()
BG = '#f3f6fa'


def data_directory():
    base = os.environ.get('LOCALAPPDATA') or str(Path.home())
    return Path(base) / 'SITFA_Lite'


class Preferences:
    def __init__(self, path=None):
        self.path = Path(path) if path else data_directory() / 'preferencias.json'
        self.data = {'destino': str(Path.home() / 'Documents' / 'SITFA Lite'), 'pausa': 1.0}
        try:
            stored = json.loads(self.path.read_text(encoding='utf-8'))
            if isinstance(stored.get('destino'), str):
                self.data['destino'] = stored['destino']
            if isinstance(stored.get('pausa'), (int, float)) and 0 <= stored['pausa'] <= 30:
                self.data['pausa'] = float(stored['pausa'])
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


class CheckGroup(ttk.LabelFrame):
    """Casillas con Todas/Ninguna; columns > 1 reparte las opciones en una grilla."""

    def __init__(self, parent, text, columns=1, on_change=None):
        super().__init__(parent, text=text, padding=8)
        self.columns = columns
        self.on_change = on_change
        self.vars = {}
        self.buttons = []
        bar = ttk.Frame(self)
        bar.pack(fill='x')
        for label, value in (('Todas', True), ('Ninguna', False)):
            b = ttk.Button(bar, text=label, width=9, command=lambda value=value: self.set_all(value))
            b.pack(side='left', padx=(0, 6))
            self.buttons.append(b)
        self.body = ttk.Frame(self)
        self.body.pack(fill='both', expand=True, pady=(6, 0))
        self.checks = []

    def set_options(self, options, selected=()):
        for w in self.body.winfo_children():
            w.destroy()
        self.vars = {}
        self.checks = []
        for i, (code, label) in enumerate(options.items()):
            var = tk.BooleanVar(value=code in selected)
            check = ttk.Checkbutton(self.body, text=label, variable=var, command=self.changed)
            check.grid(row=i // self.columns, column=i % self.columns, sticky='w', padx=(0, 14), pady=1)
            self.vars[code] = var
            self.checks.append(check)

    def set_all(self, value):
        for var in self.vars.values():
            var.set(value)
        self.changed()

    def changed(self):
        if self.on_change:
            self.on_change()

    def selected(self):
        return tuple(code for code, var in self.vars.items() if var.get())

    def set_enabled(self, enabled):
        state = 'normal' if enabled else 'disabled'
        for w in (*self.buttons, *self.checks):
            w.configure(state=state)


class TribunalList(ttk.LabelFrame):
    """Lista con filtro y selección múltiple; muestra cuántos tribunales están marcados."""

    def __init__(self, parent, on_change=None):
        super().__init__(parent, text='Tribunales', padding=8)
        self.on_change = on_change
        self.options = {}
        self.chosen = set()
        self.filter = tk.StringVar()
        self.count = tk.StringVar(value='0 marcados')
        bar = ttk.Frame(self)
        bar.pack(fill='x')
        ttk.Label(bar, text='Buscar:').pack(side='left')
        self.entry = ttk.Entry(bar, textvariable=self.filter, width=22)
        self.entry.pack(side='left', padx=6)
        self.filter.trace_add('write', lambda *a: self.render())
        self.all_button = ttk.Button(bar, text='Todos', width=8, command=lambda: self.set_visible(True))
        self.all_button.pack(side='left', padx=(0, 6))
        self.none_button = ttk.Button(bar, text='Ninguno', width=8, command=lambda: self.set_visible(False))
        self.none_button.pack(side='left')
        ttk.Label(bar, textvariable=self.count).pack(side='right')
        frame = ttk.Frame(self)
        frame.pack(fill='both', expand=True, pady=(6, 0))
        self.listbox = tk.Listbox(frame, selectmode='multiple', height=8, exportselection=False, activestyle='none')
        self.listbox.pack(side='left', fill='both', expand=True)
        bar2 = ttk.Scrollbar(frame, orient='vertical', command=self.listbox.yview)
        bar2.pack(side='right', fill='y')
        self.listbox.configure(yscrollcommand=bar2.set)
        self.listbox.bind('<<ListboxSelect>>', self.clicked)
        self.visible = []

    def set_options(self, options, selected=()):
        self.options = dict(options)
        self.chosen = set(selected) & set(self.options)
        self.filter.set('')
        self.render()

    def render(self):
        text = m.normalized(self.filter.get())
        self.visible = [k for k, v in self.options.items() if text in m.normalized(v)]
        # Tk ignora insert/delete en un Listbox deshabilitado.
        state = self.listbox.cget('state')
        self.listbox.configure(state='normal')
        self.listbox.delete(0, 'end')
        for i, code in enumerate(self.visible):
            self.listbox.insert('end', self.options[code])
            if code in self.chosen:
                self.listbox.selection_set(i)
        self.listbox.configure(state=state)
        self.update_count()

    def clicked(self, _event=None):
        current = set(self.listbox.curselection())
        for i, code in enumerate(self.visible):
            if i in current:
                self.chosen.add(code)
            else:
                self.chosen.discard(code)
        self.update_count()

    def set_visible(self, value):
        for code in self.visible:
            (self.chosen.add if value else self.chosen.discard)(code)
        self.render()

    def update_count(self):
        self.count.set(f'{len(self.chosen)} de {len(self.options)} marcados')
        if self.on_change:
            self.on_change()

    def selected(self):
        return tuple(k for k in self.options if k in self.chosen)

    def set_enabled(self, enabled):
        state = 'normal' if enabled else 'disabled'
        for w in (self.entry, self.all_button, self.none_button, self.listbox):
            w.configure(state=state)


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
        self.last_folder = None
        self.planned = 0
        self.screen = tk.StringVar(value=next(iter(SCREENS)))
        self.use_dates = tk.BooleanVar(value=False)
        today = m.now().strftime('%d/%m/%Y')
        self.from_date = tk.StringVar(value=today)
        self.to_date = tk.StringVar(value=today)
        self.keep_filters = tk.BooleanVar(value=False)
        self.year = tk.StringVar()
        self.dest = tk.StringVar(value=self.prefs.data['destino'])
        self.code = tk.StringVar(value=self.bridge.connection_code)
        self.link = tk.StringVar(value='● Sin conexión')
        self.status = tk.StringVar(value='1) Abre SITFA en Chrome  2) Pulsa el icono de la extensión  3) Pega el código.')
        self.preview = tk.StringVar(value='Pulsa «Cargar opciones» para traer tribunales y filtros de tu sesión.')
        self.phase = tk.StringVar(value='Listo')

        root.title(f'SITFA Lite {VERSION} · Descargas RUS')
        root.geometry(f'1000x{min(980, max(680, root.winfo_screenheight() - 80))}')
        root.minsize(860, 600)
        root.configure(background=BG)
        root.protocol('WM_DELETE_WINDOW', self.close_app)
        style = ttk.Style(root)
        style.theme_use('clam')
        for name in ('TFrame', 'TLabelframe', 'TCheckbutton', 'TRadiobutton'):
            style.configure(name, background=BG)
        style.configure('TLabelframe.Label', background=BG, foreground='#20324d', font=('Segoe UI', 10, 'bold'))
        style.configure('TLabel', background=BG, foreground='#20324d', font=('Segoe UI', 10))
        style.configure('Title.TLabel', font=('Segoe UI', 18, 'bold'))
        style.configure('Big.TButton', font=('Segoe UI', 11, 'bold'), padding=(16, 9))
        style.configure('TButton', padding=(8, 5))

        bottom = ttk.Frame(root, padding=(16, 6, 16, 10))
        bottom.pack(side='bottom', fill='x')
        outer = ttk.Frame(root)
        outer.pack(fill='both', expand=True)
        canvas = tk.Canvas(outer, background=BG, highlightthickness=0)
        canvas.pack(side='left', fill='both', expand=True)
        scroll = ttk.Scrollbar(outer, orient='vertical', command=canvas.yview)
        scroll.pack(side='right', fill='y')
        canvas.configure(yscrollcommand=scroll.set)
        body = ttk.Frame(canvas, padding=16)
        window = canvas.create_window(0, 0, anchor='nw', window=body)
        canvas.bind('<Configure>', lambda e: canvas.itemconfigure(window, width=e.width))
        body.bind('<Configure>', lambda e: canvas.configure(scrollregion=canvas.bbox('all')))
        root.bind(
            '<MouseWheel>',
            lambda e: canvas.yview_scroll(-int(e.delta / 120), 'units')
            if e.widget.winfo_class() not in ('Listbox', 'Treeview')
            else None,
        )
        body.columnconfigure(0, weight=1)

        ttk.Label(body, text='SITFA Lite · Descargas RUS', style='Title.TLabel').grid(
            row=0, column=0, sticky='w', pady=(0, 6)
        )

        # 1. Conexión
        box = ttk.LabelFrame(body, text='1. Conexión con tu Chrome', padding=8)
        box.grid(row=2, column=0, sticky='ew', pady=(0, 8))
        box.columnconfigure(1, weight=1)
        row = ttk.Frame(box)
        row.grid(row=0, column=0, columnspan=4, sticky='w')
        self.open_button = ttk.Button(row, text='Abrir SITFA en Chrome', command=lambda: self.enqueue('open'))
        self.open_button.pack(side='left', padx=(0, 8))
        self.setup_button = ttk.Button(row, text='Instalar extensión…', command=self.setup)
        self.setup_button.pack(side='left', padx=(0, 8))
        self.disconnect_button = ttk.Button(row, text='Desconectar', command=lambda: self.enqueue('disconnect'))
        self.disconnect_button.pack(side='left', padx=(0, 16))
        self.link_label = ttk.Label(row, textvariable=self.link, foreground='#b45309')
        self.link_label.pack(side='left')
        ttk.Label(box, text='Código:').grid(row=1, column=0, sticky='w', pady=(6, 0))
        ttk.Entry(box, textvariable=self.code, state='readonly').grid(row=1, column=1, sticky='ew', padx=8, pady=(6, 0))
        self.copy_button = ttk.Button(box, text='Copiar código', command=self.copy_code)
        self.copy_button.grid(row=1, column=2, pady=(6, 0))

        # 2. Consulta
        box = ttk.LabelFrame(body, text='2. Qué descargar', padding=8)
        box.grid(row=3, column=0, sticky='ew', pady=(0, 8))
        radios = ttk.Frame(box)
        radios.pack(side='left', fill='x')
        self.screen_radios = []
        for i, label in enumerate(SCREENS):
            r = ttk.Radiobutton(radios, text=label, value=label, variable=self.screen, command=self.change_screen)
            r.grid(row=i // 2, column=i % 2, sticky='w', padx=(0, 30), pady=2)
            self.screen_radios.append(r)
        self.load_button = ttk.Button(box, text='Cargar opciones', command=self.load_catalog)
        self.load_button.pack(side='right', padx=(10, 0))

        # 3. Opciones
        self.options_box = ttk.LabelFrame(body, text='3. Opciones y filtros', padding=8)
        self.options_box.grid(row=4, column=0, sticky='ew')
        self.options_box.columnconfigure(0, weight=3)
        self.options_box.columnconfigure(1, weight=2)
        self.tribunals = TribunalList(self.options_box, on_change=self.refresh)
        self.tribunals.grid(row=0, column=0, rowspan=4, sticky='nsew', padx=(0, 10))
        self.tabs = CheckGroup(self.options_box, 'Pestañas', columns=2, on_change=self.refresh)
        self.modalities = CheckGroup(self.options_box, 'Modalidades', columns=2, on_change=self.refresh)
        self.informes = CheckGroup(self.options_box, 'Informe (pestaña Informes)', on_change=self.refresh)
        self.medidas = CheckGroup(self.options_box, 'Medidas (pestaña Cumplimiento)', on_change=self.refresh)
        self.states = CheckGroup(self.options_box, 'Estado', on_change=self.refresh)
        self.months = CheckGroup(self.options_box, 'Mes', columns=3, on_change=self.refresh)
        self.year_box = ttk.Frame(self.options_box)
        ttk.Label(self.year_box, text='Año:').pack(side='left')
        self.year_combo = ttk.Combobox(self.year_box, textvariable=self.year, width=8, state='readonly')
        self.year_combo.pack(side='left', padx=6)
        self.year_combo.bind('<<ComboboxSelected>>', lambda e: self.refresh())

        self.date_box = ttk.Frame(self.options_box)
        self.date_check = ttk.Checkbutton(
            self.date_box, text='Filtrar por fecha', variable=self.use_dates, command=self.refresh
        )
        self.date_check.grid(row=0, column=0, sticky='w', padx=(0, 12))
        self.date_widgets = []
        for col, (label, var) in enumerate((('Desde', self.from_date), ('Hasta', self.to_date))):
            ttk.Label(self.date_box, text=label).grid(row=0, column=1 + col * 3, padx=(0, 4))
            entry = ttk.Entry(self.date_box, textvariable=var, width=12)
            entry.grid(row=0, column=2 + col * 3)
            entry.bind('<KeyRelease>', lambda e: self.refresh())
            button = ttk.Button(self.date_box, text='📅', width=3, command=lambda var=var: self.pick(var))
            button.grid(row=0, column=3 + col * 3, padx=(2, 12))
            self.date_widgets += [entry, button]
        quick = ttk.Frame(self.date_box)
        quick.grid(row=1, column=0, columnspan=8, sticky='w', pady=(6, 0))
        for label in ('Este mes', 'Mes anterior', 'Últimos 30 días'):
            b = ttk.Button(quick, text=label, command=lambda label=label: self.quick_dates(label))
            b.pack(side='left', padx=(0, 6))
            self.date_widgets.append(b)
        self.filter_check = ttk.Checkbutton(
            self.date_box,
            text='Conservar los otros filtros preparados en SITFA (centro, RUT, causa, litigante…)',
            variable=self.keep_filters,
            command=self.refresh,
        )
        self.filter_check.grid(row=2, column=0, columnspan=8, sticky='w', pady=(6, 0))
        ttk.Label(self.options_box, textvariable=self.preview, font=('Segoe UI', 10, 'bold')).grid(
            row=10, column=0, columnspan=2, sticky='w', pady=(10, 0)
        )

        # 4. Descarga
        box = ttk.LabelFrame(bottom, text='4. Descargar (subcarpeta con fecha y hora por lote)', padding=8)
        box.pack(fill='x')
        box.columnconfigure(0, weight=1)
        ttk.Entry(box, textvariable=self.dest, state='readonly').grid(row=1, column=0, sticky='ew', pady=(0, 6))
        self.browse_button = ttk.Button(box, text='Cambiar carpeta…', command=self.browse)
        self.browse_button.grid(row=1, column=1, padx=8, pady=(0, 6))
        actions = ttk.Frame(box)
        actions.grid(row=2, column=0, columnspan=3, sticky='w')
        self.download_button = ttk.Button(actions, text='⬇  Descargar', style='Big.TButton', command=self.start)
        self.download_button.pack(side='left')
        self.cancel_button = ttk.Button(actions, text='Cancelar', command=self.request_cancel)
        self.cancel_button.pack(side='left', padx=8)
        self.folder_button = ttk.Button(actions, text='Abrir carpeta del lote', command=self.open_folder)
        self.folder_button.pack(side='left')
        ttk.Label(actions, textvariable=self.phase, font=('Segoe UI', 10, 'bold')).pack(side='left', padx=(16, 0))
        self.progress = ttk.Progressbar(box)
        self.progress.grid(row=4, column=0, columnspan=3, sticky='ew', pady=6)
        table = ttk.Frame(box)
        table.grid(row=5, column=0, columnspan=3, sticky='ew')
        table.columnconfigure(0, weight=1)
        self.tree = ttk.Treeview(table, columns=('rows', 'file'), show='headings', height=4)
        self.tree.heading('rows', text='Registros')
        self.tree.heading('file', text='Archivo guardado')
        self.tree.column('rows', width=90, stretch=False, anchor='center')
        self.tree.column('file', width=760)
        self.tree.grid(row=0, column=0, sticky='ew')
        bar = ttk.Scrollbar(table, orient='vertical', command=self.tree.yview)
        bar.grid(row=0, column=1, sticky='ns')
        self.tree.configure(yscrollcommand=bar.set)
        self.tree.bind('<Double-1>', self.open_selected)
        ttk.Label(bottom, textvariable=self.status, wraplength=900, foreground='#195fa8').pack(fill='x', pady=(6, 0))
        self.change_screen(initial=True)
        if start_worker:
            threading.Thread(target=self.worker, daemon=True).start()
            root.after(100, self.poll)

    # --- selección -------------------------------------------------------------------------------
    @property
    def screen_code(self):
        return SCREENS[self.screen.get()]

    def change_screen(self, initial=False):
        self.catalog = None
        for w in (self.tabs, self.modalities, self.informes, self.medidas, self.states, self.months):
            w.set_options({})
            w.grid_remove()
        self.tribunals.set_options({})
        self.year_box.grid_remove()
        self.date_box.grid_remove()
        screen = self.screen_code
        if screen == 'seguimiento':
            self.tabs.grid(row=0, column=1, sticky='ew', pady=(0, 6))
            self.modalities.grid(row=1, column=1, sticky='ew', pady=(0, 6))
            self.informes.grid(row=2, column=1, sticky='ew', pady=(0, 6))
            self.medidas.grid(row=3, column=1, sticky='ew', pady=(0, 6))
            self.date_box.grid(row=4, column=0, columnspan=2, sticky='w', pady=(8, 0))
        elif screen == 'litigantes':
            self.states.grid(row=0, column=1, sticky='ew', pady=(0, 6))
            self.date_box.grid(row=4, column=0, columnspan=2, sticky='w', pady=(8, 0))
        else:
            self.months.grid(row=0, column=1, sticky='ew', pady=(0, 6))
            self.year_box.grid(row=1, column=1, sticky='w', pady=(0, 6))
        if not initial:
            self.status.set('Pulsa «Cargar opciones»: SITFA abrirá la pantalla elegida en tu Chrome.')
        self.refresh()

    def apply_catalog(self, catalog):
        if not isinstance(catalog, dict) or catalog.get('pantalla') != self.screen_code:
            raise m.PocError('SITFA devolvió otra pantalla. Vuelve a pulsar Cargar opciones.')
        self.catalog = catalog
        chosen = catalog.get('seleccion') or {}
        tribunal = chosen.get('tribunal')
        self.tribunals.set_options(catalog['tribunales'], (tribunal,) if tribunal in catalog['tribunales'] else ())
        screen = self.screen_code
        if screen == 'seguimiento':
            tabs = {t: t for t in TAB_ORDER if t in catalog.get('pestanas', [])}
            self.tabs.set_options(tabs, tuple(t for t in ('Espera', 'Cumplimiento') if t in tabs))
            modalities = {k: MODALITIES[k] for k in catalog.get('modalidades', {}) if k in MODALITIES}
            self.modalities.set_options(modalities, tuple(modalities))
            self.informes.set_options(catalog.get('informes', {}), tuple(catalog.get('informes', {}))[:1])
            measures = {ALL_MEASURES: 'Todas las medidas', **catalog.get('medidas', {})}
            self.medidas.set_options(measures, (ALL_MEASURES,))
            if chosen.get('desde') and chosen.get('hasta'):
                self.from_date.set(chosen['desde'])
                self.to_date.set(chosen['hasta'])
        elif screen == 'litigantes':
            states = catalog.get('estados', {})
            self.states.set_options(states, (chosen.get('estado'),) if chosen.get('estado') in states else ())
        else:
            months = catalog.get('meses', {})
            self.months.set_options(months, (chosen.get('mes'),) if chosen.get('mes') in months else ())
            years = catalog.get('anios', {})
            self.year_combo.configure(values=list(years.values()))
            self.year.set(years.get(chosen.get('anio'), next(iter(years.values()), '')))
        self.status.set('Opciones cargadas. Marca tribunales y filtros, y pulsa Descargar.')
        self.refresh()

    def batch(self):
        if not self.catalog:
            raise m.PocError('Pulsa Cargar opciones primero.')
        screen = self.screen_code
        dates = None
        if screen in ('seguimiento', 'litigantes') and self.use_dates.get():
            dates = Dates(self.from_date.get().strip(), self.to_date.get().strip())
        keep = self.keep_filters.get() and screen in ('seguimiento', 'litigantes')
        tribunals = self.tribunals.selected()
        if screen == 'seguimiento':
            tabs = self.tabs.selected()
            return Batch(
                screen,
                tribunals,
                modalities=self.modalities.selected(),
                tabs=tabs,
                informes=self.informes.selected() if 'Informes' in tabs else (),
                medidas=self.medidas.selected() if 'Cumplimiento' in tabs else (),
                dates=dates,
                keep_filters=keep,
            )
        if screen == 'litigantes':
            return Batch(screen, tribunals, states=self.states.selected(), dates=dates, keep_filters=keep)
        year = next((k for k, v in self.catalog.get('anios', {}).items() if v == self.year.get()), None)
        return Batch(screen, tribunals, months=self.months.selected(), year=year)

    def refresh(self):
        idle = not self.busy and not self.quitting
        ready = idle and self.connected and self.catalog is not None
        for widget, enabled in [
            (self.open_button, idle),
            (self.setup_button, idle),
            (self.copy_button, idle),
            (self.disconnect_button, idle and self.connected),
            (self.load_button, idle and self.connected),
            (self.browse_button, idle),
            (self.cancel_button, self.busy and not self.quitting),
            (self.folder_button, self.last_folder is not None),
            (self.date_check, ready),
            (self.filter_check, ready),
        ]:
            widget.configure(state='normal' if enabled else 'disabled')
        for radio in self.screen_radios:
            radio.configure(state='normal' if idle else 'disabled')
        self.tribunals.set_enabled(ready)
        for group in (self.tabs, self.modalities, self.states, self.months):
            group.set_enabled(ready)
        tabs = self.tabs.selected()
        self.informes.set_enabled(ready and 'Informes' in tabs)
        self.medidas.set_enabled(ready and 'Cumplimiento' in tabs)
        self.year_combo.configure(state='readonly' if ready else 'disabled')
        for widget in self.date_widgets:
            widget.configure(state='normal' if ready and self.use_dates.get() else 'disabled')
        plan_ok = False
        if self.catalog:
            try:
                plan = self.batch().plan(self.catalog)
                plan_ok = True
                self.preview.set(
                    f'{len(plan)} consulta(s) · cada una con todas sus páginas · pantallazo PDF si no hay registros'
                )
            except m.PocError as exc:
                self.preview.set(str(exc))
        else:
            self.preview.set('Pulsa «Cargar opciones» para traer tribunales y filtros de tu sesión.')
        self.download_button.configure(state='normal' if ready and plan_ok else 'disabled')

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
        start, end = date_preset(label)
        self.from_date.set(start)
        self.to_date.set(end)
        self.use_dates.set(True)
        self.refresh()

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

    def start(self):
        try:
            batch = self.batch()
            batch.plan(self.catalog)
            destination = Path(self.dest.get())
            destination.mkdir(parents=True, exist_ok=True)
        except m.PocError as exc:
            messagebox.showwarning('Revisa la selección', str(exc), parent=self.root)
            return
        except OSError:
            messagebox.showwarning('Carpeta', 'No se pudo crear la carpeta de descarga.', parent=self.root)
            return
        for item in self.tree.get_children():
            self.tree.delete(item)
        self.cancel.clear()
        self.progress.configure(value=0, maximum=1)
        self.phase.set('Iniciando…')
        self.enqueue('download', (batch, destination))

    def request_cancel(self):
        self.cancel.set()
        self.phase.set('Cancelando al terminar la página en curso…')

    def open_folder(self):
        if self.last_folder:
            open_path(self.last_folder)

    def open_selected(self, _event=None):
        item = self.tree.focus()
        if item and self.last_folder:
            path = self.last_folder / self.tree.item(item, 'values')[1]
            if path.is_file():
                open_path(path)

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
                    BatchRunner(self.bridge, self.catalog, self.emit, self.cancel, pause=self.prefs.data['pausa']).run(
                        *arg
                    )
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
            self.status.set(value)
        elif name == 'disconnected':
            self.connected = False
            self.catalog = None
            self.code.set(value)
            self.change_screen(initial=True)
            self.status.set('Desconectado. Para volver a conectar usa el código nuevo.')
        elif name == 'status':
            self.status.set(value)
        elif name == 'catalog':
            try:
                self.apply_catalog(value)
            except (m.PocError, KeyError, TypeError) as exc:
                self.catalog = None
                self.status.set(str(exc) if isinstance(exc, m.PocError) else 'SITFA entregó opciones incompletas.')
        elif name == 'folder':
            self.last_folder = Path(value)
        elif name == 'plan':
            self.planned = value
            self.progress.configure(maximum=max(1, value), value=0)
        elif name == 'query':
            self.phase.set(f'Consulta {value["index"]} de {value["total"]}: {value["label"]}')
        elif name == 'total':
            self.status.set(f'{value} página(s) en esta consulta.')
        elif name == 'progress':
            self.tree.insert('', 'end', values=(value['registros'], value['archivo']))
            self.tree.yview_moveto(1)
        elif name == 'evidence':
            self.tree.insert('', 'end', values=(0, value['archivo']))
            self.tree.yview_moveto(1)
        elif name == 'query_done':
            self.progress.step(1)
        elif name == 'warning':
            self.status.set(value)
        elif name == 'done':
            self.phase.set(
                f'Listo: {value["queries"]} consultas · {value["files"]} XLS · {value["pdfs"]} pantallazos PDF · '
                f'{value["records"]} registros'
            )
            self.status.set('Lote terminado. Doble clic en un archivo para abrirlo.')
            self.root.bell()
        elif name == 'error':
            self.phase.set('Detenido')
            self.status.set(value)
            if not self.quitting:
                messagebox.showerror('SITFA Lite', value, parent=self.root)
        elif name == 'idle':
            self.busy = False
        elif name == 'exit':
            self.root.destroy()
            return False
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
            if not linked and self.catalog is not None:
                self.change_screen(initial=True)
                self.status.set('Se perdió la conexión con Chrome. Abre de nuevo la conexión desde el icono SITFA Lite.')
        self.link.set('● Conectado' if self.connected else '● Sin conexión')
        self.link_label.configure(foreground='#15803d' if self.connected else '#b45309')
        self.refresh()
        self.root.after(400, self.poll)


def main():
    root = tk.Tk()
    Application(root)
    root.mainloop()


if __name__ == '__main__':
    main()
