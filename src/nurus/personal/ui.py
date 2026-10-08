"""Controles visuales compartidos. CustomTkinter y tablas ttk nativas, sin parches globales."""
from tkinter import ttk
import customtkinter as ctk

# Every pair is (light, dark); CTk applies changes to existing widgets.
BG=('#edf2f6','#15191f')
PANEL=('#ffffff','#20262e')
FIELD=('#f8fafc','#171d24')
TEXT=('#203248','#edf2f7')
MUTED=('#536b7c','#a5b3c2')
BLUE=('#176d8c','#277595')
LINE=('#cbd7e0','#394451')
SIDEBAR=('#f8fafc','#11161c')
HEADING=('#e8f0f5','#2b3541')
THEMES={'Claro':'Light','Oscuro':'Dark','Sistema':'System'}


def color(value):
    return value[1 if ctk.get_appearance_mode()=='Dark' else 0] if isinstance(value,(tuple,list)) else value


def install_theme(root):
    style=ttk.Style(root);style.theme_use('clam')
    panel,field,text,blue=map(color,(PANEL,FIELD,TEXT,BLUE))
    style.configure('.',background=panel,foreground=text,font=('Segoe UI',10))
    style.configure('Treeview',background=field,fieldbackground=field,foreground=text,rowheight=30,borderwidth=0)
    style.configure('Treeview.Heading',background=color(HEADING),foreground=text,padding=6)
    style.map('Treeview',background=[('selected',blue)],foreground=[('selected','white')])
    style.configure('TNotebook',background=panel,borderwidth=0)
    style.configure('TNotebook.Tab',background=color(HEADING),foreground=text,padding=(9,7))
    style.map('TNotebook.Tab',background=[('selected',blue)],foreground=[('selected','white')])
    for key,value in [('background',field),('foreground',text),('selectBackground',blue),('selectForeground','white')]:
        root.option_add('*Listbox.'+key,value)
    root.option_add('*Listbox.highlightThickness',1)
    # Only native widgets need recoloring; CTk updates them through its own pairs.
    def natives(widget):
        import tkinter as tk
        if isinstance(widget,tk.Listbox):widget.configure(background=field,foreground=text,selectbackground=blue,selectforeground='white')
        for child in widget.winfo_children():natives(child)
    natives(root)
    if hasattr(root,'records'):
        dark=ctk.get_appearance_mode()=='Dark'
        for name,background,foreground in [('warning','#653b29' if dark else '#fff1d4','#ffe2cd' if dark else '#765113'),('excluded','#665220' if dark else '#f0ebdc','#fff2cc' if dark else '#635326'),('signed','#243c56' if dark else '#e7f0fa','#ddebf7' if dark else '#284c70')]:
            root.records.tag_configure(name,background=background,foreground=foreground)


def set_theme(root,choice):
    if choice not in THEMES:raise ValueError('Tema desconocido.')
    ctk.set_appearance_mode(THEMES[choice]);install_theme(root)


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
        kwargs.setdefault('height',24);kwargs.setdefault('text_color',TEXT)
        super().__init__(parent,**kwargs)


class Button(ctk.CTkButton):
    def __init__(self,parent,**kwargs):
        kwargs.setdefault('height',30);kwargs.setdefault('width',110);kwargs.setdefault('fg_color',BLUE);kwargs.setdefault('hover_color',('#125974','#1f607e'))
        super().__init__(parent,**kwargs)
        self._canvas.configure(takefocus=1)
        border=self.cget('border_width')
        self.bind('<FocusIn>',lambda e:self.configure(border_width=max(2,border),border_color=('#a36c10','#e7bc55')),add='+')
        self.bind('<FocusOut>',lambda e:self.configure(border_width=border,border_color=LINE),add='+')
        self.bind('<Return>',lambda e:self.invoke(),add='+')
        self.bind('<space>',lambda e:self.invoke(),add='+')


class Entry(ctk.CTkEntry):
    def __init__(self,parent,width=30,**kwargs):
        super().__init__(parent,width=width*7 if width<100 else width,**kwargs)


class Checkbutton(ctk.CTkCheckBox):
    def __init__(self,parent,**kwargs):
        super().__init__(parent,checkbox_width=18,checkbox_height=18,**kwargs)


class Textbox(ctk.CTkTextbox):
    def __init__(self,parent,height=12,**kwargs):
        if 'font' in kwargs:kwargs['font']=font_size(kwargs['font'])
        super().__init__(parent,height=height*17 if height<40 else height,fg_color=FIELD,border_width=1,border_color=LINE,**kwargs)


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
                            text_color=TEXT,command=lambda:self.select(key))
        button.pack(fill='x',padx=10,pady=4);self._buttons[key]=button
        if self._active is None:self.select(key)

    def tabs(self):return tuple(self._pages)

    def select(self,page=None):
        if page is None:return self._active
        key=str(page)
        if self._active:self._pages[self._active].grid_remove()
        self._pages[key].grid(row=0,column=0,sticky='nsew');self._active=key
        for k,button in self._buttons.items():button.configure(fg_color=BLUE if k==key else 'transparent',text_color='white' if k==key else TEXT)
