"""Windows desktop palette for ttk and native controls."""
import tkinter as tk
from tkinter import ttk


def system_dark():
    try:
        import winreg
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER,r'Software\Microsoft\Windows\CurrentVersion\Themes\Personalize') as key:
            return winreg.QueryValueEx(key,'AppsUseLightTheme')[0]==0
    except (ImportError,OSError):return False


def apply(root,choice):
    dark=choice=='Oscuro' or choice=='Sistema' and system_dark()
    background,field,text,heading=('#18212d','#243345','#e8eef5','#304157') if dark else ('#f3f6fa','#ffffff','#20324d','#e7eef6')
    style=ttk.Style(root)
    for name in ('TFrame','TLabel','TLabelframe','TLabelframe.Label','TCheckbutton','TRadiobutton'):
        style.configure(name,background=background,foreground=text)
    style.configure('TButton',background=heading,foreground=text)
    style.map('TButton',background=[('active','#176d8c')],foreground=[('active','white'),('disabled','#73869a')])
    for name in ('TEntry','TCombobox'):style.configure(name,fieldbackground=field,foreground=text)
    style.map('TCombobox',fieldbackground=[('readonly',field)],foreground=[('readonly',text)])
    style.configure('Treeview',background=field,fieldbackground=field,foreground=text)
    style.configure('Treeview.Heading',background=heading,foreground=text)
    style.map('Treeview',background=[('selected','#176d8c')],foreground=[('selected','white')])
    def visit(widget):
        if isinstance(widget,tk.Canvas):widget.configure(background=background)
        if isinstance(widget,tk.Listbox):widget.configure(background=field,foreground=text,selectbackground='#176d8c',selectforeground='white')
        for child in widget.winfo_children():visit(child)
    root.configure(background=background);visit(root)
