"""Contextual form with explicit empty values and reversible bulk edits."""
from .selection import selected_ids, remember, restore_visible
import tkinter as tk
import customtkinter as ctk

from . import ui
from .widgets import ScrollPane, NamedChoice
from .record_edits import MAIL_KINDS, WORD_FIELDS, apply, undo
from .outputs import word_values

RES_NAMES = {'auto':'Automático', 'none':'Sin proyecto', 'PC_IE':'PC_IE · Ingreso efectivo',
             'PC_INFO':'PC_INFO · Informe', 'NOMENCL':'NOMENCL · Nomenclatura', 'review':'Según RES de Excel'}
MAIL_NAMES = {'auto':'Automático', 'include':'Incluir', 'omit':'Omitir'}


class RecordForm:
    def __init__(self, app, notebook):
        self.app=app
        self.row_id=None
        self.loading=False
        self.dirty=False
        self.last_edit=None
        self.review={}
        self.mail={}
        self.source={}
        self.word={}
        management=ScrollPane(notebook);notebook.add(management,text='Gestión')
        frame=management.body
        ui.Label(frame,text='Vacío significa sin indicar. CC es la carga, no una dirección de correo.').pack(anchor='w',pady=6)
        for key,label in [('FECHA_OBS','Fecha de observación · dd/mm/aaaa'),('TT','TT'),('CC','CC de carga')]:
            line=ui.Frame(frame);line.pack(fill='x',pady=3)
            ui.Label(line,text=label,width=260,anchor='w').pack(side='left')
            variable=self.variable();self.review[key]=variable
            widget=ui.Entry(line,textvariable=variable) if key=='FECHA_OBS' else ui.Combobox(line,textvariable=variable,values=['','0','1'],state='readonly')
            widget.pack(side='left',fill='x',expand=True)
        self.resolution=self.variable('auto')
        ui.Label(frame,text='Proyecto de resolución').pack(anchor='w',pady=(10,2))
        NamedChoice(frame,keyvariable=self.resolution,names=lambda:RES_NAMES,state='readonly').pack(fill='x')
        self.excluded=self.variable(False,boolean=True)
        ui.Checkbutton(frame,text='Excluir del lote actual',variable=self.excluded).pack(anchor='w',pady=8)
        ui.Label(frame,text='Gestiones de correo · independientes de la redacción de la observación').pack(anchor='w',pady=6)
        for kind in MAIL_KINDS:
            line=ui.Frame(frame);line.pack(fill='x',pady=3)
            label=app.cfg.data['correos']['plantillas'][kind]['nombre']
            ui.Label(line,text=label,width=260,anchor='w').pack(side='left')
            variable=self.variable('auto');self.mail[kind]=variable
            NamedChoice(line,keyvariable=variable,names=lambda:MAIL_NAMES,state='readonly').pack(side='left',fill='x',expand=True)
        actions=ui.Frame(frame);actions.pack(fill='x',pady=10)
        ui.Button(actions,text='Aplicar al registro',command=lambda:app._guard(self.save)).pack(side='left')
        ui.Button(actions,text='Edición masiva…',command=lambda:app._guard(self.bulk)).pack(side='left',padx=5)
        ui.Button(actions,text='Deshacer última edición',command=lambda:app._guard(self.revert)).pack(side='left')
        self.source_panel=ScrollPane(notebook);notebook.add(self.source_panel,text='Datos de origen')
        ui.Label(self.source_panel.body,text='Correcciones locales. El origen permanece archivado. Vacío restaura el dato original.').pack(anchor='w',pady=6)
        self.source_fields=ui.Frame(self.source_panel.body);self.source_fields.pack(fill='both',expand=True)
        ui.Button(self.source_panel.body,text='Aplicar y recalcular',command=lambda:app._guard(self.save)).pack(anchor='e',pady=8)
        word_panel=ScrollPane(notebook);notebook.add(word_panel,text='Datos Word')
        ui.Label(word_panel.body,text='Completa una variable o déjala vacía para usar el dato automático.').pack(anchor='w',pady=6)
        self.word_entries={}
        for key in WORD_FIELDS:
            ui.Label(word_panel.body,text=key.replace('_',' ')).pack(anchor='w')
            variable=self.variable();self.word[key]=variable
            entry=ui.Entry(word_panel.body,textvariable=variable);entry.pack(fill='x',pady=(0,5))
            self.word_entries[key]=entry
        ui.Button(word_panel.body,text='Aplicar datos Word',command=lambda:app._guard(self.save)).pack(anchor='e',pady=8)

    def variable(self, initial='', boolean=False):
        variable=(tk.BooleanVar if boolean else tk.StringVar)(value=initial)
        variable.trace_add('write', self.changed)
        return variable

    def changed(self,*_):
        if not self.loading:self.dirty=True

    def load(self,row):
        self.loading=True
        self.row_id=row.id
        for key,variable in self.review.items():
            from .work import _review_value
            raw=_review_value(row,key)
            if key=='FECHA_OBS' and raw:
                from nurus.rus.rules import as_date
                parsed=as_date(raw)
                if parsed:raw=parsed.strftime('%d/%m/%Y')
            variable.set(raw)
        mode=row.decisions.get('resolution','review' if 'RES' in row.review else 'auto')
        self.resolution.set(mode)
        self.excluded.set(row.excluded)
        for kind,variable in self.mail.items():variable.set(row.decisions.get('mail',{}).get(kind,'auto'))
        for child in self.source_fields.winfo_children():child.destroy()
        self.source={}
        for key,column in self.app.work.mapping.items():
            line=ui.Frame(self.source_fields);line.pack(fill='x',pady=3)
            ui.Label(line,text=column,width=210,anchor='w').pack(side='left')
            variable=self.variable(str(row.overrides.get(key,'')));self.source[key]=variable
            ui.Entry(line,textvariable=variable,placeholder_text=str(row.values.get(column,'') or '')).pack(side='left',fill='x',expand=True)
        automatic=word_values(self.app.work,row)
        for key,variable in self.word.items():
            variable.set(row.word_overrides.get(key,''))
            self.word_entries[key].configure(placeholder_text=str(automatic.get(key,'')))
        self.loading=False
        self.dirty=False

    def capture(self):
        if not self.row_id or not self.dirty:return
        work=self.app.work
        review={key:variable.get() for key,variable in self.review.items()}
        decisions={'resolution':self.resolution.get(),'mail':{key:variable.get() for key,variable in self.mail.items()}}
        overrides={key:variable.get().strip() for key,variable in self.source.items() if variable.get().strip()}
        word={key:variable.get().strip() for key,variable in self.word.items() if variable.get().strip()}
        old=next(row for row in work.rows if row.id==self.row_id)
        self.last_edit=apply(work,[self.row_id],review=review,decisions=decisions,
                             overrides=overrides if overrides!=old.overrides else None,
                             word_overrides=word,excluded=self.excluded.get())
        updated=next(row for row in work.rows if row.id==self.row_id)
        if overrides!=old.overrides and 'OBSERVACION' not in updated.review and self.app.observation_id==self.row_id:
            self.app.observation_editor.delete('1.0','end')
            self.app.observation_editor.insert('1.0',updated.observation)
            self.app.observation_editor._textbox.edit_reset()
        self.dirty=False

    def save(self):
        self.app._capture_observation()
        self.capture()
        self.app._save_session()
        self.app._show_work()
        if self.row_id and self.app.records.exists(self.row_id):
            self.app.records.selection_set(self.row_id)
            self.app._detail()
        self.app.status.set('Cambios guardados localmente. Exporta una copia cuando lo necesites.')

    def revert(self):
        if not self.last_edit:raise ValueError('No hay una edición de formulario que deshacer.')
        undo(self.app.work,self.last_edit)
        self.last_edit=None
        self.dirty=False
        self.app._show_work()
        self.app._save_session()

    def bulk(self):
        selected=list(selected_ids(self.app,'records'))
        if not selected:raise ValueError('Selecciona registros en Trabajo.')
        window=ctk.CTkToplevel(self.app);window.title(f'Editar {len(selected)} registros');window.geometry('600x380');window.transient(self.app)
        ui.Label(window,text=f'Solo se aplicarán los campos marcados a {len(selected)} registros.').pack(padx=12,pady=12)
        choices={}
        for key,label,values in [('FECHA_OBS','Fecha · dd/mm/aaaa',None),('TT','TT',['','0','1']),('CC','CC de carga',['','0','1']),('RES','Proyecto',['','PC_IE','PC_INFO','NOMENCL']),('excluded','Exclusión',['Incluir','Excluir'])]:
            line=ui.Frame(window);line.pack(fill='x',padx=12,pady=5)
            enabled=tk.BooleanVar();variable=tk.StringVar(value='Incluir' if key=='excluded' else '')
            ui.Checkbutton(line,text=label,variable=enabled,width=240).pack(side='left')
            widget=ui.Entry(line,textvariable=variable) if values is None else ui.Combobox(line,textvariable=variable,values=values,state='readonly')
            widget.pack(side='left',fill='x',expand=True)
            choices[key]=(enabled,variable)
        def commit():
            changes={key:var.get() for key,(enabled,var) in choices.items() if enabled.get()}
            exclusion=changes.pop('excluded',None)
            decisions={'resolution':changes['RES'] or 'none'} if 'RES' in changes else None
            self.last_edit=apply(self.app.work,selected,review=changes,decisions=decisions,excluded=None if exclusion is None else exclusion=='Excluir')
            self.dirty=False
            window.destroy();self.app._show_work();self.app._save_session()
        ui.Button(window,text='Aplicar cambios marcados',command=lambda:self.app._guard(commit)).pack(pady=12)
