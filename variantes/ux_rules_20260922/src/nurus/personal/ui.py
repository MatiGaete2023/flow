"""Controles visuales compartidos. CustomTkinter y tablas ttk nativas, sin parches globales."""
from tkinter import ttk
import customtkinter as ctk

BG='#15191f'
PANEL='#20262e'
FIELD='#171d24'
TEXT='#edf2f7'
MUTED='#a5b3c2'
BLUE='#1f538d'


def install_theme(root):
    style=ttk.Style(root);style.theme_use('clam')
    style.configure('.',background=PANEL,foreground=TEXT,font=('Segoe UI',10))
    style.configure('Treeview',background=FIELD,fieldbackground=FIELD,foreground=TEXT,rowheight=29,borderwidth=0)
    style.configure('Treeview.Heading',background='#2b3541',foreground=TEXT,padding=5)
    style.map('Treeview',background=[('selected',BLUE)],foreground=[('selected','white')])
    style.configure('TNotebook',background=PANEL,borderwidth=0)
    style.configure('TNotebook.Tab',background='#2b3541',foreground=TEXT,padding=(9,7))
    style.map('TNotebook.Tab',background=[('selected',BLUE)])
    root.option_add('*Listbox.background',FIELD);root.option_add('*Listbox.foreground',TEXT)
    root.option_add('*Listbox.selectBackground',BLUE);root.option_add('*Listbox.selectForeground','white')
    root.option_add('*Listbox.highlightThickness',0);root.option_add('*Listbox.borderWidth',0)


def font_size(font):
    if isinstance(font,tuple) and len(font)>1 and 0<int(font[1])<12:return (font[0],13,*font[2:])
    return font


class Frame(ctk.CTkFrame):
    def __init__(self,parent,padding=0,**kwargs):
        kwargs.setdefault('fg_color',PANEL);kwargs.setdefault('corner_radius',8)
        super().__init__(parent,**kwargs)


class Label(ctk.CTkLabel):
    def __init__(self,parent,**kwargs):
        if 'font' in kwargs:kwargs['font']=font_size(kwargs['font'])
        kwargs.setdefault('height',24)
        super().__init__(parent,**kwargs)


class Button(ctk.CTkButton):
    def __init__(self,parent,**kwargs):
        kwargs.setdefault('height',30);kwargs.setdefault('width',110)
        super().__init__(parent,**kwargs)


class Entry(ctk.CTkEntry):
    def __init__(self,parent,width=30,**kwargs):
        super().__init__(parent,width=width*7 if width<100 else width,**kwargs)


class Checkbutton(ctk.CTkCheckBox):
    def __init__(self,parent,**kwargs):
        super().__init__(parent,checkbox_width=18,checkbox_height=18,**kwargs)


class Textbox(ctk.CTkTextbox):
    def __init__(self,parent,height=12,**kwargs):
        if 'font' in kwargs:kwargs['font']=font_size(kwargs['font'])
        super().__init__(parent,height=height*17 if height<40 else height,fg_color=FIELD,border_width=1,border_color='#394451',**kwargs)


class Combobox(ctk.CTkComboBox):
    """IDs/variables existentes con eventos explícitos y selector CTk redondeado."""
    def __init__(self,parent,textvariable=None,width=25,values=None,**kwargs):
        self._selection_callbacks=[]
        super().__init__(parent,variable=textvariable,width=width*7 if width<100 else width,
                         values=list(values or []),command=self._chosen,**kwargs)

    def _chosen(self,value):
        for callback in self._selection_callbacks:callback(None)

    def bind(self,sequence=None,func=None,add=None):
        if sequence=='<<ComboboxSelected>>':
            if not add:self._selection_callbacks=[]
            if func:self._selection_callbacks.append(func)
            return None
        return super().bind(sequence,func,add=add or '+')

    def configure(self,cnf=None,**kwargs):
        if cnf:kwargs={**cnf,**kwargs}
        return super().configure(**kwargs)

    def current(self,index=None):
        values=self.cget('values')
        if index is not None:self.set(values[index]);return
        return values.index(self.get()) if self.get() in values else -1


# CustomTkinter no tiene una tabla de datos ni un panel con separador arrastrable.
# Estas piezas nativas conservan selección múltiple y navegación con teclado.
Treeview=ttk.Treeview
Scrollbar=ttk.Scrollbar
Panedwindow=ttk.Panedwindow
Notebook=ttk.Notebook


class PageStack(ctk.CTkFrame):
    """Navegación lateral; una sola instancia de cada área y del trabajo compartido."""
    def __init__(self,parent,sidebar):
        super().__init__(parent,fg_color='transparent');self.sidebar=sidebar
        self._pages={};self._buttons={};self._active=None
        self.grid_columnconfigure(0,weight=1);self.grid_rowconfigure(0,weight=1)

    def add(self,frame,text):
        key=str(frame);self._pages[key]=frame
        button=ctk.CTkButton(self.sidebar,text='  '+text,anchor='w',fg_color='transparent',height=38,
                            command=lambda:self.select(key))
        button.pack(fill='x',padx=10,pady=4);self._buttons[key]=button
        if self._active is None:self.select(key)

    def tabs(self):return tuple(self._pages)

    def select(self,page=None):
        if page is None:return self._active
        key=str(page)
        if self._active:self._pages[self._active].grid_remove()
        self._pages[key].grid(row=0,column=0,sticky='nsew');self._active=key
        for k,button in self._buttons.items():button.configure(fg_color=BLUE if k==key else 'transparent')
