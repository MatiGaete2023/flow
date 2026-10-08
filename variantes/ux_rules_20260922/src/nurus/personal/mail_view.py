"""Vista de Correos: filtros opcionales, tarjetas de datos reales y editor amplio."""
from datetime import date
from pathlib import Path
import tkinter as tk
import customtkinter as ctk

from . import ui
from .widgets import NamedChoice
from .modalities import MODALITIES


class DraftCards(ctk.CTkScrollableFrame):
    def __init__(self,parent):
        super().__init__(parent,width=235,fg_color=ui.FIELD)
        self._selected=None;self._callback=None;self.cards=[];self.drafts=[]

    def bind(self,sequence=None,func=None,add=None):
        if sequence=='<<ListboxSelect>>':self._callback=func;return
        return super().bind(sequence,func,add=add or '+')

    def delete(self,*args):
        for child in self.winfo_children():child.destroy()
        self.cards=[];self.drafts=[];self._selected=None

    def curselection(self):return () if self._selected is None else (self._selected,)

    def selection_set(self,index):
        self._selected=int(index)
        for i,card in enumerate(self.cards):card.configure(border_color='#5494d5' if i==self._selected else '#36414e')

    def _choose(self,index):
        self.selection_set(index)
        if self._callback:self._callback(None)

    def set_drafts(self,drafts,receipts=None):
        self.delete();self.drafts=list(drafts)
        if not drafts:
            ctk.CTkLabel(self,text='Prepara correos para ver\\nlos programas y sus causas.'.replace('\\n','\n'),text_color=ui.MUTED).pack(padx=10,pady=24)
        for i,draft in enumerate(drafts):
            card=ctk.CTkFrame(self,fg_color=ui.PANEL,corner_radius=10,border_width=1,border_color='#36414e')
            card.pack(fill='x',padx=4,pady=6);self.cards.append(card)
            title=draft.program or draft.court or draft.subject
            ctk.CTkLabel(card,text=title,wraplength=205,anchor='w',justify='left',font=('Segoe UI',13,'bold')).pack(fill='x',padx=12,pady=(10,2))
            details=draft.court if draft.program else draft.subject
            if draft.due:details+='\nVencimiento más cercano: '+draft.due
            if draft.record_ids:details+='\n'+str(len(draft.record_ids))+' registro(s)'
            ctk.CTkLabel(card,text=details,wraplength=205,anchor='w',justify='left',text_color=ui.MUTED,font=('Segoe UI',12)).pack(fill='x',padx=12,pady=3)
            badges=ctk.CTkFrame(card,fg_color='transparent');badges.pack(fill='x',padx=12,pady=(2,10))
            expired=bool(draft.due and draft.due<date.today().isoformat())
            saved=(receipts or {}).get(draft.key,{}).get('state')=='created'
            text='Guardado en Outlook' if saved else 'Borrador editable'
            ctk.CTkLabel(badges,text=text,fg_color='#245f54' if saved else ui.BLUE,corner_radius=5,height=21,font=('Segoe UI',11)).pack(anchor='w')
            if draft.due:
                ctk.CTkLabel(badges,text='Fecha vencida' if expired else 'Por vencer',fg_color='#813c32' if expired else '#665220',corner_radius=5,height=21,font=('Segoe UI',11)).pack(anchor='w',pady=(4,0))
            self._bind_card(card,i)

    def _bind_card(self,widget,index):
        widget.bind('<Button-1>',lambda event:self._choose(index),add='+')
        for child in widget.winfo_children():
            # CTk contiene canvas internos; se vinculan también para que toda la tarjeta responda.
            if isinstance(child,(ctk.CTkFrame,ctk.CTkLabel)):self._bind_card(child,index)


class AttachmentChips(ctk.CTkScrollableFrame):
    def __init__(self,parent,variable):
        # Una sola franja horizontal evita que muchos adjuntos reduzcan el editor.
        super().__init__(parent,height=42,fg_color=ui.FIELD,orientation='horizontal')
        self.variable=variable;self._trace=variable.trace_add('write',self._draw);self._draw()

    def _remove(self,index):
        files=[p for p in self.variable.get().split('\n') if p]
        del files[index];self.variable.set('\n'.join(files))

    def _draw(self,*args):
        for child in self.winfo_children():child.destroy()
        files=[p for p in self.variable.get().split('\n') if p]
        if not files:ctk.CTkLabel(self,text='Sin adjuntos',text_color=ui.MUTED,height=24).pack(side='left',padx=6)
        for i,path in enumerate(files):
            chip=ctk.CTkFrame(self,fg_color=ui.BLUE,corner_radius=7);chip.pack(side='left',pady=2,padx=3)
            ctk.CTkButton(chip,text='×',width=27,height=24,fg_color='transparent',command=lambda index=i:self._remove(index)).pack(side='right',padx=3)
            ctk.CTkLabel(chip,text=Path(path).name,anchor='w',height=24).pack(side='left',fill='x',expand=True,padx=8)

    def destroy(self):
        self.variable.trace_remove('write',self._trace);super().destroy()


def build_mail_page(app):
    page=app.pages['Correos'];page.grid_columnconfigure(0,weight=1);page.grid_rowconfigure(3,weight=1)
    cfg=app.cfg.data['correos']
    top=ctk.CTkFrame(page,fg_color='transparent');top.grid(row=0,column=0,sticky='ew',padx=8,pady=(8,4));top.grid_columnconfigure(1,weight=1)
    app.mail_target=tk.StringVar(value='programas')
    app.target_box=NamedChoice(top,keyvariable=app.mail_target,names=lambda:{'programas':'Solo programas','tribunales':'Solo tribunales','todos':'Ambos'},state='readonly',width=21)
    app.target_box.grid(row=0,column=0,padx=(0,8));app.target_box.bind('<<ComboboxSelected>>',app._mail_target_changed)
    app.mail_kind=tk.StringVar(value='programa_por_vencer')
    app.kind_box=NamedChoice(top,keyvariable=app.mail_kind,names=lambda:{k:v['nombre'] for k,v in app.cfg.data['correos']['plantillas'].items()},state='readonly',width=32)
    app.kind_box.grid(row=0,column=1,sticky='ew');app.kind_box.bind('<<ComboboxSelected>>',app._mail_kind_changed)
    ui.Button(top,text='Preparar tipo',command=lambda:app._guard(app._prepare_mail)).grid(row=0,column=2,padx=7)
    options=ctk.CTkFrame(page,fg_color='#28313c');options.grid(row=2,column=0,sticky='ew',padx=8,pady=4);options.grid_remove()
    def toggle():
        if options.winfo_manager():options.grid_remove()
        else:options.grid()
    ui.Button(top,text='Filtros / opciones',command=toggle).grid(row=0,column=3)
    toolbar=ctk.CTkFrame(page,fg_color='transparent');toolbar.grid(row=1,column=0,sticky='ew',padx=8,pady=(2,6))
    ui.Button(toolbar,text='Preparar todos',command=lambda:app._guard(app._prepare_all_mail)).pack(side='left')
    ui.Button(toolbar,text='Cargar planilla',command=lambda:app._guard(app._external)).pack(side='left',padx=7)
    app.mail_note=tk.StringVar();ui.Label(toolbar,textvariable=app.mail_note,wraplength=380,text_color=ui.MUTED).pack(side='left',padx=7)

    # Filtrar causas por tribunal es independiente de elegir a quién se dirige el correo.
    court_panel=ctk.CTkFrame(options,fg_color='transparent');court_panel.pack(side='left',fill='both',expand=True,padx=8,pady=6)
    ui.Label(court_panel,text='Tribunal de las causas (filtro opcional)').pack(anchor='w')
    app.mail_court_keys=list(cfg['tribunales'])
    app.mail_courts=tk.Listbox(court_panel,selectmode='extended',exportselection=False,height=3,font=('Segoe UI',10))
    app.mail_courts.pack(fill='x')
    for key in app.mail_court_keys:app.mail_courts.insert('end',cfg['tribunales'][key].get('nombre',key))
    ui.Label(court_panel,text='Sin selección: todos los tribunales',text_color=ui.MUTED).pack(anchor='w')
    app.period=tk.StringVar(value=cfg.get('periodo_default') or date.today().strftime('%m/%Y'))
    period_bar=ctk.CTkFrame(court_panel,fg_color='transparent');period_bar.pack(fill='x',pady=3)
    ui.Label(period_bar,text='Período').pack(side='left');ctk.CTkEntry(period_bar,textvariable=app.period,width=90).pack(side='left',padx=8)
    modes=ctk.CTkFrame(options,fg_color='transparent');modes.pack(side='left',fill='both',expand=True,padx=8,pady=6)
    app.modality_vars={};defaults=set(cfg.get('modalidades_default') or MODALITIES.values())
    for key,title in [('RES','Residencial'),('AMB','Ambulatorio'),('FAE','Familia de acogida (FAE / FAS)'),('DCE','Diagnóstico clínico (DCE)')]:
        var=tk.BooleanVar(value=MODALITIES[key] in defaults);app.modality_vars[key]=var
        ui.Checkbutton(modes,text=title,variable=var,command=app._mail_modality_changed).pack(anchor='w',pady=2)
    app.mail_scope=tk.StringVar();ui.Label(modes,textvariable=app.mail_scope,wraplength=300,text_color=ui.MUTED).pack(anchor='w')
    app.manual_mail=tk.BooleanVar()
    ui.Checkbutton(modes,text='Solo filas seleccionadas en Trabajo',variable=app.manual_mail).pack(anchor='w',pady=4)
    ui.Button(modes,text='Adjuntar a todos…',command=lambda:app._guard(app._attachment_all)).pack(anchor='w')

    content=ctk.CTkFrame(page,fg_color='transparent');content.grid(row=3,column=0,sticky='nsew',padx=8,pady=(0,8))
    content.grid_rowconfigure(0,weight=1);content.grid_columnconfigure(1,weight=1)
    app.mail_list=DraftCards(content);app.mail_list.grid(row=0,column=0,sticky='ns',padx=(0,10));app.mail_list.bind('<<ListboxSelect>>',app._select_mail)
    app.mail_list.set_drafts([])
    compose=ctk.CTkFrame(content,fg_color=ui.PANEL,corner_radius=12,border_width=1,border_color='#394451')
    compose.grid(row=0,column=1,sticky='nsew');compose.grid_columnconfigure(1,weight=1);compose.grid_rowconfigure(5,weight=1,minsize=120)
    ui.Label(compose,text='Correo editable',font=('Segoe UI',16,'bold')).grid(row=0,column=0,columnspan=2,sticky='w',padx=12,pady=(8,4))
    app.mail_detail=tk.StringVar();ui.Label(compose,textvariable=app.mail_detail,justify='left',anchor='w',wraplength=650,text_color=ui.MUTED).grid(row=1,column=0,columnspan=2,sticky='ew',padx=12,pady=(0,5))
    ui.Button(compose,text='Vista previa',width=100,command=lambda:app._guard(app._preview_mail)).grid(row=0,column=1,sticky='e',padx=12,pady=5)
    app.to=tk.StringVar();app.cc=tk.StringVar();app.subject=tk.StringVar();app.attach=tk.StringVar()
    for row,label,var in [(2,'Para',app.to),(3,'CC',app.cc),(4,'Asunto',app.subject)]:
        ui.Label(compose,text=label).grid(row=row,column=0,sticky='w',padx=(12,5))
        ctk.CTkEntry(compose,textvariable=var,height=30).grid(row=row,column=1,sticky='ew',padx=(0,12),pady=3)
    app.body=ui.Textbox(compose,height=200,wrap='word',font=('Segoe UI',14),undo=True)
    app.body.grid(row=5,column=0,columnspan=2,sticky='nsew',padx=12,pady=7)
    app.mail_compare=tk.StringVar();ui.Label(compose,textvariable=app.mail_compare,justify='left',anchor='w',wraplength=650,text_color=ui.MUTED).grid(row=6,column=0,columnspan=2,sticky='ew',padx=12,pady=(0,4));app.attachment_chips=AttachmentChips(compose,app.attach);app.attachment_chips.grid(row=7,column=0,columnspan=2,sticky='ew',padx=12,pady=(0,4))
    actions=ctk.CTkFrame(compose,fg_color='transparent');actions.grid(row=8,column=0,columnspan=2,sticky='ew',padx=12,pady=(3,10))
    ui.Button(actions,text='Adjuntar…',width=82,command=app._attachment).pack(side='left')
    ui.Button(actions,text='Guardar este',width=106,command=lambda:app._guard(app._send_draft)).pack(side='left',padx=5)
    app.save_all_button=ui.Button(actions,text='Guardar borradores',fg_color=ui.BLUE,width=135,command=lambda:app._guard(app._send_all));app.save_all_button.pack(side='right')
    app._mail_kind_changed();app._mail_modality_changed()
