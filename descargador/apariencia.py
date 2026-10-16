"""Windows desktop palette for ttk and native controls."""

import tkinter as tk
from csmp_shared import parser

parser()  # Desarrollo desde ZIP/monorrepositorio, también sin paquete instalado.
from nurus.ui_theme import install_ttk


def system_dark():
    try:
        import winreg

        with winreg.OpenKey(
            winreg.HKEY_CURRENT_USER, r'Software\Microsoft\Windows\CurrentVersion\Themes\Personalize'
        ) as key:
            return winreg.QueryValueEx(key, 'AppsUseLightTheme')[0] == 0
    except (ImportError, OSError):
        return False


def apply(root, choice):
    dark = choice == 'Oscuro' or choice == 'Sistema' and system_dark()
    palette = install_ttk(root, 'Oscuro' if dark else 'Claro')
    background, field, text = (palette[key] for key in ('background', 'field', 'text'))

    def visit(widget):
        if isinstance(widget, tk.Canvas):
            widget.configure(background=background)
        if isinstance(widget, tk.Listbox):
            widget.configure(
                background=field,
                foreground=text,
                selectbackground=palette['primary'],
                selectforeground=palette['on_primary'],
            )
        for child in widget.winfo_children():
            visit(child)

    root.configure(background=background)
    visit(root)
