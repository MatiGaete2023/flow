"""Kit de interfaz común de Flow (Tk/ttk, sin dependencias).

Fuente única: ui/flowkit.py. Cada producto lleva una copia idéntica junto a su código para empaquetarse solo;
tests/test_flowkit.py exige que las copias coincidan y `python tools/sync_flowkit.py` las actualiza.

Esqueleto de cada ventana: TopBar (nombre, estado, menú ⋯) · contenido · Footer (resumen y una acción principal).
"""

import tkinter as tk
from tkinter import ttk

VERSION = '1.0'

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
FONT = ('Segoe UI', 10)
FONT_BOLD = ('Segoe UI', 10, 'bold')
FONT_SMALL = ('Segoe UI', 9)
STATE_MARK = {'ok': '✓', 'run': '●', 'wait': '○', 'warn': '!', 'bad': '✕', 'skip': '–'}


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
    style.configure('BarMuted.TLabel', background=p['bar'], foreground=p['muted'])
    style.configure('Key.TLabel', foreground=p['muted'])
    style.configure('Banner.TLabel', background=p['accent_soft'], foreground=p['fg'], padding=(10, 6))
    style.configure('BannerBad.TLabel', background='#fbeaea', foreground=p['fg'], padding=(10, 6))
    style.configure('TButton', padding=(12, 4), background=p['paper'])
    style.map('TButton', background=[('active', p['chip']), ('disabled', p['bg'])])
    style.configure('Primary.TButton', background=p['accent'], foreground=p['accent_fg'], bordercolor=p['accent'])
    style.map(
        'Primary.TButton',
        background=[('disabled', p['line']), ('active', '#2549ad')],
        foreground=[('disabled', p['muted'])],
    )
    style.configure('Menu.TMenubutton', background=p['bar'], padding=(8, 2), relief='flat')
    style.configure('Chip.Toolbutton', background=p['chip'], foreground=p['fg'], padding=(10, 3), relief='flat')
    style.map(
        'Chip.Toolbutton',
        background=[('disabled', p['bg']), ('selected', p['chip_on']), ('active', p['line'])],
        foreground=[('disabled', p['muted']), ('selected', p['chip_on_fg'])],
    )
    style.configure('Side.Toolbutton', background=p['bg'], anchor='w', padding=(10, 5), relief='flat')
    style.map(
        'Side.Toolbutton',
        background=[('selected', p['chip']), ('active', p['chip'])],
        font=[('selected', FONT_BOLD)],
    )
    style.configure('Group.TLabel', foreground=p['muted'], font=FONT_SMALL)
    style.configure('Pill.TLabel', background=p['paper'], padding=(8, 1), bordercolor=p['line'], relief='solid')
    style.configure('Treeview', background=p['paper'], fieldbackground=p['paper'], rowheight=24, borderwidth=0)
    style.configure('Treeview.Heading', background=p['bar'], foreground=p['muted'], font=FONT_SMALL, relief='flat')
    for name in ('TProgressbar', 'Horizontal.TProgressbar'):
        style.configure(name, background=p['accent'], troughcolor=p['line'], thickness=4, borderwidth=0)
    style.configure('TEntry', fieldbackground=p['paper'], padding=4)
    style.configure('TCombobox', fieldbackground=p['paper'], padding=3)
    return style


class TopBar(ttk.Frame):
    """Nombre del producto, píldora de estado y menú ⋯ con todo lo secundario."""

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

    def add(self, label, command, separator=False):
        if separator:
            self.menu.add_separator()
        self.menu.add_command(label=label, command=command)

    def add_check(self, label, variable, command=None):
        self.menu.add_checkbutton(label=label, variable=variable, command=command)


class Chips(ttk.Frame):
    """Fichas que se marcan con un clic; se reparten en filas según el ancho disponible."""

    def __init__(self, parent, options=(), multi=True, command=None, style='Chip.Toolbutton'):
        super().__init__(parent)
        self.multi = multi
        self.command = command
        self.style_name = style
        self.vars = {}
        self.buttons = {}
        self.single = tk.StringVar()
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
        width = self.winfo_width()
        self._layout(width if width > 50 else 10_000)

    def _layout(self, width):
        # Cada fila es un marco propio: las fichas fluyen sin alinearse en columnas compartidas.
        for frame in getattr(self, '_rows', []):
            frame.destroy()
        self._rows = []
        used = width + 1
        for button in self.buttons.values():
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

    def _changed(self):
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
        for button in self.buttons.values():
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
        ttk.Label(self, textvariable=self.summary, style='BarMuted.TLabel').pack(side='left', fill='x', expand=True)
        self.primary = ttk.Button(self, text=primary_text, style='Primary.TButton', command=primary_command)
        self.primary.pack(side='right')
        self.secondary = []

    def add(self, text, command):
        button = ttk.Button(self, text=text, command=command)
        button.pack(side='right', padx=(0, 8))
        self.secondary.append(button)
        return button

    def set(self, summary, enabled=True, primary_text=None):
        self.summary.set(summary)
        self.primary.state(['!disabled'] if enabled else ['disabled'])
        if primary_text:
            self.primary.configure(text=primary_text)


class ProgressList(ttk.Frame):
    """Avance por ítem: estado (✓ ● ○ ! ✕), nombre y detalle. Doble clic llama a `on_open(clave)`."""

    def __init__(self, parent, on_open=None, height=8):
        super().__init__(parent)
        self.bar = ttk.Progressbar(self, maximum=1, mode='determinate')
        self.bar.pack(fill='x')
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

    def _open(self, _event):
        item = self.tree.focus()
        if item and self.on_open:
            self.on_open(item)


class Banner(ttk.Frame):
    """Un aviso en el lugar, con una acción opcional; se oculta solo al resolverse."""

    def __init__(self, parent):
        super().__init__(parent)
        self.text = tk.StringVar()
        self.label = ttk.Label(self, textvariable=self.text, style='Banner.TLabel', wraplength=640, anchor='w')
        self.label.pack(side='left', fill='x', expand=True)
        self.action = ttk.Button(self, text='')

    def show(self, text, bad=False, action=None, action_text=''):
        self.text.set(text)
        self.label.configure(style='BannerBad.TLabel' if bad else 'Banner.TLabel')
        if action:
            self.action.configure(text=action_text, command=action)
            self.action.pack(side='right', padx=(8, 0))
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
    """Esqueleto común: TopBar arriba, `body` al centro, Banner y Footer abajo."""

    def __init__(self, root, name, primary_text, primary_command, size='900x640', minsize=(720, 520)):
        self.root = root
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

    def _banner(self, visible):
        if visible and not self.banner.winfo_manager():
            self.banner.pack(side='bottom', fill='x', padx=12, pady=(0, 8), before=self.body)
        elif not visible:
            self.banner.pack_forget()


def with_sidebar(window, groups, command=None, width=180):
    """Divide `window.body` en lista lateral y panel; devuelve (SideList, panel)."""
    side = SideList(window.body, groups, command, width=width)
    side.pack(side='left', fill='y')
    side.pack_propagate(False)
    ttk.Separator(window.body, orient='vertical').pack(side='left', fill='y')
    pane = ttk.Frame(window.body, padding=(16, 14))
    pane.pack(side='left', fill='both', expand=True)
    return side, pane
