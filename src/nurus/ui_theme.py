"""Tokens visuales comunes a Tk/ttk y al adaptador CustomTkinter.

No guarda preferencias ni importa ninguna aplicación o dependencia externa.
Cada aplicación resuelve «Sistema» con su mecanismo existente.
"""

from tkinter import ttk


COLORS = {
    'background': ('#f3f6fa', '#15191f'),
    'panel': ('#ffffff', '#20262e'),
    'field': ('#f8fafc', '#171d24'),
    'text': ('#203248', '#edf2f7'),
    'muted': ('#536b7c', '#a5b3c2'),
    'primary': ('#176d8c', '#277595'),
    'primary_hover': ('#125974', '#1f607e'),
    'on_primary': ('#ffffff', '#ffffff'),
    'danger': ('#b42318', '#b42318'),
    'danger_hover': ('#912018', '#912018'),
    'line': ('#cbd7e0', '#394451'),
    'sidebar': ('#f8fafc', '#11161c'),
    'heading': ('#e8f0f5', '#2b3541'),
    'focus': ('#176d8c', '#e7bc55'),
    'ok_background': ('#dcfce7', '#143827'),
    'ok_text': ('#166534', '#bbf7d0'),
    'warning_background': ('#fff1d4', '#653b29'),
    'warning_text': ('#765113', '#ffe2cd'),
    'error_background': ('#fee2e2', '#4c2020'),
    'error_text': ('#991b1b', '#fecaca'),
    'pending_background': ('#e8f0f5', '#2b3541'),
    'pending_text': ('#203248', '#edf2f7'),
}
SPACING = (4, 8, 12, 16, 24)
BODY_FONT = ('Segoe UI', 11)
TITLE_FONT = ('Segoe UI', 18, 'bold')
SECTION_FONT = ('Segoe UI', 12, 'bold')


def tokens(mode):
    """Devuelve una paleta nueva; solo acepta un modo ya resuelto."""
    if mode not in ('Claro', 'Oscuro'):
        raise ValueError('Resuelve el tema Sistema antes de aplicar los tokens.')
    index = int(mode == 'Oscuro')
    return {name: pair[index] for name, pair in COLORS.items()}


def button_options(role='primary', mode=None):
    """Opciones CTk; sin modo, pares claro/oscuro que CTk actualiza solo."""
    palette = COLORS if mode is None else tokens(mode)
    if role == 'primary':
        background, hover, text = 'primary', 'primary_hover', 'on_primary'
    elif role == 'secondary':
        background, hover, text = 'panel', 'heading', 'text'
    elif role == 'danger':
        background, hover, text = 'danger', 'danger_hover', 'on_primary'
    else:
        raise ValueError('Rol visual desconocido: ' + role)
    options = {
        'fg_color': palette[background],
        'hover_color': palette[hover],
        'text_color': palette[text],
        'height': 32,
    }
    if role == 'secondary':
        options.update(border_width=1, border_color=palette['line'])
    return options


def install_ttk(root, mode):
    """Instala estilos del intérprete de root; no cambia widgets ni preferencias."""
    p = tokens(mode)
    style = ttk.Style(root)
    style.theme_use('clam')
    style.configure('.', background=p['panel'], foreground=p['text'], font=BODY_FONT)
    for name in ('TFrame', 'TLabel', 'TLabelframe', 'TLabelframe.Label', 'TCheckbutton', 'TRadiobutton'):
        style.configure(name, background=p['background'], foreground=p['text'])
    style.configure('Card.TFrame', background=p['panel'])
    style.configure('Title.TLabel', font=TITLE_FONT)
    style.configure('Section.TLabel', font=SECTION_FONT)
    style.configure('TButton', padding=(10, 7), font=BODY_FONT, background=p['panel'], foreground=p['text'])
    for name, background, hover, foreground in (
        ('TButton', p['panel'], p['heading'], p['text']),
        ('Secondary.TButton', p['panel'], p['heading'], p['text']),
        ('Primary.TButton', p['primary'], p['primary_hover'], p['on_primary']),
        ('Danger.TButton', p['danger'], p['danger_hover'], p['on_primary']),
    ):
        focus = p['focus'] if name in ('TButton', 'Secondary.TButton') or mode == 'Oscuro' else p['on_primary']
        style.configure(name, background=background, foreground=foreground, borderwidth=1, focuscolor=focus)
        style.map(
            name,
            background=[('disabled', p['heading']), ('pressed', hover), ('active', hover)],
            foreground=[('disabled', p['muted']), ('pressed', foreground), ('active', foreground)],
            bordercolor=[('focus', focus)],
        )
    for name, token in (
        ('OK.TLabel', 'ok'),
        ('Warning.TLabel', 'warning'),
        ('Error.TLabel', 'error'),
        ('Pending.TLabel', 'pending'),
    ):
        style.configure(name, background=p[token + '_background'], foreground=p[token + '_text'], padding=(8, 4))
    for name in ('TEntry', 'TCombobox'):
        style.configure(name, fieldbackground=p['field'], foreground=p['text'])
        style.map(
            name,
            fieldbackground=[('readonly', p['field'])],
            foreground=[('readonly', p['text'])],
        )
    style.configure(
        'Treeview',
        background=p['field'],
        fieldbackground=p['field'],
        foreground=p['text'],
        rowheight=30,
        borderwidth=0,
        font=BODY_FONT,
    )
    style.configure('Treeview.Heading', background=p['heading'], foreground=p['text'], padding=6)
    style.map('Treeview', background=[('selected', p['primary'])], foreground=[('selected', p['on_primary'])])
    style.configure('TNotebook', background=p['panel'], borderwidth=0)
    style.configure('TNotebook.Tab', background=p['heading'], foreground=p['text'], padding=(9, 7))
    style.map('TNotebook.Tab', background=[('selected', p['primary'])], foreground=[('selected', p['on_primary'])])
    return p
