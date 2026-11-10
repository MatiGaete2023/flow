"""Ventana local: el Chrome habitual hace las consultas mediante una extension."""

from datetime import date
import os
from pathlib import Path
import queue
import subprocess
import threading
import time
import tkinter as tk
from tkinter import filedialog, messagebox, simpledialog, ttk

import motor as m
from contrato_integral import VERSION
from lotes import Batch, BatchRunner, Dates, MODALITIES
from puente import Bridge
from perfiles import SCREENS
from pausa import PauseGate
from preferencias import Preferences, pending_plan

RELOAD = 'Más › Recargar opciones de SITFA'
ALL_TRIBUNALS = 'Todos los tribunales disponibles'
ALL_MODALITIES = 'Todas las modalidades'
TAB_OPTIONS = {
    'Espera': ('Espera',),
    'Cumplimiento': ('Cumplimiento',),
    'Espera y Cumplimiento': ('Espera', 'Cumplimiento'),
    'Informes': ('Informes',),
    'Egresados': ('Egresados',),
}


class TribunalPicker(tk.Toplevel):
    """Las casillas se guardan solo al pulsar Aplicar; cerrar conserva la selección. Sirve también para modalidades."""

    def __init__(
        self,
        root,
        catalog,
        selected,
        apply,
        *,
        title='Elegir tribunales',
        prompt='Marca los tribunales que quieres incluir.',
        counted='tribunales seleccionados',
    ):
        super().__init__(root)
        self.title(title)
        self.counted = counted
        self.geometry('560x520' if len(catalog) > 8 else '480x360')
        self.minsize(430, 300)
        self.transient(root)
        self.grab_set()
        self.apply_selection = apply
        self.choices = {key: tk.BooleanVar(self, value=key in selected) for key in catalog}
        self.labels = catalog
        self.search = tk.StringVar()
        self.selection_count = tk.StringVar()
        ttk.Label(self, text=prompt, padding=12).pack(anchor='w')
        ttk.Entry(self, textvariable=self.search).pack(fill='x', padx=12)
        ttk.Label(self, textvariable=self.selection_count).pack(anchor='w', padx=12)
        controls = ttk.Frame(self, padding=(12, 0))
        controls.pack(fill='x')
        ttk.Button(controls, text='Marcar todos', command=lambda: self.set_all(True)).pack(side='left', padx=(0, 8))
        ttk.Button(controls, text='Desmarcar todos', command=lambda: self.set_all(False)).pack(side='left')
        ttk.Button(controls, text='Marcar visibles', command=lambda: self.set_visible(True)).pack(side='left', padx=8)
        container = ttk.Frame(self, padding=12)
        container.pack(fill='both', expand=True)
        canvas = tk.Canvas(container, highlightthickness=0, bg='#f3f6fa')
        canvas.pack(side='left', fill='both', expand=True)
        bar = ttk.Scrollbar(container, orient='vertical', command=canvas.yview)
        bar.pack(side='right', fill='y')
        canvas.configure(yscrollcommand=bar.set)
        items = ttk.Frame(canvas)
        window = canvas.create_window(0, 0, anchor='nw', window=items)
        canvas.bind('<Configure>', lambda e: canvas.itemconfigure(window, width=e.width))
        items.bind('<Configure>', lambda e: canvas.configure(scrollregion=canvas.bbox('all')))
        self.checks = {}
        for key, label in catalog.items():
            self.checks[key] = ttk.Checkbutton(items, text=label, variable=self.choices[key])
            self.checks[key].pack(anchor='w', fill='x', pady=3)
            self.choices[key].trace_add('write', lambda *_: self.filter_choices())
        self.search.trace_add('write', lambda *_: self.filter_choices())
        self.filter_choices()
        canvas.bind('<MouseWheel>', lambda e: canvas.yview_scroll(-int(e.delta / 120), 'units'))
        actions = ttk.Frame(self, padding=12)
        actions.pack(fill='x')
        ttk.Button(actions, text='Cancelar', command=self.destroy).pack(side='right')
        ttk.Button(actions, text='Aplicar', command=self.apply).pack(side='right', padx=8)

    def set_all(self, value):
        for choice in self.choices.values():
            choice.set(value)

    def set_visible(self, value):
        for key, choice in self.choices.items():
            if m.normalized(self.search.get()) in m.normalized(self.labels[key]):
                choice.set(value)

    def filter_choices(self):
        query = m.normalized(self.search.get())
        for key, widget in self.checks.items():
            widget.pack_forget()
            if query in m.normalized(self.labels[key]):
                widget.pack(anchor='w', fill='x', pady=3)
        self.selection_count.set(
            f'{sum(v.get() for v in self.choices.values())} {self.counted} (se conservan los ocultos)'
        )

    def apply(self):
        self.apply_selection(tuple(k for k, v in self.choices.items() if v.get()))
        self.destroy()


def open_chrome(url):
    candidates = [
        Path(os.environ.get(env, '')) / 'Google/Chrome/Application/chrome.exe'
        for env in ('ProgramFiles', 'ProgramFiles(x86)', 'LOCALAPPDATA')
    ]
    chrome = next((p for p in candidates if p.is_file()), None)
    if chrome is None:
        raise m.PocError('No se encontró Chrome instalado.')
    # Chrome resuelve su perfil habitual. No se usa depuracion ni una copia del perfil.
    subprocess.Popen([str(chrome), url], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


DOCS = [
    ('Manual de uso (PDF)', 'Manual_Descargador_SITFA.pdf'),
    ('Video: Descargador SITFA', 'Flow_Video_Descargador_SITFA.mp4'),
    ('Video: retomar un paquete', 'Flow_Video_Retomar_Paquete.mp4'),
]
SIDE_NAMES = {
    'calendario_medidas': 'Medidas por vencer',
    'calendario_informes': 'Informes por vencer',
    'carga': 'Resoluciones firmadas',
    'litigantes': 'Litigantes y órdenes',
}


class Application:
    def __init__(self, root, start_worker=True, bridge_factory=Bridge, preferences_path=None):
        self.root = root
        self.events = queue.Queue()
        self.commands = queue.Queue()
        self.cancel = PauseGate()
        self.bridge_factory = bridge_factory
        self.bridge = bridge_factory(self.emit)
        self.catalog = None
        self.connected = False
        self.busy = False
        self.quitting = False
        self.last_folder = None
        self.last_sheets_folder = None
        self.last_diary_manifest = None
        self.prefs = Preferences(preferences_path, memory=not start_worker and preferences_path is None)
        self.recovery = None
        self.context = tk.StringVar(value='')
        self.query_index = 0
        self.query_total = 0
        self.result_windows = []
        self.last_package = None
        self.phase_name = tk.StringVar(value='')
        self.progress_detail = tk.StringVar(value='')
        self.result_notice = tk.StringVar()
        self._phase_started = None
        self._pause_started = None
        self._phase_paused = 0.0
        self.csmp_reply = None
        self._report_options = None
        self.csmp_mode = None
        self.bitacoras_reply = None
        self.bitacoras_id = None
        self.tribunal = tk.StringVar(value=ALL_TRIBUNALS)
        self.modality = tk.StringVar(value=ALL_MODALITIES)
        self.selected_modalities = None
        self.selected_tribunals = (
            None if self.prefs.data['tribunales'] is None else tuple(self.prefs.data['tribunales'])
        )
        self.screen = tk.StringVar(value='Seguimiento')
        self.order_state = tk.StringVar()
        self.month = tk.StringVar()
        self.year = tk.StringVar()
        self.tab = tk.StringVar(value='Espera y Cumplimiento')
        self.report = tk.StringVar()
        self.from_date = tk.StringVar(value=m.now().strftime('%d/%m/%Y'))
        self.to_date = tk.StringVar(value=self.from_date.get())
        self.use_dates = tk.BooleanVar(value=False)
        self.keep_filters = tk.BooleanVar(value=False)
        from preferencias import data_directory

        default_folder = data_directory() / 'descargas'
        self.dest = tk.StringVar(value=self.prefs.data['destino'] or str(default_folder))
        from preferencias import onedrive_warning

        self.destination_warning = tk.StringVar(value=onedrive_warning(self.dest.get()))
        self.dest.trace_add('write', lambda *args: self.destination_warning.set(onedrive_warning(self.dest.get())))
        self.code = tk.StringVar(value=self.bridge.connection_code)
        self.status = tk.StringVar(value='Abre tu Chrome, entra a Seguimiento y conecta la extensión local.')
        self.preview = tk.StringVar(value='Carga las opciones de tu sesión para preparar el lote.')
        self.count = tk.StringVar(value='')
        self.title = f'Descargador SITFA {VERSION}'
        root.title(self.title)
        import flowkit as ui

        self.ui = ui
        self.topbar = ui.TopBar(root, 'Descargador SITFA')
        self.topbar.pack(side='top', fill='x')
        tools = self.topbar.menu
        self.theme = tk.StringVar(value=self.prefs.data.get('tema', 'Sistema'))
        # Abrir SITFA también desde el menú: el recuadro de conexión se oculta al conectar.
        tools.add_command(label='Abrir mi Chrome en SITFA', command=lambda: self.enqueue('open'))
        tools.add_command(label='Instalar extensión…', command=self.setup)
        tools.add_command(label='Copiar código de conexión', command=self.copy_code)
        tools.add_command(label='Recargar opciones de SITFA', command=lambda: self.enqueue('catalog'))
        tools.add_command(label='Desconectar', command=lambda: self.enqueue('disconnect'))
        tools.add_command(label='Carpeta de descarga…', command=self.browse)
        # Todo lo que no es «crear el paquete» vive aquí; su estado sigue al de su botón (ver _sync_menu).
        tools.add_separator()
        self.menu_actions = []
        for label, name, command in [
            ('Descargar sin crear paquete', 'download_button', self.start_download),
            ('Descarga conjunta CSMP', 'joint_button', self.start_joint),
            ('Leer bitácoras del lote', 'diary_button', self.start_diary),
            ('Reintentar lecturas fallidas…', 'diary_retry_button', self.resume_diary),
            ('Crear paquete de este lote…', 'package_button', self.create_package),
            ('Abrir carpeta del lote', 'folder_button', self.open_folder),
            ('Abrir carpeta del paquete', 'package_folder_button', self.open_package_folder),
            ('Copiar ruta del paquete', 'copy_path_button', self.copy_package_path),
        ]:
            tools.add_command(label=label, command=command)
            self.menu_actions.append((tools.index('end'), name))
        tools.add_separator()
        for label, command in [
            ('Guardar consulta favorita…', self.save_favorite),
            ('Elegir favorita…', self.choose_favorite),
            ('Resultados e historial', self.show_results),
            ('Limpiar pendientes preparados', self.clear_pending),
            ('Configurar ejecutable CSMP…', self.configure_csmp),
        ]:
            tools.add_command(label=label, command=command)
        tools.add_command(label='Retomar paquete…', command=self.resume_pipeline)
        tools.add_command(label='Retomar descarga conjunta…', command=self.resume_joint)
        tools.add_command(label='Pausa entre consultas…', command=self.configure_pause)
        self.sound = tk.BooleanVar(value=self.prefs.data['aviso_sonoro'])
        tools.add_checkbutton(label='Aviso sonoro al terminar', variable=self.sound, command=self.save_preferences)
        tools.add_separator()
        appearance = tk.Menu(tools, tearoff=False)
        tools.add_cascade(label='Apariencia', menu=appearance)
        for choice in ('Claro', 'Oscuro', 'Sistema'):
            appearance.add_radiobutton(label=choice, variable=self.theme, value=choice, command=self.apply_theme)
        tools.add_separator()
        for label, name in DOCS:
            tools.add_command(label=label, command=lambda name=name: self.open_doc(name))
        self.topbar.add_note(f'Versión {VERSION}')
        tools.configure(postcommand=self._sync_menu)
        root.geometry(f'1000x{min(900, max(600, root.winfo_screenheight() - 120))}')
        root.minsize(880, 600)
        root.protocol('WM_DELETE_WINDOW', self.close_app)
        ui.apply_theme(root)
        style = ttk.Style(root)
        style.configure('Section.TLabel', font=('Segoe UI', 12, 'bold'))
        # Pie: una sola acción principal; Detener y Pausar aparecen solo mientras trabaja.
        footer = ttk.Frame(root, style='Bar.TFrame', padding=(12, 8))
        footer.pack(side='bottom', fill='x')
        ttk.Separator(root).pack(side='bottom', fill='x')
        bar = ttk.Frame(footer, style='Bar.TFrame')
        bar.pack(fill='x')
        self.folder_link = ttk.Button(bar, command=self.browse, style='Link.TButton')
        self.browse_button = self.folder_link
        self.folder_link.pack(side='left', padx=(0, 10))
        # width=1: el resumen usa el espacio libre sin fijar el ancho mínimo de la ventana.
        ttk.Label(bar, textvariable=self.preview, style='BarMuted.TLabel', width=1).pack(
            side='left', fill='x', expand=True
        )
        self.pipeline_button = ttk.Button(
            bar, text='Consultar y crear paquete', style='Primary.TButton', command=self.primary_action
        )
        self.pipeline_button.pack(side='right')
        self.live_cancel = ttk.Button(bar, text='Detener', command=self.request_cancel)
        self.live_pause = ttk.Button(bar, text='Pausar', command=self.toggle_pause)
        self.banner = ui.Banner(root)
        self.banner.bind('<<BannerShow>>', lambda _e: self._banner(True))
        self.banner.bind('<<BannerHide>>', lambda _e: self._banner(False))
        container = ttk.Frame(root)
        container.pack(fill='both', expand=True)
        self.container = container
        from flowkit import SideList

        # Lista lateral de consultas (como SITFA Lite): el paquete arriba; las descargas sueltas debajo.
        self.side = SideList(
            container,
            [
                ('Paquete', [('Seguimiento', 'Seguimiento')]),
                ('Solo descargar', [(k, SIDE_NAMES.get(v, k)) for k, v in SCREENS.items() if v != 'seguimiento']),
            ],
            command=self._side_selected,
            width=210,
        )
        self.side.pack(side='left', fill='y')
        self.side.pack_propagate(False)
        ttk.Separator(container, orient='vertical').pack(side='left', fill='y')
        self.canvas = tk.Canvas(container, bg=ui.PALETTE['bg'], highlightthickness=0)
        self.canvas.pack(side='left', fill='both', expand=True)
        scroll = ttk.Scrollbar(container, orient='vertical', command=self.canvas.yview)
        scroll.pack(side='right', fill='y')
        self.canvas.configure(yscrollcommand=scroll.set)
        body = ttk.Frame(self.canvas, padding=(18, 14))
        window = self.canvas.create_window(0, 0, anchor='nw', window=body)
        self.canvas.bind(
            '<Configure>',
            lambda e: self.canvas.itemconfigure(window, width=e.width, height=max(e.height, body.winfo_reqheight())),
        )
        body.bind('<Configure>', lambda e: self.canvas.configure(scrollregion=self.canvas.bbox('all')))
        root.bind(
            '<MouseWheel>',
            lambda e: (
                self.canvas.yview_scroll(-int(e.delta / 120), 'units')
                if e.widget.winfo_class() not in ('TCombobox', 'Treeview')
                else None
            ),
        )
        body.columnconfigure(0, weight=1)
        # Conexión guiada en tres pasos (como SITFA Lite); se oculta al conectar.
        connect = ttk.Frame(body)
        connect.grid(row=2, column=0, sticky='ew', pady=(0, 12))
        self.connect_box = connect
        ttk.Label(connect, text='Conecta tu Chrome', font=ui.FONT_BOLD).pack(anchor='w', pady=(0, 10))
        self.step_marks = {}

        def step(number, text):
            line = ttk.Frame(connect)
            line.pack(anchor='w', fill='x', pady=(0, 6))
            mark = ttk.Label(line, text=str(number), width=3, style='Step.TLabel')
            mark.pack(side='left')
            ttk.Label(line, text=text).pack(side='left')
            content = ttk.Frame(connect)
            content.pack(anchor='w', padx=(28, 0), pady=(0, 10))
            self.step_marks[number] = mark
            return content

        first = step(1, 'Abre SITFA en tu Chrome, inicia sesión y entra a Seguimiento.')
        self.open_button = ttk.Button(
            first, text='Abrir mi Chrome en SITFA', style='Primary.TButton', command=lambda: self.enqueue('open')
        )
        self.open_button.pack(side='left')
        second = step(2, 'Copia el código de conexión.')
        self.code_entry = ttk.Entry(second, textvariable=self.code, state='readonly', width=44)
        self.code_entry.pack(side='left')
        self.copy_button = ttk.Button(second, text='Copiar', command=self.copy_code)
        self.copy_button.pack(side='left', padx=(6, 0))
        step(3, 'En la pestaña de SITFA pulsa el ícono de la extensión, pega el código y pulsa «Conectar».')
        ttk.Separator(connect).pack(fill='x', pady=(2, 10))
        first_time = ttk.Frame(connect)
        first_time.pack(anchor='w')
        ttk.Label(
            first_time, text='¿Primera vez? La extensión de Chrome se instala una sola vez.', style='Muted.TLabel'
        ).pack(side='left')
        self.setup_button = ttk.Button(first_time, text='Instalar extensión…', command=self.setup)
        self.setup_button.pack(side='left', padx=(8, 0))
        # Controles de estado sin mostrar: guardan la elección y el resto del código y las pruebas los consultan.
        hidden = ttk.Frame(body)
        self.load_button = ttk.Button(hidden, text='Cargar opciones', command=lambda: self.enqueue('catalog'))
        self.disconnect_button = ttk.Button(hidden, text='Desconectar', command=lambda: self.enqueue('disconnect'))
        self.screen_combo = ttk.Combobox(hidden, textvariable=self.screen, values=tuple(SCREENS), state='disabled')
        self.trib_button = ttk.Button(hidden, text='Elegir tribunales…', command=self.choose_tribunals)
        self.mod_combo = ttk.Button(hidden, text='Elegir modalidades…', command=self.choose_modalities)
        self.tab_combo = ttk.Combobox(hidden, textvariable=self.tab, values=tuple(TAB_OPTIONS))
        self.report_combo = ttk.Combobox(hidden, textvariable=self.report)
        self.state_combo = ttk.Combobox(hidden, textvariable=self.order_state, width=20)
        self.month_combo = ttk.Combobox(hidden, textvariable=self.month, width=16)
        self.year_combo = ttk.Combobox(hidden, textvariable=self.year, width=8)
        self.date_check = ttk.Checkbutton(
            hidden, text='Filtrar por fecha', variable=self.use_dates, command=self.refresh
        )
        self.filter_check = ttk.Checkbutton(
            hidden, text='Conservar los otros filtros de SITFA', variable=self.keep_filters, command=self.refresh
        )
        # Qué descargar: todo como fichas en la misma pantalla.
        self.form = ui.Form(body, key_width=14)
        self.form.grid(row=3, column=0, sticky='ew', pady=(0, 6))
        f = self.form
        self.trib_chips = f.row(
            'Tribunales', lambda p: ui.Chips(p, command=self._chips_tribunals, all_label='Todos'), name='tribunales'
        )
        self.mod_chips = f.row('Modalidad', lambda p: ui.Chips(p, command=self._chips_modalities), name='modalidad')
        self.tab_chips = f.row(
            'Registros',
            lambda p: ui.Chips(p, [(k, k) for k in TAB_OPTIONS], multi=False, command=self._chips_tab),
            name='registros',
        )
        self.report_chips = f.row(
            'Filtro', lambda p: ui.Chips(p, multi=False, command=self._chips_report), name='filtro'
        )
        self.state_chips = f.row(
            'Estado',
            lambda p: ui.Chips(p, multi=False, command=self._chips_single('state_chips', self.order_state)),
            name='estado',
        )
        self.month_chips = f.row(
            'Mes',
            lambda p: ui.Chips(p, multi=False, command=self._chips_single('month_chips', self.month)),
            name='mes',
        )
        self.year_chips = f.row(
            'Año',
            lambda p: ui.Chips(p, multi=False, command=self._chips_single('year_chips', self.year)),
            name='anio',
        )
        self.period_chips = f.row(
            'Período',
            lambda p: ui.Chips(
                p, [(x, x) for x in ('Sin filtro', 'Por fecha')], multi=False, command=self._chips_period
            ),
            name='periodo',
        )
        f.row('', self._range_row, name='rango')
        self.keep_chips = f.row(
            'Otros filtros',
            lambda p: ui.Chips(p, [('keep', 'Conservar los otros filtros de SITFA')], command=self._chips_keep),
            name='otros',
        )
        options = f.row('Incluir', lambda p: ttk.Frame(p), name='incluir')
        self.package_consolidated = tk.BooleanVar(value=self.prefs.data['paquete_consolidado'])
        self.reuse_diaries = tk.BooleanVar(value=self.prefs.data['reutilizar_bitacoras'])
        self.read_calendars = tk.BooleanVar(value=self.prefs.data['leer_calendarios'])
        self.read_history = tk.BooleanVar(value=self.prefs.data['leer_historia'])
        self.history_in_package = tk.BooleanVar(value=self.prefs.data['historia_en_paquete'])
        self.reports_due = tk.BooleanVar(value=self.prefs.data['informes_por_vencer'])
        self.package_per_sheet = tk.BooleanVar(value=self.prefs.data['paquete_por_hoja'])
        # Pantallazo de constancia en consultas sin registros; sin él la consulta vacía igual queda registrada.
        self.empty_pdf = tk.BooleanVar(value=self.prefs.data['pdf_sin_registros'])

        def chip(text, variable, column, row, command=None):
            button = ttk.Checkbutton(
                options, text=text, style='Chip.Toolbutton', variable=variable, command=command or self.save_preferences
            )
            button.grid(row=row, column=column, sticky='w', padx=(0, 6), pady=(0, 6))
            return button

        # D6: el paquete puede llevar solo el XLSX consolidado (nunca HTML ni PDF) para la copia nativa del Asistente.
        self.package_choice = chip('XLSX consolidado', self.package_consolidated, 0, 0)
        # EF-04: menos aperturas en SITFA; solo lecturas del mismo día y de menos de 12 h, con hash comprobado.
        self.reuse_choice = chip('Reutilizar bitácoras de hoy', self.reuse_diaries, 1, 0)
        # WD-1: lectura opcional del calendario de informes proyectados; cuenta en el máximo de lecturas del lote.
        self.calendar_choice = chip('Calendario de informes', self.read_calendars, 2, 0)
        self.reports_choice = chip('Informes por vencer (60 días)', self.reports_due, 0, 1)
        chip('Un paquete por hoja', self.package_per_sheet, 1, 1)
        chip('PDF si no hay registros', self.empty_pdf, 2, 1)
        # Historia de la causa (últimos 4 meses): una lectura por causa en la misma pasada de las bitácoras.
        self.history_choice = chip('Historia de la causa (prueba)', self.read_history, 0, 2, self.toggle_history)
        self.history_package_choice = chip('Historia en el paquete', self.history_in_package, 1, 2)
        if not self.read_history.get():
            self.history_package_choice.grid_remove()
        # Período a recolectar en bitácoras e historia de la causa (1 a 4 meses hacia atrás).
        period = ttk.Frame(options)
        period.grid(row=3, column=0, columnspan=3, sticky='w', pady=(0, 6))
        self.months_back = tk.StringVar(value=str(self.prefs.data['meses_atras']))
        ttk.Label(period, text='Meses de bitácora', style='Muted.TLabel').pack(side='left', padx=(0, 4))
        self.months_combo = ttk.Combobox(
            period, textvariable=self.months_back, values=('1', '2', '3', '4'), width=3, state='readonly'
        )
        self.months_combo.pack(side='left')
        self.months_combo.bind('<<ComboboxSelected>>', lambda _event: self.save_preferences())
        self.history_counts = {'leidas': 0, 'fallidas': 0}
        ttk.Label(body, textvariable=self.destination_warning, foreground=ui.PALETTE['warn'], wraplength=800).grid(
            row=4, column=0, sticky='w'
        )
        # Botones de las acciones secundarias: no se muestran; el menú «Más» los usa (mismo estado y comando).
        actions = ttk.Frame(hidden)
        self.advanced_actions = actions
        self.download_button = ttk.Button(
            actions, text='Descargar el lote completo', style='Secondary.TButton', command=self.start_download
        )
        self.cancel_button = ttk.Button(actions, text='Cancelar', command=self.request_cancel)
        self.folder_button = ttk.Button(actions, text='Abrir carpeta', command=self.open_folder)
        self.joint_button = ttk.Button(actions, text='Descarga conjunta CSMP', command=self.start_joint)
        self.diary_button = ttk.Button(actions, text='Leer bitácoras del lote', command=self.start_diary)
        self.diary_retry_button = ttk.Button(actions, text='Reintentar lecturas fallidas…', command=self.resume_diary)
        self.pause_button = ttk.Button(actions, text='Pausar / continuar', command=self.toggle_pause)
        self.package_button = ttk.Button(actions, text='Crear paquete de este lote…', command=self.create_package)
        self.package_folder_button = ttk.Button(
            actions, text='Abrir carpeta del paquete', command=self.open_package_folder
        )
        self.copy_path_button = ttk.Button(actions, text='Copiar ruta', command=self.copy_package_path)
        self.advanced_toggle = ttk.Button(hidden, text='Avanzado / recuperación', command=self.toggle_advanced)
        # Avance: aparece con la primera operación.
        results = ttk.Frame(body)
        results.grid(row=5, column=0, sticky='ew', pady=(8, 0))
        results.grid_remove()
        self.results_box = results
        results.columnconfigure(0, weight=1)
        ttk.Label(results, textvariable=self.phase_name, style='Section.TLabel').grid(row=3, column=0, sticky='w')
        ttk.Label(results, textvariable=self.progress_detail, style='Muted.TLabel').grid(row=4, column=0, sticky='w')
        self.progress = ttk.Progressbar(results)
        self.progress.grid(row=5, column=0, sticky='ew', pady=10)
        table = ttk.Frame(results)
        table.grid(row=6, column=0, sticky='ew')
        table.columnconfigure(0, weight=1)
        self.tree = ttk.Treeview(table, columns=('page', 'rows', 'file'), show='headings', height=5)
        for col, label, width in [('page', 'Página', 80), ('rows', 'Registros', 85), ('file', 'Archivo guardado', 590)]:
            self.tree.heading(col, text=label)
            self.tree.column(col, width=width, stretch=col == 'file')
        self.tree.grid(row=0, column=0, sticky='ew')
        bar = ttk.Scrollbar(table, orient='vertical', command=self.tree.yview)
        bar.grid(row=0, column=1, sticky='ns')
        self.tree.configure(yscrollcommand=bar.set)
        self.tree.bind('<Double-1>', self.open_selected)
        ttk.Label(results, text='Doble clic en una fila para abrir el archivo.', style='Hint.TLabel').grid(
            row=7, column=0, sticky='w', pady=(4, 0)
        )
        ttk.Label(results, textvariable=self.count).grid(row=8, column=0, sticky='w', pady=(6, 0))
        ttk.Label(results, textvariable=self.context, style='Muted.TLabel').grid(row=9, column=0, sticky='w')
        ttk.Label(results, textvariable=self.result_notice, wraplength=760).grid(row=10, column=0, sticky='ew')
        ttk.Label(body, textvariable=self.status, wraplength=760, style='Muted.TLabel').grid(
            row=6, column=0, sticky='ew', pady=(12, 0)
        )
        for combo in (
            self.tab_combo,
            self.report_combo,
            self.state_combo,
            self.month_combo,
            self.year_combo,
        ):
            combo.bind('<<ComboboxSelected>>', lambda e: self.refresh())
        # Teclado: Enter = acción principal · Esc = detener · F1 = manual.
        root.bind('<Return>', self._enter, add='+')
        root.bind('<Escape>', lambda _e: self.request_cancel() if self._cancellable() else None, add='+')
        root.bind('<F1>', lambda _e: self.open_doc(DOCS[0][1]), add='+')
        self.dest.trace_add('write', lambda *_: self._show_destination())
        self._show_destination()
        self.refresh()
        self.apply_theme(save=False)
        self.auto_catalog = start_worker
        if start_worker:
            threading.Thread(target=self.worker, daemon=True).start()
            root.after(100, self.poll)
            root.after(600, self.offer_resume)

    # --- piezas de la ventana -------------------------------------------------------------------------------------
    def _range_row(self, parent):
        frame = ttk.Frame(parent)
        self.from_field = self.ui.DateField(frame, self.from_date, self.refresh, label='Desde')
        self.from_field.pack(side='left', padx=(0, 14))
        self.to_field = self.ui.DateField(frame, self.to_date, self.refresh, label='Hasta')
        self.to_field.pack(side='left', padx=(0, 14))
        self.date_error = ttk.Label(frame, style='Bad.TLabel')
        self.date_error.pack(side='left')
        self.date_widgets = [self.from_field.entry, self.from_field.button, self.to_field.entry, self.to_field.button]
        return frame

    def _banner(self, visible):
        if visible and not self.banner.winfo_manager():
            self.banner.pack(side='bottom', fill='x', padx=12, pady=(0, 8), before=self.container)
        elif not visible:
            self.banner.pack_forget()

    def _show_destination(self):
        self.folder_link.configure(text='Carpeta: ' + self.ui.short_path(self.dest.get()))

    def _mark_step(self, number):
        self.step_marks[number].configure(text='✓', style='Ok.TLabel')

    def _cancellable(self):
        return str(self.cancel_button.cget('state')) == 'normal'

    def _enter(self, event):
        if event.widget.winfo_class() in ('TEntry', 'Entry', 'TCombobox', 'Text', 'Treeview', 'TButton'):
            return
        if event.widget.winfo_toplevel() is self.root and self.pipeline_button.instate(['!disabled']):
            self.pipeline_button.invoke()

    def _sync_menu(self):
        """El menú «Más» refleja el estado de cada acción secundaria al abrirse."""
        for index, name in self.menu_actions:
            self.topbar.menu.entryconfigure(index, state=str(getattr(self, name).cget('state')))

    def open_doc(self, name):
        path = self.ui.find_doc(name, __file__)
        if path is None or not self.ui.open_path(path):
            self.banner.show(
                f'No se encontró {name} junto al programa: búscalo en la carpeta de manuales de la entrega.',
                kind='warn',
            )

    def primary_action(self):
        """Seguimiento crea el paquete; las otras consultas (y los pendientes preparados) solo descargan."""
        if SCREENS[self.screen.get()] == 'seguimiento' and self.recovery is None:
            self.start_pipeline()
        else:
            self.start_download()

    def offer_resume(self):
        """Si un paquete reciente quedó cortado, ofrecer retomarlo sin buscar la carpeta."""
        from flujo_paquete import pendientes

        found = pendientes(self.dest.get())
        if found and not self.busy:
            folder, phase = found[0]
            fase = {
                'consultas': 'en las consultas',
                'bitacoras': 'en las bitácoras',
                'informes': 'en los informes',
                'paquete': 'al crear el ZIP',
            }.get(phase, '')
            self.banner.show(
                f'El paquete «{folder.name}» quedó sin terminar {fase}. Puedes retomarlo sin repetir lo ya hecho.'.replace(
                    '  ', ' '
                ),
                kind='warn',
                action=lambda: self.resume_pipeline(folder),
                action_text='Retomar',
            )
        return found

    # --- fichas ↔ estado ------------------------------------------------------------------------------------------
    def _chips_tribunals(self):
        if self.catalog:
            self.select_tribunals(self.trib_chips.get())

    def _chips_modalities(self):
        if self.catalog:
            self.select_modalities(self.mod_chips.get())

    def _chips_tab(self):
        self.tab.set(self.tab_chips.get()[0])
        self.refresh()

    def _chips_report(self):
        if self.report_chips.get():
            self.report.set(self.report_chips.get()[0])
        self.refresh()

    def _chips_single(self, name, variable):
        def changed():
            chosen = getattr(self, name).get()
            if chosen:
                variable.set(chosen[0])
            self.refresh()

        return changed

    def _chips_period(self):
        self.use_dates.set(self.period_chips.get() == ('Por fecha',))
        self.refresh()

    def _chips_keep(self):
        self.keep_filters.set(bool(self.keep_chips.get()))
        self.refresh()

    def _fill_chips(self):
        """Opciones de las fichas a partir del catálogo de SITFA (una vez por catálogo)."""
        catalog = self.catalog or {}
        self.trib_chips.set_options(list(catalog.get('tribunales', {}).items()), self.selected_tribunals or ())
        available = self.available_modalities()
        self.mod_chips.set_options(
            list(available.items()), tuple(available) if self.selected_modalities is None else self.selected_modalities
        )
        for chips, key, variable in (
            (self.state_chips, 'estados', self.order_state),
            (self.month_chips, 'meses', self.month),
            (self.year_chips, 'anios', self.year),
        ):
            chips.set_options([(v, v) for v in catalog.get(key, {}).values()], [variable.get()])
        self._report_options = None

    def _sync_chips(self, ready, screen):
        followup, calendar_screen = screen == 'seguimiento', screen.startswith('calendario_')
        values = list(self.report_combo.cget('values') or ())
        if tuple(values) != self._report_options:
            self._report_options = tuple(values)
            self.report_chips.set_options([(v, v) for v in values], [self.report.get()])
        self.trib_chips.set(self.selected_tribunals or ())
        available = self.available_modalities()
        self.mod_chips.set(tuple(available) if self.selected_modalities is None else self.selected_modalities)
        self.tab_chips.set([self.tab.get()])
        self.report_chips.set([self.report.get()])
        self.state_chips.set([self.order_state.get()])
        self.month_chips.set([self.month.get()])
        self.year_chips.set([self.year.get()])
        self.period_chips.set(['Por fecha' if self.use_dates.get() else 'Sin filtro'])
        self.keep_chips.set(['keep'] if self.keep_filters.get() else [])
        dates = screen == 'carga' or (screen in ('seguimiento', 'litigantes') and self.use_dates.get())
        rows = {
            'tribunales': True,
            'modalidad': followup,
            'registros': followup,
            'filtro': followup and self.tab.get() != 'Espera',
            'estado': screen == 'litigantes',
            'mes': calendar_screen,
            'anio': calendar_screen,
            'periodo': screen in ('seguimiento', 'litigantes'),
            'rango': dates,
            'otros': screen in ('seguimiento', 'litigantes'),
            'incluir': followup,
        }
        for name, visible in rows.items():
            self.form.show(name, visible)
        locked = bool(self.recovery)
        self.trib_chips.enable(ready and not locked)
        self.mod_chips.enable(ready and followup and not locked)
        self.tab_chips.enable(ready and followup and not locked)
        self.report_chips.enable(ready and followup and self.tab.get() != 'Espera' and not locked)
        self.state_chips.enable(ready and screen == 'litigantes' and not locked)
        self.month_chips.enable(ready and calendar_screen and not locked)
        self.year_chips.enable(ready and calendar_screen and not locked)
        self.period_chips.enable(ready and not calendar_screen and screen != 'carga' and not locked)
        self.keep_chips.enable(ready and not calendar_screen and not locked)
        # Fechas: aviso en el lugar si están mal escritas o invertidas.
        reason, bad = '', []
        if dates:
            parsed = {}
            for field, var in ((self.from_field, self.from_date), (self.to_field, self.to_date)):
                try:
                    parsed[field] = self.ui.parse_date(var.get())
                except ValueError:
                    bad.append(field)
            if bad:
                reason = 'Fecha DD/MM/AAAA'
            elif parsed[self.from_field] > parsed[self.to_field]:
                bad, reason = [self.from_field, self.to_field], '«Desde» es posterior a «Hasta»'
        for field in (self.from_field, self.to_field):
            field.error(field in bad)
        self.date_error.configure(text=reason)

    def apply_theme(self, save=True):
        from apariencia import apply

        choice = self.theme.get()
        if choice not in ('Claro', 'Oscuro', 'Sistema'):
            choice = 'Sistema'
            self.theme.set(choice)
        palette = apply(self.root, choice)
        if palette:
            self._kit_styles(palette)
        if save:
            self.prefs.data['tema'] = choice
            self.prefs.save()

    def _kit_styles(self, p):
        """Barra superior y fichas del kit común con la paleta clara u oscura vigente."""
        style = ttk.Style(self.root)
        bar, text, muted = p.get('heading', p['background']), p['text'], p.get('muted', p['text'])
        style.configure('Bar.TFrame', background=bar)
        style.configure('Name.TLabel', background=bar, foreground=text, font=('Segoe UI', 10, 'bold'))
        style.configure('BarMuted.TLabel', background=bar, foreground=muted)
        style.configure('Menu.TMenubutton', background=bar, foreground=text, padding=(8, 2), relief='flat')
        style.configure('Chip.Toolbutton', background=p['field'], foreground=text, padding=(10, 3), relief='flat')
        chip = [('disabled', p['background']), ('selected', p['primary']), ('active', bar), ('!selected', p['field'])]
        style.map(
            'Chip.Toolbutton',
            background=chip,
            bordercolor=chip,
            lightcolor=chip,
            darkcolor=chip,
            foreground=[('disabled', muted), ('selected', p['on_primary'])],
        )
        for tone in ('Info', 'Ok', 'Warn', 'Bad'):
            style.configure(f'Banner{tone}.TLabel', foreground=self.ui.PALETTE['fg'])
        style.configure('Link.TButton', background=bar)
        style.configure('Hint.TLabel', background=p['background'], foreground=muted)
        self.topbar.dot.configure(background=bar)
        style.configure('Side.Toolbutton', background=p['background'], foreground=text, anchor='w', padding=(10, 5))
        side = [('selected', bar), ('active', bar), ('!selected', p['background'])]
        style.map(
            'Side.Toolbutton',
            background=side,
            bordercolor=side,
            lightcolor=side,
            darkcolor=side,
            font=[('selected', ('Segoe UI', 10, 'bold'))],
        )
        style.configure('Group.TLabel', background=p['background'], foreground=muted, font=('Segoe UI', 9))

    def toggle_pause(self):
        if self.cancel.paused:
            self.cancel.resume()
            self.status.set('Descarga reanudada.')
            self.pause_button.configure(text='Pausar / continuar')
            self.live_pause.configure(text='Pausar')
            if self._pause_started is not None:
                self._phase_paused += time.monotonic() - self._pause_started
                self._pause_started = None
            self.progress_detail.set('Continuando · tiempo restante: estimando…')
        else:
            self.cancel.pause()
            self.status.set('Pausa solicitada: se completará la página en curso.')
            self.pause_button.configure(text='Continuar descarga')
            self.live_pause.configure(text='Continuar')
            self._pause_started = time.monotonic()
            self.progress_detail.set('Pausa solicitada · continuará en el límite seguro de la consulta.')

    def emit(self, name, value):
        self.events.put((name, value))

    def toggle_history(self):
        if self.read_history.get():
            self.history_package_choice.grid()
            self.status.set(
                'Se abrirá una historia por causa (no por registro), hasta 600 por lote. La consulta tardará más.'
            )
        else:
            self.history_package_choice.grid_remove()
        self.save_preferences()

    def months(self):
        try:
            value = int(self.months_back.get())
        except ValueError:
            return 4
        return value if 1 <= value <= 4 else 4

    def save_preferences(self):
        self.prefs.data.update(
            destino=self.dest.get(),
            tribunales=None if self.selected_tribunals is None else list(self.selected_tribunals),
            aviso_sonoro=self.sound.get(),
            paquete_consolidado=self.package_consolidated.get(),
            reutilizar_bitacoras=self.reuse_diaries.get(),
            leer_calendarios=self.read_calendars.get(),
            leer_historia=self.read_history.get(),
            historia_en_paquete=self.history_in_package.get(),
            informes_por_vencer=self.reports_due.get(),
            paquete_por_hoja=self.package_per_sheet.get(),
            pdf_sin_registros=self.empty_pdf.get(),
            meses_atras=self.months(),
        )
        try:
            self.prefs.save()
        except OSError:
            self.status.set('E_CONFIG: no se pudo guardar la configuración local.')

    def _paced_batch_runner(self, *args, **kwargs):
        return BatchRunner(
            *args, pause=self.prefs.data['pausa_consultas'], evidencia=self.prefs.data['pdf_sin_registros'], **kwargs
        )

    def configure_pause(self):
        from preferencias import PAUSA_MAXIMA_S, PAUSA_MINIMA_S, clamp_pause

        value = simpledialog.askfloat(
            'Pausa entre consultas',
            f'Segundos entre bitácoras y entre páginas ({PAUSA_MINIMA_S:g} a {PAUSA_MAXIMA_S:g}):',
            parent=self.root,
            initialvalue=self.prefs.data['pausa_consultas'],
            minvalue=PAUSA_MINIMA_S,
            maxvalue=PAUSA_MAXIMA_S,
        )
        if value is None:
            return
        self.prefs.data['pausa_consultas'] = clamp_pause(value)
        self.save_preferences()
        self.status.set(f"Pausa entre consultas: {self.prefs.data['pausa_consultas']:g} s.")

    def configure_csmp(self):
        executable = filedialog.askopenfilename(
            parent=self.root,
            title='CSMP experimental: ejecutable o Python de su entorno',
            filetypes=[('Ejecutables', '*.exe')],
        )
        if executable:
            self.prefs.data['csmp'] = executable
            self.save_preferences()

    def save_favorite(self):
        if self.busy:
            return
        try:
            batch = self.batch()
            batch.plan(self.catalog)
            name = simpledialog.askstring(
                'Favorito', 'Nombre del perfil (fechas recalculadas al aplicarlo):', parent=self.root
            )
            if name:
                offset = 0
                if batch.screen.startswith('calendario_'):
                    today = m.now().date()
                    offset = int(batch.year) * 12 + int(batch.month) - today.year * 12 - today.month
                    if offset not in (0, 1):
                        raise m.PocError('Los perfiles automáticos admiten el mes actual o siguiente.')
                self.prefs.favorite(name, batch, calendar_offset=offset)
                self.status.set('Perfil guardado; recalcula el período al aplicarlo.')
        except (m.PocError, OSError):
            self.status.set('Carga las opciones y revisa el alcance; no se guardan filtros privados.')

    def apply_batch(self, batch):
        self.select_tribunals(batch.tribunals)
        self.select_modalities(batch.modalities)
        self.tab.set(next((k for k, v in TAB_OPTIONS.items() if v == batch.tabs), 'Espera y Cumplimiento'))
        self.order_state.set(self.catalog.get('estados', {}).get(batch.state, ''))
        self.month.set(self.catalog.get('meses', {}).get(batch.month, ''))
        self.year.set(self.catalog.get('anios', {}).get(batch.year, ''))
        choices = self.catalog.get('informes', {}) if batch.tabs == ('Informes',) else self.catalog.get('medidas', {})
        self.report.set(choices.get(batch.report, 'Todas las medidas'))
        self.keep_filters.set(False)
        self.use_dates.set(batch.dates is not None)
        if batch.dates:
            self.from_date.set(batch.dates.start)
            self.to_date.set(batch.dates.end)
        self.refresh()

    def choose_favorite(self):
        if self.busy or not self.catalog:
            return
        popup = tk.Toplevel(self.root)
        popup.title('Consultas favoritas')
        popup.transient(self.root)
        choice = tk.StringVar()
        box = ttk.Combobox(popup, textvariable=choice, values=list(self.prefs.data['favoritos']), state='readonly')
        box.pack(padx=16, pady=16)

        def apply():
            try:
                name = choice.get()
                value = self.prefs.data['favoritos'][name]
                if value['screen'] != SCREENS[self.screen.get()]:
                    raise m.PocError('Selecciona la pantalla del favorito y carga sus opciones antes de aplicarlo.')
                batch, missing = self.prefs.restore(name, self.catalog, today=m.now().date())
                self.clear_pending()
                self.apply_batch(batch)
                self.status.set(
                    'Favorito aplicado; revisa el alcance y las fechas.'
                    + (' Opciones retiradas: ' + ', '.join(missing) if missing else '')
                )
                popup.destroy()
            except (m.PocError, KeyError, ValueError) as exc:
                messagebox.showerror('Favorito', str(exc), parent=popup)

        ttk.Button(popup, text='Aplicar', command=apply).pack(pady=8)

    def show_results(self):
        if self.busy:
            self.status.set('Espera a que termine la operación para trabajar con resultados.')
            return
        from ventana_resultados import ResultsWindow

        ResultsWindow(self)

    def open_selected(self, event=None):
        selected = self.tree.selection()
        if selected and self.last_folder:
            from ventana_resultados import open_local
            from resultados import inside

            try:
                open_local(inside(self.last_folder, self.tree.item(selected[0], 'values')[2]))
            except (m.PocError, OSError) as exc:
                self.status.set(str(exc))

    def clear_pending(self):
        self.recovery = None
        self.download_button.configure(text='Descargar el lote completo')
        self.refresh()

    def prepare_pending(self, folder):
        from resultados import read_json
        from perfiles import Selection, selection_profile

        planned = pending_plan(folder)
        if not planned:
            raise m.PocError('No hay consultas pendientes.')
        summary = read_json(Path(folder) / 'resumen.json')
        screen = planned[0]['screen']
        if not self.catalog or SCREENS[self.screen.get()] != screen:
            raise m.PocError('Selecciona la pantalla del lote y carga sus opciones primero.')
        keys = Selection.__dataclass_fields__
        selections = [Selection(**{k: v for k, v in s.items() if k in keys}) for s in planned]
        for s in selections:
            selection_profile(s, self.catalog)
        s = selections[0]
        dates = summary.get('filtro_fecha')
        batch = Batch(
            (s.tribunal,),
            (s.modality,) if s.modality else (),
            (s.tab,) if s.tab else (),
            report=s.report or '0',
            dates=Dates(dates['desde'], dates['hasta']) if dates else None,
            screen=screen,
            state=s.state or '',
            month=s.month,
            year=s.year,
        )
        self.recovery = (batch, selections)
        self.download_button.configure(text='Descargar consultas pendientes')
        self.status.set(
            f'{len(selections)} consultas pendientes preparadas. Se descargarán desde página 1 en un lote nuevo.'
        )
        self.refresh()

    def _side_selected(self, label):
        if label != self.screen.get():
            self.screen.set(label)
            self.change_screen()

    def change_screen(self):
        self.catalog = None
        self.refresh()
        if getattr(self, 'auto_catalog', False) and self.connected and not self.busy:
            self.status.set('Abriendo la consulta elegida en SITFA…')
            self.enqueue('catalog')
        else:
            self.status.set(
                f'Elige «{RELOAD}» para preparar la consulta elegida.'
                if self.connected
                else 'Conecta tu Chrome para cargar las opciones de la consulta elegida.'
            )

    def select_tribunals(self, ids):
        if not self.catalog or set(ids) - set(self.catalog['tribunales']):
            raise m.PocError('La selección contiene tribunales no disponibles.')
        self.selected_tribunals = tuple(k for k in self.catalog['tribunales'] if k in ids)
        self.tribunal.set(
            'Ningún tribunal marcado'
            if not ids
            else (self.catalog['tribunales'][ids[0]] if len(ids) == 1 else f'{len(ids)} tribunales marcados')
        )
        self.refresh()

    def available_modalities(self):
        return {k: MODALITIES[k] for k in (self.catalog or {}).get('modalidades', {}) if k in MODALITIES}

    def select_modalities(self, ids):
        available = self.available_modalities()
        if set(ids) - set(available):
            raise m.PocError('La selección contiene modalidades no disponibles.')
        chosen = tuple(k for k in available if k in ids)
        # Todas marcadas equivale a «Todas las modalidades»: el lote sigue las que ofrezca SITFA.
        self.selected_modalities = None if set(chosen) == set(available) else chosen
        if self.selected_modalities is None:
            text = ALL_MODALITIES
        elif not chosen:
            text = 'Ninguna modalidad marcada'
        elif len(chosen) == 1:
            text = available[chosen[0]]
        else:
            text = f'{len(chosen)} modalidades marcadas: ' + ', '.join(available[k] for k in chosen)
        self.modality.set(text)
        self.refresh()

    def choose_modalities(self):
        if self.catalog and not self.busy:
            available = self.available_modalities()
            TribunalPicker(
                self.root,
                available,
                tuple(available) if self.selected_modalities is None else self.selected_modalities,
                self.select_modalities,
                title='Elegir modalidades',
                prompt='Marca las modalidades que quieres incluir.',
                counted='modalidades seleccionadas',
            )

    def choose_tribunals(self):
        if self.catalog and not self.busy:
            TribunalPicker(self.root, self.catalog['tribunales'], self.selected_tribunals or (), self.select_tribunals)

    def setup(self):
        from contrato_integral import extension_dir

        directory = extension_dir()
        if not (directory / 'manifest.json').is_file():
            # Sin la carpeta, Chrome recibiría otra (p. ej. Descargas, con _internal) y la rechazaría.
            messagebox.showerror(
                'Falta la carpeta extension',
                'No se encontró la carpeta extension de esta entrega en:\n\n'
                + str(directory)
                + '\n\nDescomprime el paquete completo en una carpeta propia (no sueltos en Descargas) y abre el '
                'Descargador desde ahí.',
                parent=self.root,
            )
            return
        self.root.clipboard_clear()
        self.root.clipboard_append(str(directory))
        self.root.update()
        try:
            open_chrome('chrome://extensions/')
        except m.PocError as exc:
            # Sin Chrome en las rutas habituales: las instrucciones siguen sirviendo si se abre a mano.
            self.status.set(f'{exc} Abre chrome://extensions a mano.')
        messagebox.showinfo(
            'Instalación única en tu Chrome',
            'Activa Modo de desarrollador y pulsa Cargar extensión sin empaquetar.\n\nElige exactamente esta carpeta (su ruta '
            'está copiada al portapapeles; pégala en la barra de dirección):\n\n'
            + str(directory)
            + '\n\nNo elijas Descargas ni la carpeta del programa: Chrome rechaza las que contienen _internal.\n\n'
            'Después entra a Seguimiento y pulsa el icono de SITFA · Descargador local.',
            parent=self.root,
        )

    def copy_code(self):
        self.root.clipboard_clear()
        self.root.clipboard_append(self.code.get())
        self.root.update()
        self._mark_step(2)
        self.status.set('Código copiado. Pégalo en la pestaña de conexión de la extensión.')

    def batch(self):
        if not self.catalog:
            raise m.PocError('Carga las opciones de Seguimiento primero.')
        tribunals = self.selected_tribunals or ()
        screen = SCREENS[self.screen.get()]

        def code(key, variable):
            return next((k for k, v in self.catalog.get(key, {}).items() if v == variable.get()), None)

        if screen != 'seguimiento':
            return Batch(
                tribunals,
                (),
                (),
                dates=Dates(self.from_date.get(), self.to_date.get())
                if screen == 'carga' or self.use_dates.get() and screen == 'litigantes'
                else None,
                keep_filters=self.keep_filters.get() if screen == 'litigantes' else False,
                screen=screen,
                state=code('estados', self.order_state) or '',
                month=code('meses', self.month),
                year=code('anios', self.year),
            )
        available = self.available_modalities()
        modalities = (
            tuple(available)
            if self.selected_modalities is None
            else tuple(k for k in available if k in self.selected_modalities)
        )
        if not modalities:
            raise m.PocError('Marca al menos una modalidad.')
        report_options = self.catalog['informes'] if self.tab.get() == 'Informes' else self.catalog.get('medidas', {})
        reports = [k for k, v in report_options.items() if v == self.report.get()]
        return Batch(
            tribunals,
            modalities,
            TAB_OPTIONS[self.tab.get()],
            reports[0] if reports else ('1' if self.tab.get() == 'Informes' else '0'),
            Dates(self.from_date.get(), self.to_date.get()) if self.use_dates.get() else None,
            self.keep_filters.get(),
        )

    def refresh(self):
        if hasattr(self, 'topbar'):
            self.topbar.set_state(
                'Chrome conectado' if self.connected else 'Sin conexión', 'ok' if self.connected else 'muted'
            )
            self.connect_box.grid_remove() if self.connected else self.connect_box.grid()
            self.form.grid() if self.connected else self.form.grid_remove()
        idle = not self.busy and not self.quitting
        ready = idle and self.connected and self.catalog is not None
        screen = SCREENS[self.screen.get()]
        followup = screen == 'seguimiento'
        calendar_screen = screen.startswith('calendario_')
        self.screen_combo.configure(state='readonly' if idle and self.connected else 'disabled')
        if hasattr(self, 'side'):
            if self.side.get() != self.screen.get():
                self.side.select(self.screen.get())
            self.side.enable(idle and self.connected and not self.recovery)
        for button, enabled in [
            (self.open_button, idle),
            (self.setup_button, idle),
            (self.load_button, idle and self.connected),
            (self.disconnect_button, idle and self.connected),
            (self.copy_button, idle),
            (self.browse_button, idle),
            (self.download_button, ready and bool(self.selected_tribunals)),
            (self.pipeline_button, ready and followup and bool(self.selected_tribunals) and self.recovery is None),
            (self.joint_button, ready and followup and bool(self.selected_tribunals)),
            (self.diary_button, ready and followup and bool(self.selected_tribunals)),
            (self.diary_retry_button, ready and followup),
            (self.trib_button, ready),
            (
                self.cancel_button,
                self.busy
                and getattr(self, 'operation', None)
                in ('download', 'joint', 'resume_joint', 'diary', 'pipeline', 'resume_pipeline')
                and not self.quitting,
            ),
            (
                self.pause_button,
                self.busy
                and getattr(self, 'operation', None)
                in ('download', 'joint', 'resume_joint', 'diary', 'pipeline', 'resume_pipeline')
                and not self.quitting,
            ),
            (self.folder_button, self.last_folder is not None and not self.quitting),
            (
                self.package_button,
                idle and self.last_sheets_folder is not None and self.last_diary_manifest is not None,
            ),
            (self.date_check, ready and not calendar_screen),
            (self.filter_check, ready and not calendar_screen),
        ]:
            button.configure(state='normal' if enabled else 'disabled')
        self.package_choice.configure(state='normal' if idle else 'disabled')
        self.live_cancel.configure(state=self.cancel_button.cget('state'))
        self.live_pause.configure(state=self.pause_button.cget('state'))
        package_ready = bool(self.last_package and self.last_package.is_file() and not self.busy)
        self.copy_path_button.configure(state='normal' if package_ready else 'disabled')
        self.package_folder_button.configure(state='normal' if package_ready and not self.quitting else 'disabled')
        self.mod_combo.configure(state='normal' if ready and followup else 'disabled')
        self.tab_combo.configure(state='readonly' if ready and followup else 'disabled')
        self.state_combo.configure(state='readonly' if ready and screen == 'litigantes' else 'disabled')
        for combo in (self.month_combo, self.year_combo):
            combo.configure(state='readonly' if ready and calendar_screen else 'disabled')
        if self.catalog and followup:
            values = (
                list(self.catalog['informes'].values())
                if self.tab.get() == 'Informes'
                else ['Todas las medidas', *self.catalog.get('medidas', {}).values()]
            )
            self.report_combo.configure(values=values)
            if self.report.get() not in values and values:
                self.report.set(values[0])
        self.report_combo.configure(
            state='readonly' if ready and followup and self.tab.get() != 'Espera' else 'disabled'
        )
        for widget in self.date_widgets:
            widget.configure(
                state='normal'
                if ready and not calendar_screen and (screen == 'carga' or self.use_dates.get())
                else 'disabled'
            )
        if self.catalog:
            try:
                self.preview.set(
                    f'{len(self.recovery[1] if self.recovery else self.batch().plan(self.catalog))} consultas · '
                    + (
                        'pendientes del lote anterior (desde página 1)'
                        if self.recovery
                        else 'una descarga mensual por tribunal'
                        if calendar_screen
                        else 'todas sus páginas en orden'
                    )
                )
            except Exception:
                self.preview.set('Revisa las opciones y el rango de fechas.')
        else:
            self.preview.set('Carga las opciones de tu sesión para preparar el lote.')
        if self.recovery:
            for widget in (
                self.screen_combo,
                self.mod_combo,
                self.tab_combo,
                self.state_combo,
                self.month_combo,
                self.year_combo,
                self.report_combo,
                self.trib_button,
                self.date_check,
                self.filter_check,
                *self.date_widgets,
            ):
                widget.configure(state='disabled')
        self._sync_chips(ready, screen)
        # Una sola acción principal: crear el paquete (Seguimiento) o descargar (otras consultas y pendientes).
        if followup and not self.recovery:
            self.pipeline_button.configure(text='Consultar y crear paquete')
        else:
            self.pipeline_button.configure(
                text='Descargar consultas pendientes' if self.recovery else 'Descargar',
                state=str(self.download_button.cget('state')),
            )
        working = self._cancellable()
        for button in (self.live_pause, self.live_cancel):
            if working and not button.winfo_manager():
                button.pack(side='right', padx=(0, 8), before=self.pipeline_button)
            elif not working:
                button.pack_forget()
        self.folder_link.configure(state='normal' if idle else 'disabled')
        self.setup_button.configure(state='normal' if idle else 'disabled')

    def toggle_advanced(self):
        if self.advanced_actions.winfo_manager():
            self.advanced_actions.grid_remove()
        else:
            self.advanced_actions.grid()

    def open_package_folder(self):
        if self.last_package and self.last_package.is_file() and not self.busy:
            from ventana_resultados import open_local

            open_local(self.last_package.parent)

    def copy_package_path(self):
        if self.last_package and self.last_package.is_file() and not self.busy:
            self.root.clipboard_clear()
            self.root.clipboard_append(str(self.last_package.resolve()))
            self.status.set('Ruta del paquete copiada.')

    def start_pipeline(self):
        if self.busy or self.quitting:
            return
        try:
            batch = self.batch()
            if batch.screen != 'seguimiento' or self.recovery is not None:
                raise m.PocError(
                    'Para crear paquete usa Seguimiento; recuperaciones y otras consultas están en Avanzado.'
                )
            batch.plan(self.catalog)
            destination = Path(self.dest.get()).resolve()
            destination.mkdir(parents=True, exist_ok=True)
        except (m.PocError, OSError, KeyError, TypeError) as exc:
            messagebox.showerror('Revisa el alcance del paquete', str(exc), parent=self.root)
            return
        self.cancel.clear()
        self.last_folder = self.last_package = None
        self.last_sheets_folder = self.last_diary_manifest = None
        self.result_notice.set('')
        self.history_counts = {'leidas': 0, 'fallidas': 0}
        self.tree.delete(*self.tree.get_children())
        self.save_preferences()
        self.enqueue('pipeline', (batch, destination, self.package_consolidated.get()))

    def resume_pipeline(self, folder=None):
        """Continúa un paquete cortado: no repite consultas validadas ni bitácoras ya leídas."""
        if self.busy or self.quitting:
            return
        if not (self.connected and self.catalog):
            self.status.set('Conecta Chrome y carga las opciones de Seguimiento antes de retomar un paquete.')
            self.banner.show(
                'Conecta Chrome (Seguimiento) y vuelve a pulsar Retomar.',
                kind='warn',
                action=lambda: self.resume_pipeline(folder),
                action_text='Retomar',
            )
            return
        folder = folder or filedialog.askdirectory(parent=self.root, title='Carpeta «Paquete CSMP …» a retomar')
        if not folder:
            return
        if not (Path(folder) / 'respaldo' / 'proceso.json').is_file():
            messagebox.showerror(
                'Retomar paquete',
                'Elige una carpeta «Paquete CSMP …» (contiene respaldo/proceso.json).',
                parent=self.root,
            )
            return
        self.cancel.clear()
        self.last_folder = self.last_package = None
        self.last_sheets_folder = self.last_diary_manifest = None
        self.result_notice.set('')
        self.history_counts = {'leidas': 0, 'fallidas': 0}
        self.tree.delete(*self.tree.get_children())
        self.status.set('Retomando el paquete: se reutiliza lo ya validado y se leen solo las bitácoras pendientes.')
        self.enqueue('resume_pipeline', (Path(folder), self.package_consolidated.get()))

    def create_package(self):
        """Paquete del Asistente con el lote de hojas y las bitácoras recién leídas, sin abrir Resultados."""
        if self.busy or not (self.last_sheets_folder and self.last_diary_manifest):
            return
        target = filedialog.asksaveasfilename(
            parent=self.root,
            title='Guardar paquete para el Asistente',
            defaultextension='.zip',
            initialfile=f'Captura_CSMP_{m.now():%Y-%m-%d_%H%M}.zip',
            filetypes=[('Paquete CSMP', '*.zip')],
            confirmoverwrite=False,
        )
        if not target:
            return
        if Path(target).exists():
            self.status.set('Ese archivo ya existe. Elige un nombre nuevo para no reemplazar otro paquete.')
            return
        self.enqueue(
            'package',
            (
                self.last_sheets_folder,
                self.last_diary_manifest,
                Path(target),
                self.package_consolidated.get(),
                None,
                (),
                self.months(),
            ),
        )

    def enqueue(self, command, arg=None):
        if self.busy or self.quitting:
            return
        if command == 'catalog':
            arg = SCREENS[self.screen.get()]
        self.busy = True
        self.operation = command
        if command in ('download', 'joint', 'resume_joint', 'diary', 'package', 'pipeline', 'resume_pipeline'):
            self._phase_started = time.monotonic()
            self._phase_paused = 0.0
            self._pause_started = None
            self.phase_name.set(
                {
                    'download': 'Consultar SITFA',
                    'joint': 'Descarga conjunta',
                    'resume_joint': 'Recuperar conjunto',
                    'diary': 'Leer bitácoras',
                    'package': 'Crear paquete',
                    'pipeline': 'Preparar paquete',
                    'resume_pipeline': 'Retomar paquete',
                }[command]
            )
            self.progress_detail.set('Operación iniciada · tiempo restante: estimando…')
            self.results_box.grid()
            self.result_notice.set('')
        self.refresh()
        self.commands.put((command, arg))

    def browse(self):
        directory = filedialog.askdirectory(parent=self.root, title='Carpeta de descarga')
        if directory:
            self.dest.set(directory)
            self.save_preferences()

    def start_download(self):
        try:
            batch = self.recovery[0] if self.recovery else self.batch()
            batch.plan(self.catalog)
            destination = Path(self.dest.get()).resolve()
            destination.mkdir(parents=True, exist_ok=True)
        except (m.PocError, OSError, KeyError) as exc:
            messagebox.showerror(
                'Revisa el lote',
                str(exc) if isinstance(exc, m.PocError) else 'Seleccione una carpeta local accesible.',
                parent=self.root,
            )
            return
        self.cancel.clear()
        self.last_folder = None
        self.tree.delete(*self.tree.get_children())
        self.progress.configure(value=0, maximum=1)
        self.count.set('Preparando el lote…')
        self.status.set('Descargando en tu sesión Chrome…')
        self.save_preferences()
        self.enqueue('download', (batch, destination, self.recovery[1] if self.recovery else None))

    def request_cancel(self):
        self.cancel.set()
        self.status.set('Se detendrá antes de la siguiente consulta.')
        self.cancel_button.configure(state='disabled')

    def start_diary(self, resume=None):
        try:
            from lectura_bitacoras import saved_batch, load

            batch = saved_batch(load(Path(resume) / 'bitacoras.json')) if resume else self.batch()
            if batch.screen != 'seguimiento':
                raise m.PocError('Selecciona Seguimiento para leer bitácoras.')
            batch.plan(self.catalog)
            destination = Path(self.dest.get()).resolve()
            destination.mkdir(parents=True, exist_ok=True)
            if self.bitacoras_reply and resume and not Path(resume).resolve().is_relative_to(destination):
                raise m.PocError('Para devolver a CSMP, recupera un lote de la carpeta de esta solicitud.')
        except (m.PocError, OSError, ValueError, KeyError) as exc:
            messagebox.showerror('Revisa el lote de bitácoras', str(exc), parent=self.root)
            return
        self.cancel.clear()
        self.tree.delete(*self.tree.get_children())
        self.last_folder = None
        self.status.set('Leyendo y copiando bitácoras. Las fallas quedan en el control del lote.')
        self.save_preferences()
        self.enqueue('diary', (batch, destination, resume))

    def resume_diary(self):
        folder = filedialog.askdirectory(parent=self.root, title='Carpeta del lote de bitácoras a reintentar')
        if folder:
            self.start_diary(folder)

    def start_joint(self):
        try:
            batch = self.batch()
            if batch.screen != 'seguimiento' or len(batch.tabs) != 1 or batch.tabs[0] not in ('Espera', 'Cumplimiento'):
                raise m.PocError('Selecciona Espera o Cumplimiento para la descarga conjunta.')
            self.check_csmp_mode(batch.tabs[0])
            from flujo_csmp import plan

            plan(batch.tribunals, batch.modalities, batch.tabs[0], m.now().date())
            destination = Path(self.dest.get()).resolve()
            destination.mkdir(parents=True, exist_ok=True)
        except (m.PocError, ValueError, tk.TclError, OSError) as exc:
            messagebox.showerror('Revisa el flujo conjunto', str(exc), parent=self.root)
            return
        self.cancel.clear()
        self.tree.delete(*self.tree.get_children())
        self.last_folder = None
        self.status.set(
            'Descargando principal completa e informes del mes actual y siguiente. Al terminar se entrega el libro a CSMP.'
        )
        self.save_preferences()
        self.enqueue('joint', (batch.tribunals, batch.modalities, batch.tabs[0], destination))

    def check_csmp_mode(self, mode):
        if self.csmp_reply and self.csmp_mode and mode.upper() != self.csmp_mode:
            raise m.PocError(
                'CSMP solicitó '
                + self.csmp_mode
                + '. Selecciona ese registro o inicia una nueva descarga desde CSMP con el modo que necesitas.'
            )

    def resume_joint(self, folder=None):
        if self.busy or not self.connected:
            self.status.set('Conecta Chrome y termina la operación actual antes de retomar un flujo.')
            return
        folder = folder or filedialog.askdirectory(parent=self.root, title='Elegir carpeta de descarga conjunta')
        if not folder:
            return
        try:
            from resultados import read_json

            data = read_json(Path(folder) / 'flujo.json')
            s = data['seleccion']
            self.check_csmp_mode(s['modo'])
            from flujo_csmp import plan

            cutoff = date.fromisoformat(s['corte'])
            plan(tuple(s['tribunales']), tuple(s['modalidades']), s['modo'], cutoff, s.get('dias_firmas', 60))
        except (m.PocError, ValueError, KeyError, OSError) as exc:
            self.status.set('No se puede recuperar el flujo: ' + str(exc))
            return
        self.cancel.clear()
        self.tree.delete(*self.tree.get_children())
        self.enqueue('resume_joint', (s, Path(folder)))

    def open_folder(self):
        if self.last_folder and self.last_folder.exists():
            from ventana_resultados import open_local

            open_local(self.last_folder)

    def close_app(self):
        if self.quitting:
            return
        if any(window.winfo_exists() and window.busy for window in self.result_windows):
            self.status.set('Espera a que termine la escritura de resultados antes de cerrar.')
            return
        self.quitting = True
        self.cancel.set()
        self.status.set('Terminando la página en curso y cerrando la conexión local…')
        self.refresh()
        self.commands.put(('exit', None))

    def worker(self):
        while True:
            command, arg = self.commands.get()
            try:
                if command in ('download', 'joint', 'resume_joint', 'pipeline', 'resume_pipeline'):
                    from contrato_integral import verify_extension

                    verify_extension(self.bridge)
                if command == 'open':
                    open_chrome(m.START)
                    self.emit('opened', None)
                    self.emit('status', 'SITFA abierto en tu Chrome. Entra a Seguimiento y conecta su extensión.')
                elif command == 'catalog':
                    self.emit('catalog', self.bridge.call('catalog', {'screen': arg}))
                    try:
                        self.emit('extension_caps', self.bridge.call('capabilities'))
                    except m.PocError:
                        pass
                elif command == 'download':
                    self._paced_batch_runner(self.bridge, self.catalog, self.emit, self.cancel).run(*arg)
                elif command == 'package':
                    self.emit('package_done', package_current_lot(*arg))
                elif command in ('pipeline', 'resume_pipeline'):
                    from flujo_paquete import retomar, run, run_por_hoja

                    if command == 'resume_pipeline':
                        funcion = retomar
                    else:
                        funcion = run_por_hoja if self.prefs.data.get('paquete_por_hoja') else run
                    funcion(
                        self.bridge,
                        self.catalog,
                        self.emit,
                        self.cancel,
                        *arg,
                        pause=self.prefs.data['pausa_consultas'],
                        reuse=self.prefs.data['reutilizar_bitacoras'],
                        calendars=self.prefs.data.get('leer_calendarios', False),
                        historia=self.prefs.data['leer_historia'],
                        historia_en_paquete=self.prefs.data['historia_en_paquete'],
                        meses=self.prefs.data.get('meses_atras', 4),
                        informes=self.prefs.data.get('informes_por_vencer', False),
                        pdf_vacias=self.prefs.data['pdf_sin_registros'],
                    )
                elif command == 'diary':
                    from lectura_bitacoras import DiaryRunner

                    batch, destination, resume = arg
                    reuse_base = destination if self.prefs.data['reutilizar_bitacoras'] and not resume else None
                    historia = None
                    if resume and self.prefs.data['leer_historia']:
                        anterior = Path(resume) / 'historia' / 'historias.json'
                        if anterior.is_file():
                            from lectura_historia import LectorHistoria

                            historia = LectorHistoria(
                                self.bridge,
                                self.emit,
                                self.cancel,
                                pause=self.prefs.data['pausa_consultas'],
                                anterior=anterior,
                                meses=self.prefs.data.get('meses_atras', 4),
                            )
                    DiaryRunner(
                        self.bridge,
                        self.catalog,
                        self.emit,
                        self.cancel,
                        pause=self.prefs.data['pausa_consultas'],
                        reuse_base=reuse_base,
                        calendars=self.prefs.data.get('leer_calendarios', False),
                        **({'historia': historia} if historia else {}),
                    ).run(batch, destination, resume)
                elif command == 'joint':
                    from flujo_csmp import JointRunner

                    JointRunner(self.bridge, self.emit, self.cancel, runner_factory=self._paced_batch_runner).run(*arg)
                elif command == 'resume_joint':
                    from flujo_csmp import JointRunner

                    selection, folder = arg
                    JointRunner(self.bridge, self.emit, self.cancel, runner_factory=self._paced_batch_runner).run(
                        tuple(selection['tribunales']),
                        tuple(selection['modalidades']),
                        selection['modo'],
                        folder.parent,
                        selection.get('dias_firmas', 60),
                        today=date.fromisoformat(selection['corte']),
                        resume=folder,
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
                retomable = command in ('pipeline', 'resume_pipeline') and 'Retomar paquete' not in str(exc)
                self.emit(
                    'error',
                    'E_'
                    + command.upper()
                    + ': '
                    + str(exc)
                    + (' Para continuar sin repetir lo ya hecho: Más › Retomar paquete….' if retomable else ''),
                )
            except Exception as exc:
                if type(exc).__module__ == 'nurus.capture.contract' and type(exc).__name__ == 'CaptureError':
                    # Mensajes fijos del contrato de captura, sin datos de causas: dicen qué rechazó el paquete.
                    self.emit('error', 'E_' + command.upper() + ': El paquete no pasó la validación: ' + str(exc))
                else:
                    self.emit(
                        'error',
                        'E_'
                        + command.upper()
                        + ': No se pudo completar la operación. Revisa la conexión, SITFA y la carpeta. Se conservan los archivos ya verificados.',
                    )
            finally:
                self.emit('idle', None)

    def process_event(self, name, value):
        if name == 'opened':
            self._mark_step(1)
            return True
        if name == 'pipeline_folder':
            self.last_folder = Path(value)
            self.refresh()
            return True
        if name == 'pipeline_source':
            from archivo_local import remember

            remember(self.prefs, value['folder'])
            if value['fase'] == 'consultas':
                self.last_sheets_folder = Path(value['folder'])
            elif value['fase'] == 'bitacoras':
                self.last_diary_manifest = Path(value['folder']) / 'bitacoras.json'
            return True
        if name == 'phase':
            self.phase_name.set(
                {
                    'consultas': '1 Consultar SITFA',
                    'bitacoras': '2 Leer bitácoras',
                    'informes': '3 Informes por vencer',
                    'paquete': '4 Crear paquete',
                }.get(value, value)
            )
            self._phase_started = time.monotonic()
            self._phase_paused = 0.0
            self.progress_detail.set('Fase iniciada · tiempo restante: estimando…')
            self.progress.stop()
            self.progress.configure(value=0, maximum=1, mode='indeterminate')
            self.progress.start()
            return True
        if name in ('diary_done', 'joint_done', 'done') and value.get('folder'):
            from archivo_local import remember

            try:
                remember(self.prefs, value['folder'])
            except (m.PocError, OSError, ValueError):
                pass
        if name == 'package_done':
            self.last_package = Path(value['path'])
            self.last_folder = Path(value['path']).parent
            if value.get('sheets_folder'):
                self.last_sheets_folder = Path(value['sheets_folder'])
                self.last_diary_manifest = Path(value['diary_manifest'])
            self.status.set(
                'Paquete creado: '
                + Path(value['path']).name
                + '. '
                + value['reuse']
                + ' Ábrelo en el Asistente con «Importar paquete».'
            )
            if value.get('partial'):
                self.status.set(self.status.get() + ' Cobertura parcial: revisa incidencias y fuentes antes de usarlo.')
            self.result_notice.set(self.status.get() + '\n' + str(self.last_package))
            self.phase_name.set('Paquete parcial · revisar' if value.get('partial') else 'Paquete creado')
            self.progress.stop()
            self.progress.configure(mode='determinate', maximum=1, value=1)
            self.progress_detail.set('Puedes abrir la carpeta o copiar la ruta del ZIP.')
            self.banner.show(
                'Paquete parcial: revisa incidencias antes de usarlo.' if value.get('partial') else 'Paquete creado.',
                kind='warn' if value.get('partial') else 'ok',
                action=self.open_package_folder,
                action_text='Abrir carpeta',
            )
            self.refresh()
            return True
        if name == 'diary_done':
            from lectura_bitacoras import MANIFEST

            manifest = Path(value['folder']) / MANIFEST
            self.last_diary_manifest = manifest if manifest.is_file() else None
            self.last_folder = Path(value['folder'])
            self.count.set(
                f"{value['leidas']} lecturas verificadas · {value['fallidas']} filas fallidas o sin vínculo · {value.get('consultas_fallidas', 0)} consultas fallidas"
            )
            self.context.set('Bitácoras · ' + value['estado'])
            self.status.set(
                ('Copias guardadas. ' if value['leidas'] else 'No se guardaron copias de bitácoras. ')
                + (value.get('detalle') or 'Abre Resultados para revisar o reutilizar este lote.')
            )
            if value.get('excel'):
                self.status.set(self.status.get() + ' Excel consolidado: ' + Path(value['excel']).name)
            if value.get('excel_error'):
                self.status.set(self.status.get() + ' Excel pendiente: ' + value['excel_error'])
            if self.bitacoras_reply:
                from lectura_bitacoras import write

                write(Path(self.bitacoras_reply), {**value, 'id': self.bitacoras_id})
            self.refresh()
            return True
        if name == 'extension_caps':
            soportada = isinstance(value, dict) and 'historia_lectura' in (value.get('protocolos') or {})
            estado = 'normal' if soportada else 'disabled'
            self.history_choice.configure(state=estado)
            self.history_package_choice.configure(state=estado)
            if not soportada:
                from contrato_integral import VERSION

                self.status.set(f'Actualiza la extensión a {VERSION} para leer la historia.')
            return True
        if name == 'history_progress':
            key = 'leidas' if value['estado'] in ('LEIDA', 'VACIA_COMPROBADA') else 'fallidas'
            self.history_counts[key] += 1
            self.status.set(
                f"Historia: {self.history_counts['leidas']} leídas · {self.history_counts['fallidas']} fallidas"
            )
            return True
        if name == 'history_done':
            self.status.set(
                f"Historia {value['estado']}: {value['leidas']} leídas · {value['fallidas']} fallidas · "
                f"{value['tramites_en_ventana']} trámites en el período recolectado"
            )
            return True
        if name == 'diary_progress':
            self.tree.insert(
                '', 'end', values=(value['pagina'], value['registros'], value['rit'] + ' · ' + value['estado'])
            )
            self.tree.yview_moveto(1)
            self.count.set(f'{len(self.tree.get_children())} filas revisadas en el lote')
            self.status.set('Bitácora ' + value['rit'] + ' · ' + value['estado'])
            return True
        if name == 'joint_done':
            self.catalog = None
            self.last_folder = Path(value['folder'])
            self.last_sheets_folder = Path(value['folder'])
            self.tree.delete(*self.tree.get_children())
            self.tree.insert('', 'end', values=('Final', '', Path(value['archivo']).name))
            self.status.set(
                'Libro preparado para CSMP: principal completa e informes de ambos meses. No se consultaron resoluciones firmadas.'
            )
            self.count.set('Descarga conjunta terminada')
            self.context.set(value['estado'])
            if self.csmp_reply:
                from flujo_csmp import write_state
                import hashlib

                path = Path(value['archivo'])
                write_state(
                    Path(self.csmp_reply),
                    {
                        'archivo': str(path),
                        'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                        'modo': value['modo'],
                    },
                )
            return True
        if name == 'connected':
            self.connected = True
            self.catalog = None
            self.status.set(value)
            # Sin botón «Cargar opciones»: al conectar se traen las opciones de la pantalla elegida.
            if getattr(self, 'auto_catalog', False):
                if self.busy:
                    self._catalog_pending = True  # se cargan al quedar libre (evento idle)
                else:
                    self.root.after(0, lambda: self.enqueue('catalog'))
        elif name == 'disconnected':
            self.connected = False
            self.catalog = None
            self.code.set(value)
            self.status.set('Desconectado. Tu Chrome sigue abierto; usa el nuevo código para reconectar.')
        elif name == 'catalog':
            self.catalog = value
            self.screen.set(next(k for k, v in SCREENS.items() if v == value.get('pantalla', 'seguimiento')))
            self.select_tribunals(
                tuple(value['tribunales'])
                if self.selected_tribunals is None
                else tuple(k for k in self.selected_tribunals if k in value['tribunales'])
            )
            for key, combo, variable, selection_key in [
                ('estados', self.state_combo, self.order_state, 'estado'),
                ('meses', self.month_combo, self.month, 'mes'),
                ('anios', self.year_combo, self.year, 'anio'),
            ]:
                choices = value.get(key, {})
                combo.configure(values=list(choices.values()))
                if variable.get() not in choices.values():
                    current = m.now().date()
                    preferred = (
                        str(current.month)
                        if key == 'meses'
                        else str(current.year)
                        if key == 'anios'
                        else value.get('seleccion', {}).get(selection_key)
                    )
                    variable.set(
                        choices.get(
                            preferred,
                            choices.get(
                                value.get('seleccion', {}).get(selection_key), next(iter(choices.values()), '')
                            ),
                        )
                    )
            self.report_combo.configure(values=list(value['informes'].values()))
            if value['informes']:
                self.report.set(next(iter(value['informes'].values())))
            available = self.available_modalities()
            if self.selected_modalities is not None and not set(self.selected_modalities) <= set(available):
                self.selected_modalities = None
            self.select_modalities(tuple(available) if self.selected_modalities is None else self.selected_modalities)
            self._fill_chips()
            self.status.set('Opciones cargadas. Elige el alcance del lote y, si corresponde, las fechas.')
        elif name == 'folder':
            self.last_folder = Path(value)
        elif name == 'query':
            self.query_index = value['index']
            self.query_total = value['total']
            self.context.set(f'Consulta {self.query_index}/{self.query_total} · {value["label"]}')
            self.root.title(f'{self.query_index} de {self.query_total} · {self.title}')
            self.status.set('Consultando SITFA…')
            self.count.set('Consultando…')
            self.progress.configure(value=0, maximum=1)
            self._phase_started = time.monotonic()
            self._phase_paused = 0.0
        elif name == 'total':
            self.progress.stop()
            self.progress.configure(maximum=value, value=0, mode='determinate')
        elif name == 'progress':
            self.tree.insert(
                '', 'end', values=(f'{value["pagina"]}/{value["total"]}', value['registros'], value['archivo'])
            )
            self.tree.yview_moveto(1)
            self.progress.configure(value=value['pagina'])
            self.count.set(f'{len(self.tree.get_children())} archivos guardados en el lote')
            self.status.set(
                f'Consulta {self.query_index}/{self.query_total} · Página {value["pagina"]}/{value["total"]} verificada · {value["registros"]} registros'
            )
        elif name == 'empty':
            self.tree.insert('', 'end', values=('Sin PDF', 0, value['consulta']))
            self.tree.yview_moveto(1)
        elif name == 'evidence':
            self.tree.insert('', 'end', values=('PDF', 0, value['archivo']))
            self.tree.yview_moveto(1)
            self.count.set(f'{len(self.tree.get_children())} archivos guardados en el lote')
        elif name == 'done':
            self.last_sheets_folder = self.last_folder
            self.status.set(
                f'Lote completo: {value["queries"]} consultas, {value["files"]} Excel, {value.get("pdfs", 0)} PDF, {value.get("empty", 0)} sin registros y {value["records"]} registros.'
            )
            self.count.set('Completado. Puedes abrir Resultados o iniciar otro lote.')
            self.context.set(self.last_folder.name if self.last_folder else 'Lote completo')
            self.recovery = None
            self.download_button.configure(text='Descargar el lote completo')
            if self.sound.get():
                self.root.bell()
        elif name in ('status', 'error', 'warning'):
            self.status.set(value)
            if name in ('error', 'warning'):
                self.result_notice.set(value)
            if 'pausa' in str(value).lower() or 'espera' in str(value).lower():
                self.progress_detail.set(str(value))
            if name == 'error' and self.sound.get():
                self.root.bell()
        elif name == 'idle':
            self.busy = False
            self.operation = None
            self.progress.stop()
            self.root.title(self.title)
            if getattr(self, '_catalog_pending', False):
                self._catalog_pending = False
                if self.connected and self.catalog is None:
                    self.root.after(0, lambda: self.enqueue('catalog'))
        elif name == 'exit':
            try:
                if self.root.clipboard_get() == self.code.get():
                    self.root.clipboard_clear()
            except tk.TclError:
                pass
            self.root.destroy()
            return False
        self.refresh()
        return True

    def poll(self):
        if self.busy and self._phase_started is not None and not self.cancel.paused:
            elapsed = max(0, time.monotonic() - self._phase_started - self._phase_paused)
            n, total = float(self.progress.cget('value')), float(self.progress.cget('maximum'))
            if str(self.progress.cget('mode')) == 'determinate' and n >= 2 and total > n:
                eta = int(elapsed * (total - n) / n)
                self.progress_detail.set(
                    f'{int(n)}/{int(total)} páginas · {int(elapsed)} s transcurridos · aprox. {eta} s restantes'
                )
            elif not any(word in self.progress_detail.get().lower() for word in ('pausa', 'espera sitfa')):
                self.progress_detail.set(f'{int(elapsed)} s transcurridos · tiempo restante: estimando…')
        while True:
            try:
                event = self.events.get_nowait()
            except queue.Empty:
                break
            try:
                if not self.process_event(*event):
                    return
            except Exception as exc:
                # Un evento defectuoso no detiene la cola: los siguientes (p. ej. idle) liberan la ventana.
                self.status.set(f'No se pudo completar «{event[0]}»: {exc}')
        self.root.after(100, self.poll)


def package_current_lot(
    sheets_folder, diary_manifest, target, consolidated=False, historia=None, extra_folders=(), meses=4, avisos=()
):
    """La misma ruta que Resultados › Paquete: identidad verificada de bitácoras (match_diaries) y destino nuevo."""
    from archivo_local import load_sources
    from export_csmp_capture import export_capture_with_note
    from nurus.capture import read_capture

    target = Path(target)
    if target.exists():
        raise m.PocError('El destino ya existe; elige un nombre nuevo.')
    lots = load_sources([sheets_folder, *extra_folders])
    path, note = export_capture_with_note(
        lots,
        target,
        diaries=[Path(diary_manifest)],
        match_diaries=True,
        consolidated=consolidated,
        historia=historia,
        meses=meses,
        avisos=avisos,
    )
    capture = read_capture(path)
    summary = next(
        (i['message'] for i in capture.rows('incidencias') if i['code'] == 'BITACORAS_REUTILIZADAS'),
        'Sin bitácoras añadidas.',
    )
    return {'path': str(path), 'reuse': summary + note}


def main(argv=None):
    import argparse

    parser = argparse.ArgumentParser(description='Descargador SITFA · prototipo integral')
    parser.add_argument('--csmp-reply', type=Path)
    parser.add_argument('--modo', choices=('ESPERA', 'CUMPLIMIENTO'))
    parser.add_argument('--destino', type=Path)
    parser.add_argument('--bitacoras-reply', type=Path)
    parser.add_argument('--bitacoras-id')
    parser.add_argument('--verificar-paquete', type=Path, help=argparse.SUPPRESS)
    parser.add_argument('--capacidades-integrales', type=Path, help=argparse.SUPPRESS)
    parser.add_argument('--generar-lote-prueba', type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args(argv)
    if args.capacidades_integrales:
        import json
        from contrato_integral import capabilities

        path = args.capacidades_integrales.resolve()
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(capabilities(), ensure_ascii=False), encoding='utf-8')
        return
    if args.generar_lote_prueba:
        import json
        from prueba_bitacoras_paquete import produce

        path = args.generar_lote_prueba.resolve()
        path.mkdir(parents=True, exist_ok=True)
        result = produce(path)
        (path / 'prueba.json').write_text(json.dumps(result, ensure_ascii=False), encoding='utf-8')
        return
    if args.verificar_paquete:
        from verificar_paquete import check

        try:
            check(args.verificar_paquete)
        except Exception as exc:
            import json, traceback

            destination = args.verificar_paquete.resolve()
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_text(
                json.dumps(
                    {
                        'ok': False,
                        'tipo': type(exc).__name__,
                        'ubicacion': [
                            {'modulo': Path(t.filename).name, 'linea': t.lineno}
                            for t in traceback.extract_tb(exc.__traceback__)
                        ],
                    },
                    indent=2,
                ),
                encoding='utf-8',
            )
            raise SystemExit(1)
        return
    root = tk.Tk()
    app = Application(root)
    app.csmp_reply = args.csmp_reply
    app.csmp_mode = args.modo
    if bool(args.bitacoras_reply) != bool(args.bitacoras_id):
        parser.error('La lectura de bitácoras requiere respuesta e identificador.')
    app.bitacoras_reply = args.bitacoras_reply
    app.bitacoras_id = args.bitacoras_id
    if args.modo:
        app.tab.set('Espera' if args.modo == 'ESPERA' else 'Cumplimiento')
    if args.destino:
        app.dest.set(str(args.destino.resolve()))
    app.refresh()
    root.mainloop()


if __name__ == '__main__':
    main()
