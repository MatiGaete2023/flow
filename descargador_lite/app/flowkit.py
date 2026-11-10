"""Kit de interfaz común de Flow (Tk/ttk, sin dependencias).

Fuente única: ui/flowkit.py. Cada producto lleva una copia idéntica junto a su código para empaquetarse solo;
tests/test_flowkit.py exige que las copias coincidan y `python tools/sync_flowkit.py` las actualiza.

Esqueleto de cada ventana: TopBar (nombre, estado, menú «Más») · contenido · Banner · Footer (resumen y una acción
principal). Teclado común: Enter = acción principal · Esc = detener (si el producto lo define) · F1 = manual.
"""

import calendar
from datetime import date
import os
from pathlib import Path
import subprocess
import sys
import tkinter as tk
from tkinter import ttk

VERSION = '1.1'

PALETTE = {
    'bg': '#f6f7f9',
    'paper': '#ffffff',
    'bar': '#eef0f3',
    'fg': '#1b2330',
    'muted': '#5d6878',
    'line': '#dde2e9',
    'accent': '#2f5bd3',
    'accent_fg': '#ffffff',
    'accent_soft': '#e8eefc',
    'chip': '#eceff3',
    'chip_on': '#1b2330',
    'chip_on_fg': '#ffffff',
    'ok': '#1f8a4c',
    'warn': '#b4540f',
    'bad': '#c23b3b',
}
# Fondo de los avisos según el tono: información, listo, aviso y error.
BANNER_TONES = {'info': '#e8eefc', 'ok': '#e6f4ec', 'warn': '#fdf1e3', 'bad': '#fbeaea'}
FONT = ('Segoe UI', 10)
FONT_BOLD = ('Segoe UI', 10, 'bold')
FONT_SMALL = ('Segoe UI', 9)
STATE_MARK = {'ok': '✓', 'run': '●', 'wait': '○', 'warn': '!', 'bad': '✕', 'skip': '–'}
MONTH_NAMES = 'Enero Febrero Marzo Abril Mayo Junio Julio Agosto Septiembre Octubre Noviembre Diciembre'.split()
DATE_FORMAT = '%d/%m/%Y'


def _flat_toolbutton(style, name, background_map):
    """Ficha plana: sin relieve hundido al marcarla (en Windows dibujaba un borde punteado) ni anillo de foco."""
    label = [('Toolbutton.label', {'sticky': 'nswe'})]
    style.layout(
        name,
        [
            (
                'Toolbutton.border',
                {'sticky': 'nswe', 'children': [('Toolbutton.padding', {'sticky': 'nswe', 'children': label})]},
            )
        ],
    )
    style.map(
        name,
        relief=[('selected', 'flat'), ('pressed', 'flat'), ('active', 'flat'), ('disabled', 'flat')],
        background=background_map,
        bordercolor=background_map,
        lightcolor=background_map,
        darkcolor=background_map,
    )


def apply_theme(root):
    """Tema claro único para todos los productos; devuelve el ttk.Style configurado."""
    p = PALETTE
    style = ttk.Style(root)
    if 'clam' in style.theme_names():
        style.theme_use('clam')
    root.configure(background=p['bg'])
    root.option_add('*Font', FONT)
    style.configure('.', background=p['bg'], foreground=p['fg'], font=FONT, bordercolor=p['line'])
    style.configure('TFrame', background=p['bg'])
    style.configure('Paper.TFrame', background=p['paper'])
    style.configure('Bar.TFrame', background=p['bar'])
    style.configure('TLabel', background=p['bg'], foreground=p['fg'])
    style.configure('Bar.TLabel', background=p['bar'])
    style.configure('Name.TLabel', background=p['bar'], font=FONT_BOLD)
    style.configure('Muted.TLabel', foreground=p['muted'])
    style.configure('Bad.TLabel', foreground=p['bad'])
    style.configure('Ok.TLabel', foreground=p['ok'])
    style.configure('Hint.TLabel', foreground=p['muted'], font=FONT_SMALL)
    style.configure('Step.TLabel', font=FONT_BOLD)
    style.configure('Empty.TLabel', foreground=p['muted'], font=('Segoe UI', 11))
    style.configure('BarMuted.TLabel', background=p['bar'], foreground=p['muted'])
    style.configure('Key.TLabel', foreground=p['muted'])
    for tone, color in BANNER_TONES.items():
        style.configure(f'Banner{tone.title()}.TLabel', background=color, foreground=p['fg'], padding=(10, 6))
        style.configure(f'Banner{tone.title()}.TButton', background=color, relief='flat', padding=(6, 2))
        style.map(f'Banner{tone.title()}.TButton', background=[('active', p['line'])])
    style.configure('TButton', padding=(12, 4), background=p['paper'])
    style.map('TButton', background=[('active', p['chip']), ('disabled', p['bg'])])
    style.configure(
        'Primary.TButton',
        background=p['accent'],
        foreground=p['accent_fg'],
        bordercolor=p['accent'],
        lightcolor=p['accent'],
        darkcolor=p['accent'],
    )
    edge = [('disabled', p['line']), ('active', '#2549ad')]
    style.map(
        'Primary.TButton',
        background=edge,
        foreground=[('disabled', p['muted'])],
        lightcolor=edge,
        darkcolor=edge,
        bordercolor=edge,
    )
    style.configure('Link.TButton', background=p['bar'], foreground=p['accent'], relief='flat', padding=(6, 2))
    style.map('Link.TButton', background=[('active', p['line'])])
    style.configure('Menu.TMenubutton', background=p['bar'], padding=(8, 2), relief='flat')
    style.configure('Chip.Toolbutton', background=p['chip'], foreground=p['fg'], padding=(10, 3), relief='flat')
    _flat_toolbutton(
        style,
        'Chip.Toolbutton',
        [('disabled', p['bg']), ('selected', p['chip_on']), ('active', p['line']), ('!selected', p['chip'])],
    )
    style.map('Chip.Toolbutton', foreground=[('disabled', p['muted']), ('selected', p['chip_on_fg'])])
    style.configure('Side.Toolbutton', background=p['bg'], anchor='w', padding=(10, 5), relief='flat')
    _flat_toolbutton(style, 'Side.Toolbutton', [('selected', p['chip']), ('active', p['chip']), ('!selected', p['bg'])])
    style.map('Side.Toolbutton', font=[('selected', FONT_BOLD)])
    style.configure('Group.TLabel', foreground=p['muted'], font=FONT_SMALL)
    style.configure('Pill.TLabel', background=p['paper'], padding=(8, 1), bordercolor=p['line'], relief='solid')
    style.configure('Treeview', background=p['paper'], fieldbackground=p['paper'], rowheight=24, borderwidth=0)
    style.configure('Treeview.Heading', background=p['bar'], foreground=p['muted'], font=FONT_SMALL, relief='flat')
    for name in ('TProgressbar', 'Horizontal.TProgressbar'):
        style.configure(name, background=p['accent'], troughcolor=p['line'], thickness=4, borderwidth=0)
    style.configure('TEntry', fieldbackground=p['paper'], padding=4)
    style.configure('Bad.TEntry', fieldbackground='#fbeaea', padding=4)
    style.configure('TCombobox', fieldbackground=p['paper'], padding=3)
    return style


# --- archivos locales y manuales ------------------------------------------------------------------------------------
def open_path(path):
    """Abre un archivo o carpeta local con el programa del sistema. Devuelve False si no existe."""
    path = Path(path)
    if not path.exists():
        return False
    if sys.platform == 'win32':
        os.startfile(path)  # noqa: S606 - ruta local elegida o creada por la herramienta
    else:
        subprocess.Popen(['xdg-open', str(path)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return True


def doc_folders(anchor=None):
    """Carpetas donde puede estar un manual: junto al ejecutable (instalado) o en docs/ del repositorio."""
    folders = []
    if getattr(sys, 'frozen', False):
        exe = Path(sys.executable).resolve().parent
        folders += [exe / 'manuales', exe / '_internal' / 'manuales', exe.parent / 'manuales', exe]
    here = Path(anchor or __file__).resolve()
    for parent in here.parents:
        folders += [parent / 'manuales', parent / 'docs' / 'manuales', parent / 'docs']
        if (parent / '.git').exists():
            break
    return folders


def find_doc(name, anchor=None):
    return next((folder / name for folder in doc_folders(anchor) if (folder / name).is_file()), None)


def short_path(path, keep=2):
    """«…/Documentos/SITFA Lite»: las últimas carpetas de una ruta larga."""
    parts = Path(path).parts
    return str(path) if len(parts) <= keep + 1 else '…' + os.sep + os.sep.join(parts[-keep:])


# --- fechas ---------------------------------------------------------------------------------------------------------
def parse_date(text):
    """DD/MM/AAAA (también acepta AAAA-MM-DD); devuelve date o lanza ValueError con un motivo legible."""
    text = (text or '').strip()
    try:
        if '-' in text and len(text.split('-')[0]) == 4:
            return date.fromisoformat(text)
        day, month, year = (int(x) for x in text.split('/'))
        return date(year, month, day)
    except (ValueError, TypeError):
        raise ValueError('Escribe la fecha como DD/MM/AAAA, por ejemplo 16/10/2026.') from None


def pick_date(root, variable, on_pick=None):
    """Calendario mensual; al elegir un día escribe DD/MM/AAAA en la variable."""
    try:
        chosen = parse_date(variable.get())
    except ValueError:
        chosen = date.today()
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
        variable.set(day.strftime(DATE_FORMAT))
        popup.destroy()
        if on_pick:
            on_pick()

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
    return popup


class DateField(ttk.Frame):
    """Campo de fecha DD/MM/AAAA con botón de calendario. `error(True)` marca el campo en rojo."""

    def __init__(self, parent, variable, command=None, label=None, width=11):
        super().__init__(parent)
        self.variable = variable
        self.command = command
        if label:
            ttk.Label(self, text=label, style='Muted.TLabel').pack(side='left', padx=(0, 4))
        self.entry = ttk.Entry(self, textvariable=variable, width=width)
        self.entry.pack(side='left')
        self.entry.bind('<KeyRelease>', lambda _e: self._changed())
        self.button = ttk.Button(self, text='Calendario', command=self.pick)
        self.button.pack(side='left', padx=(2, 0))

    def pick(self):
        pick_date(self.winfo_toplevel(), self.variable, self._changed)

    def _changed(self):
        if self.command:
            self.command()

    def error(self, bad):
        self.entry.configure(style='Bad.TEntry' if bad else 'TEntry')

    def enable(self, enabled=True):
        for widget in (self.entry, self.button):
            widget.state(['!disabled'] if enabled else ['disabled'])


# --- piezas de la ventana -------------------------------------------------------------------------------------------
class Tooltip:
    """Texto flotante al pasar el mouse; `text_for(event)` devuelve el texto o '' para no mostrar nada."""

    def __init__(self, widget, text_for):
        self.widget, self.text_for, self.tip, self.shown = widget, text_for, None, ''
        widget.bind('<Motion>', self._move, add='+')
        widget.bind('<Leave>', lambda _e: self.hide(), add='+')

    def _move(self, event):
        text = self.text_for(event)
        if text == self.shown:
            if self.tip:
                self.tip.geometry(f'+{event.x_root + 14}+{event.y_root + 12}')
            return
        self.hide()
        if text:
            self.shown = text
            self.tip = tk.Toplevel(self.widget)
            self.tip.wm_overrideredirect(True)
            tk.Label(
                self.tip, text=text, background=PALETTE['fg'], foreground='white', font=FONT_SMALL, padx=8, pady=3
            ).pack()
            self.tip.geometry(f'+{event.x_root + 14}+{event.y_root + 12}')

    def hide(self):
        if self.tip:
            self.tip.destroy()
        self.tip, self.shown = None, ''


class TopBar(ttk.Frame):
    """Nombre del producto, píldora de estado y menú «Más» con todo lo secundario."""

    def __init__(self, parent, name):
        super().__init__(parent, style='Bar.TFrame', padding=(12, 6))
        ttk.Label(self, text=name, style='Name.TLabel').pack(side='left')
        self.extra = ttk.Frame(self, style='Bar.TFrame')
        self.extra.pack(side='left', padx=(16, 0), fill='x', expand=True)
        self.menu_button = ttk.Menubutton(self, text='Más', style='Menu.TMenubutton')
        self.menu = tk.Menu(self.menu_button, tearoff=False)
        self.menu_button['menu'] = self.menu
        self.menu_button.pack(side='right')
        self.dot = tk.Label(self, text='●', background=PALETTE['bar'], foreground=PALETTE['muted'], font=FONT_SMALL)
        self.state_text = tk.StringVar()
        self.pill = ttk.Label(self, textvariable=self.state_text, style='BarMuted.TLabel')
        self.locked = []

    def set_state(self, text, kind='ok'):
        """Estado breve a la derecha (p. ej. «Chrome conectado»); texto vacío lo oculta."""
        self.state_text.set(text)
        if not text:
            self.pill.pack_forget()
            self.dot.pack_forget()
            return
        self.dot.configure(foreground=PALETTE.get(kind, PALETTE['muted']))
        if not self.pill.winfo_manager():
            self.pill.pack(side='right', padx=(0, 10))
            self.dot.pack(side='right')

    def add(self, label, command, separator=False, busy_lock=False):
        """busy_lock: la acción se ve deshabilitada mientras el producto trabaja (ver `lock`)."""
        if separator:
            self.menu.add_separator()
        self.menu.add_command(label=label, command=command)
        if busy_lock:
            self.locked.append(self.menu.index('end'))

    def add_check(self, label, variable, command=None, busy_lock=False):
        self.menu.add_checkbutton(label=label, variable=variable, command=command)
        if busy_lock:
            self.locked.append(self.menu.index('end'))

    def add_note(self, label, separator=True):
        """Línea informativa (p. ej. la versión): se ve deshabilitada para no parecer una acción."""
        if separator:
            self.menu.add_separator()
        self.menu.add_command(label=label, state='disabled')

    def lock(self, busy):
        for index in self.locked:
            self.menu.entryconfigure(index, state='disabled' if busy else 'normal')


class Chips(ttk.Frame):
    """Fichas que se marcan con un clic; se reparten en filas según el ancho disponible.

    all_label: agrega al inicio una ficha (p. ej. «Todos») que marca o desmarca todas; solo en selección múltiple y
    cuando hay más de una opción.
    """

    def __init__(self, parent, options=(), multi=True, command=None, style='Chip.Toolbutton', all_label=None):
        super().__init__(parent)
        self.multi = multi
        self.command = command
        self.style_name = style
        self.vars = {}
        self.buttons = {}
        self.single = tk.StringVar()
        self.all_var = tk.BooleanVar(value=False)
        self.all_button = None
        if all_label and multi:
            self.all_button = ttk.Checkbutton(
                self, text=all_label, variable=self.all_var, style=style, command=self._toggle_from_all
            )
        self._width = 0
        self.bind('<Configure>', self._reflow)
        self.set_options(options)

    def set_options(self, options, selected=None):
        """options: [(valor, texto)]; selected: valores marcados (por defecto, se conservan los que sigan)."""
        keep = set(self.get()) if selected is None else set(selected)
        for button in self.buttons.values():
            button.destroy()
        self.vars, self.buttons = {}, {}
        for value, text in options:
            if self.multi:
                var = tk.BooleanVar(value=value in keep)
                button = ttk.Checkbutton(self, text=text, variable=var, style=self.style_name, command=self._changed)
                self.vars[value] = var
            else:
                button = ttk.Radiobutton(
                    self, text=text, value=value, variable=self.single, style=self.style_name, command=self._changed
                )
            self.buttons[value] = button
        if not self.multi:
            # La primera opción pedida que exista; si ninguna existe, la primera disponible.
            pedidas = [v for v in (selected or [self.single.get()]) if v in self.buttons]
            self.single.set(pedidas[0] if pedidas else next(iter(self.buttons), ''))
        self._sync_all()
        width = self.winfo_width()
        self._layout(width if width > 50 else 10_000)

    def _layout(self, width):
        # Cada fila es un marco propio: las fichas fluyen sin alinearse en columnas compartidas.
        for frame in getattr(self, '_rows', []):
            frame.destroy()
        self._rows = []
        used = width + 1
        buttons = list(self.buttons.values())
        if self.all_button is not None:
            self.all_button.pack_forget()
            if len(buttons) > 1:
                buttons.insert(0, self.all_button)
        for button in buttons:
            need = button.winfo_reqwidth() + 6
            if used + need > width and (used <= width or not self._rows):
                frame = ttk.Frame(self)
                frame.pack(side='top', anchor='w', pady=(0, 6))
                self._rows.append(frame)
                used = 0
            button.pack(in_=self._rows[-1], side='left', padx=(0, 6))
            button.lift(self._rows[-1])
            used += need

    def _reflow(self, event):
        if abs(event.width - self._width) > 4:
            self._width = event.width
            self._layout(event.width)

    def _sync_all(self):
        self.all_var.set(bool(self.vars) and all(var.get() for var in self.vars.values()))

    def _toggle_from_all(self):
        for var in self.vars.values():
            var.set(self.all_var.get())
        self._changed()

    def _changed(self):
        self._sync_all()
        if self.command:
            self.command()

    def get(self):
        if self.multi:
            return tuple(v for v, var in self.vars.items() if var.get())
        value = self.single.get()
        return (value,) if value else ()

    def set(self, values):
        values = set(values)
        if self.multi:
            for value, var in self.vars.items():
                var.set(value in values)
            self._sync_all()
        else:
            self.single.set(next(iter(values), ''))

    def toggle_all(self):
        """Clic en el nombre de la fila: si hay alguna sin marcar, marca todas; si no, las desmarca."""
        if not self.multi:
            return
        target = not all(var.get() for var in self.vars.values())
        for var in self.vars.values():
            var.set(target)
        self._changed()

    def enable(self, enabled=True):
        for button in [*self.buttons.values(), *([self.all_button] if self.all_button else [])]:
            button.state(['!disabled'] if enabled else ['disabled'])


class Form(ttk.Frame):
    """Filas «nombre · contenido». Un clic en el nombre de una fila de fichas marca o desmarca todas."""

    def __init__(self, parent, key_width=12, **kwargs):
        super().__init__(parent, **kwargs)
        self.key_width = key_width
        self.rows = {}
        self.columnconfigure(1, weight=1)

    def row(self, key, widget_factory, name=None):
        index = len(self.rows)
        label = ttk.Label(self, text=key, style='Key.TLabel', width=self.key_width, cursor='hand2')
        label.grid(row=index, column=0, sticky='nw', pady=(3, 6))
        widget = widget_factory(self)
        widget.grid(row=index, column=1, sticky='w' if isinstance(widget, (ttk.Entry, ttk.Combobox)) else 'ew')
        if isinstance(widget, Chips):
            label.bind('<Button-1>', lambda _e: widget.toggle_all())
        self.rows[name or key] = (label, widget)
        return widget

    def show(self, name, visible=True):
        for item in self.rows[name]:
            item.grid() if visible else item.grid_remove()


class SideList(ttk.Frame):
    """Lista fija de «qué» (consultas, secciones) a la izquierda; una sola elegida."""

    def __init__(self, parent, groups, command=None, width=170):
        super().__init__(parent, padding=(8, 10), width=width)
        self.var = tk.StringVar()
        self.command = command
        self.buttons = {}
        for group, items in groups:
            if group:
                ttk.Label(self, text=group.upper(), style='Group.TLabel').pack(anchor='w', padx=10, pady=(10, 2))
            for key, text in items:
                button = ttk.Radiobutton(
                    self, text=text, value=key, variable=self.var, style='Side.Toolbutton', command=self._changed
                )
                button.pack(fill='x')
                self.buttons[key] = button
        if self.buttons:
            self.var.set(next(iter(self.buttons)))

    def _changed(self):
        if self.command:
            self.command(self.var.get())

    def get(self):
        return self.var.get()

    def select(self, key, notify=False):
        self.var.set(key)
        if notify:
            self._changed()

    def set_text(self, key, text):
        self.buttons[key].configure(text=text)

    def enable(self, enabled=True):
        for button in self.buttons.values():
            button.state(['!disabled'] if enabled else ['disabled'])


class Footer(ttk.Frame):
    """Resumen de lo que va a pasar y la acción principal; botones secundarios a su izquierda."""

    def __init__(self, parent, primary_text, primary_command):
        super().__init__(parent, style='Bar.TFrame', padding=(12, 8))
        self.summary = tk.StringVar()
        self.summary_label = ttk.Label(self, textvariable=self.summary, style='BarMuted.TLabel')
        self.summary_label.pack(side='left', fill='x', expand=True)
        self.primary = ttk.Button(self, text=primary_text, style='Primary.TButton', command=primary_command)
        self.primary.pack(side='right')
        self.secondary = []

    def add(self, text, command):
        button = ttk.Button(self, text=text, command=command)
        button.pack(side='right', padx=(0, 8))
        self.secondary.append(button)
        return button

    def add_link(self, text, command):
        """Enlace discreto a la izquierda del resumen (p. ej. la carpeta de destino, con clic para cambiarla)."""
        button = ttk.Button(self, text=text, command=command, style='Link.TButton')
        button.pack(side='left', padx=(0, 10), before=self.summary_label)
        return button

    def set(self, summary, enabled=True, primary_text=None):
        self.summary.set(summary)
        self.primary.state(['!disabled'] if enabled else ['disabled'])
        if primary_text:
            self.primary.configure(text=primary_text)


class ProgressList(ttk.Frame):
    """Avance por ítem: estado (✓ ● ○ ! ✕), nombre y detalle. Doble clic (o Enter) llama a `on_open(clave)`."""

    def __init__(self, parent, on_open=None, height=8, hint=None):
        super().__init__(parent)
        self.bar = ttk.Progressbar(self, maximum=1, mode='determinate')
        self.bar.pack(side='top', fill='x')
        self.hint = ttk.Label(self, text=hint or '', style='Hint.TLabel')
        if hint:
            self.hint.pack(side='bottom', anchor='w', pady=(4, 0))
        self.tree = ttk.Treeview(self, columns=('estado', 'nombre', 'detalle'), show='', height=height)
        self.tree.column('estado', width=28, stretch=False, anchor='center')
        self.tree.column('nombre', width=360)
        self.tree.column('detalle', width=180, anchor='e')
        for kind in ('ok', 'run', 'wait', 'warn', 'bad', 'skip'):
            self.tree.tag_configure(kind, foreground=PALETTE.get(kind, PALETTE['muted']))
        self.tree.tag_configure('run', foreground=PALETTE['accent'])
        scroll = ttk.Scrollbar(self, orient='vertical', command=self.tree.yview)
        self.tree.configure(yscrollcommand=scroll.set)
        self.tree.pack(side='left', fill='both', expand=True)
        scroll.pack(side='right', fill='y')
        self.on_open = on_open
        self.tree.bind('<Double-1>', self._open)
        self.tree.bind('<Return>', self._open)
        self.done = 0

    def clear(self):
        self.tree.delete(*self.tree.get_children())
        self.bar.configure(maximum=1, value=0)
        self.done = 0

    def plan(self, items):
        """items: [(clave, nombre)] en estado pendiente."""
        items = list(items)
        self.clear()
        for key, name in items:
            self.tree.insert('', 'end', iid=str(key), values=(STATE_MARK['wait'], name, ''), tags=('wait',))
        self.bar.configure(maximum=max(1, len(items)))

    def set(self, key, kind, detail='', name=None):
        key = str(key)
        if not self.tree.exists(key):
            self.tree.insert('', 'end', iid=key, values=(STATE_MARK.get(kind, ''), name or key, detail), tags=(kind,))
        else:
            values = list(self.tree.item(key, 'values'))
            values[0], values[2] = STATE_MARK.get(kind, ''), detail
            if name:
                values[1] = name
            self.tree.item(key, values=values, tags=(kind,))
        if kind in ('ok', 'warn', 'bad', 'skip'):
            self.done = sum(
                1 for i in self.tree.get_children() if self.tree.item(i, 'tags')[0] in ('ok', 'warn', 'bad', 'skip')
            )
            self.bar.configure(value=self.done)
        self.tree.see(key)

    def kinds(self):
        """{clave: estado} de cada ítem (p. ej. para reintentar los que no quedaron listos)."""
        return {i: self.tree.item(i, 'tags')[0] for i in self.tree.get_children()}

    def _open(self, _event):
        item = self.tree.focus()
        if item and self.on_open:
            self.on_open(item)


class Banner(ttk.Frame):
    """Un aviso en el lugar con tono (info, ok, warn, bad), una acción opcional y «×» para cerrarlo."""

    def __init__(self, parent):
        super().__init__(parent)
        self.text = tk.StringVar()
        self.kind = 'info'
        self.label = ttk.Label(self, textvariable=self.text, style='BannerInfo.TLabel', wraplength=640, anchor='w')
        self.label.pack(side='left', fill='x', expand=True)
        self.close = ttk.Button(self, text='×', width=2, command=self.hide, style='BannerInfo.TButton')
        self.close.pack(side='right')
        self.action = ttk.Button(self, text='')

    def show(self, text, bad=False, action=None, action_text='', kind=None):
        self.kind = kind or ('bad' if bad else 'info')
        self.text.set(text)
        self.label.configure(style=f'Banner{self.kind.title()}.TLabel')
        self.close.configure(style=f'Banner{self.kind.title()}.TButton')
        if action:
            self.action.configure(text=action_text, command=action)
            self.action.pack(side='right', padx=(8, 4), before=self.close)
        else:
            self.action.pack_forget()
        self.event_generate('<<BannerShow>>')

    def hide(self):
        self.text.set('')
        self.event_generate('<<BannerHide>>')

    @property
    def visible(self):
        return bool(self.text.get())


class Window:
    """Esqueleto común: TopBar arriba, `body` al centro, Banner y Footer abajo.

    Teclado: Enter pulsa la acción principal (salvo escribiendo en un campo); Esc llama a `on_escape` si el producto
    lo define; F1 abre el primer manual registrado con `add_docs`.
    """

    def __init__(self, root, name, primary_text, primary_command, size='900x640', minsize=(720, 520)):
        self.root = root
        self.title = name
        self.on_escape = None
        self.docs = []
        apply_theme(root)
        root.title(name)
        root.geometry(size)
        root.minsize(*minsize)
        self.top = TopBar(root, name)
        self.top.pack(side='top', fill='x')
        ttk.Separator(root).pack(side='top', fill='x')
        self.footer = Footer(root, primary_text, primary_command)
        self.footer.pack(side='bottom', fill='x')
        ttk.Separator(root).pack(side='bottom', fill='x')
        self.banner = Banner(root)
        self.banner.bind('<<BannerShow>>', lambda _e: self._banner(True))
        self.banner.bind('<<BannerHide>>', lambda _e: self._banner(False))
        self.body = ttk.Frame(root)
        self.body.pack(side='top', fill='both', expand=True)
        root.bind('<Return>', self._enter, add='+')
        root.bind('<KP_Enter>', self._enter, add='+')
        root.bind('<Escape>', lambda _e: self.on_escape() if self.on_escape else None, add='+')
        root.bind('<F1>', lambda _e: self.open_doc(self.docs[0][1]) if self.docs else None, add='+')

    def _banner(self, visible):
        if visible and not self.banner.winfo_manager():
            self.banner.pack(side='bottom', fill='x', padx=12, pady=(0, 8), before=self.body)
        elif not visible:
            self.banner.pack_forget()

    def _enter(self, event):
        # En un campo, una lista o un botón con foco, Enter es de ese control.
        if event.widget.winfo_class() in ('TEntry', 'Entry', 'TCombobox', 'Text', 'Treeview', 'TButton'):
            return
        if event.widget.winfo_toplevel() is not self.root:
            return
        if self.footer.primary.instate(['!disabled']):
            self.footer.primary.invoke()

    def add_docs(self, docs, anchor=None):
        """docs: [(texto del menú, archivo)]. Se agregan a «Más»; F1 abre el primero."""
        self.top.menu.add_separator()
        for label, name in docs:
            self.docs.append((label, name))
            self.top.menu.add_command(label=label, command=lambda name=name: self.open_doc(name, anchor))
        self._doc_anchor = anchor

    def open_doc(self, name, anchor=None):
        path = find_doc(name, anchor or getattr(self, '_doc_anchor', None))
        if path is None or not open_path(path):
            self.banner.show(
                f'No se encontró {name} junto al programa: búscalo en la carpeta de manuales de la entrega.',
                kind='warn',
            )

    def set_progress(self, text=''):
        """Avance visible en la barra de tareas: «3 de 8 · SITFA Lite»."""
        self.root.title(f'{text} · {self.title}' if text else self.title)


def with_sidebar(window, groups, command=None, width=180):
    """Divide `window.body` en lista lateral y panel; devuelve (SideList, panel)."""
    side = SideList(window.body, groups, command, width=width)
    side.pack(side='left', fill='y')
    side.pack_propagate(False)
    ttk.Separator(window.body, orient='vertical').pack(side='left', fill='y')
    pane = ttk.Frame(window.body, padding=(16, 14))
    pane.pack(side='left', fill='both', expand=True)
    return side, pane
