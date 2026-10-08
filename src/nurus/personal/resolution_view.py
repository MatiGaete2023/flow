"""One case list and four focused project panels."""
from .selection import selected_ids, remember, restore_visible
import tkinter as tk
from . import ui
from .widgets import CaseDetailPanel, ScrollPane
from .resolutions import KINDS, KIND_LABELS, _case_key


class ProjectSelection:
    """Listbox-compatible selection model, with no duplicated visible list."""
    def __init__(self,app):self.app=app;self.labels=[];self.index=None
    def delete(self,*args):self.labels=[];self.index=None
    def insert(self,position,label):self.labels.append(label)
    def selection_clear(self,*args):self.index=None
    def selection_set(self,index):self.index=int(index)
    def curselection(self):return () if self.index is None else (self.index,)


def build(app):
    page=app.pages['Resoluciones']
    actions=ui.Frame(page);actions.pack(fill='x',pady=4)
    app.manual_word=tk.StringVar(value=KIND_LABELS['PC_IE'])
    app.word_type_box=ui.Combobox(actions,textvariable=app.manual_word,values=[KIND_LABELS[k] for k in KINDS],state='readonly',width=36)
    app.word_type_box.pack(side='left')
    ui.Button(actions,text='Aplicar tipo',width=100,command=lambda:app._guard(app._assign_word_type)).pack(side='left',padx=5)
    ui.Button(actions,text='Agregar desde Trabajo',width=145,command=lambda:app._guard(app._add_words)).pack(side='left')
    def set_decision(mode):
        work=app._require_work()
        selected=selected_ids(app,'words')
        if not selected:raise ValueError('Selecciona las causas que quieres cambiar.')
        by_id={row.id:row for row in work.rows}
        cases={_case_key(work,by_id[iid.split('|')[0]]) for iid in selected}
        ids=[row.id for row in work.rows if _case_key(work,row) in cases]
        from .record_edits import apply
        apply(work,ids,decisions={'resolution':mode})
        app._show_work();app._save_session()
    ui.Button(actions,text='Quitar del lote',width=105,command=lambda:app._guard(lambda:set_decision('none'))).pack(side='right')
    bar=ui.Frame(page);bar.pack(fill='x',pady=4)
    ui.Button(bar,text='Preparar / actualizar proyectos',width=205,command=lambda:app._guard(app._prepare_words)).pack(side='left')
    from .manual import open_editor
    ui.Button(bar,text='Word manual',width=120,command=lambda:app._guard(lambda:open_editor(app,'word'))).pack(side='left',padx=4)
    ui.Button(bar,text='Restaurar automático',width=150,command=lambda:app._guard(lambda:set_decision('auto'))).pack(side='left',padx=5)
    app.generate_word_button=ui.Button(bar,text='Generar Word',width=135,command=lambda:app._guard(app._generate_words))
    app.generate_word_button.pack(side='right')
    ui.Button(bar,text='Abrir Word',width=95,command=lambda:app._guard(lambda:app._open(app.last_word))).pack(side='right',padx=5)
    filters=ui.Frame(page);filters.pack(fill='x',pady=5)
    app.resolution_search=tk.StringVar();app.resolution_filter=tk.StringVar(value='Todos')
    app.resolution_search_entry=ui.Entry(filters,textvariable=app.resolution_search,width=28,placeholder_text='Buscar causa o persona')
    app.resolution_search_entry.pack(side='left',fill='x',expand=True)
    box=ui.Combobox(filters,textvariable=app.resolution_filter,values=['Todos','Definido en RES','Ajustado manualmente','Sugerencia automática','RES antiguo'],state='readonly',width=24)
    box.pack(side='left',padx=5)
    box.bind('<<ComboboxSelected>>',app._apply_resolution_filter)
    app.resolution_search.trace_add('write',app._apply_resolution_filter)
    app.resolution_split=split=ui.Panedwindow(page,orient='horizontal');split.pack(fill='both',expand=True)
    left=ui.Frame(split);right=ui.Frame(split);split.add(left,weight=2);split.add(right,weight=3)
    app.words=app._tree(left,('RIT','Tribunal','Tipo','Origen'))
    for key,width in [('RIT',80),('Tribunal',80),('Tipo',140),('Origen',140)]:app.words.column(key,width=width,minwidth=65)
    app.words.tag_configure('res_explicit',background='#1f3b2d',foreground='#d9fbe7')
    app.words.tag_configure('res_manual',background='#234047',foreground='#d8f6fa')
    app.words.tag_configure('res_legacy',background='#4a3b20',foreground='#fff0c2')
    app.project_list=ProjectSelection(app)
    def select(event=None):
        if app.busy:return
        app._resolution_detail()
        if not selected_ids(app,'words'):return
        rid,kind=selected_ids(app,'words')[0].split('|')
        index=next((i for i,p in enumerate(app.projects) if rid in p.record_ids and p.kind==kind),None)
        if index is not None:
            app.project_list.selection_set(index);app._select_project()
    app.words.bind('<<TreeviewSelect>>',select)
    tabs=ui.Notebook(right);tabs.pack(fill='both',expand=True)
    text=ui.Frame(tabs);tabs.add(text,text='Texto')
    from .word_preview import preview
    ui.Button(text,text='Vista PDF del Word generado',command=lambda:app._guard(lambda:preview(app))).pack(anchor='w',pady=4)
    ui.Label(text,text='Texto editable · revisa el formato final en Word',text_color=ui.MUTED).pack(anchor='w')
    app.project_editor=ui.Textbox(text,wrap='word',height=18,font=('Segoe UI',11),undo=True);app.project_editor.pack(fill='both',expand=True)
    app.project_review_status=tk.StringVar()
    ui.Label(text,textvariable=app.project_review_status,wraplength=420).pack(fill='x')
    def accept():
        app._capture_project()
        if app.project_index is None:raise ValueError('Selecciona un proyecto preparado.')
        app.projects[app.project_index].review_required=False
        app.project_review_status.set('Cambios revisados.')
        app._save_session()
    ui.Button(text,text='Confirmar revisión de cambios',command=lambda:app._guard(accept)).pack(anchor='e',pady=4)
    data=ScrollPane(tabs);tabs.add(data,text='Datos')
    app.resolution_case_detail=CaseDetailPanel(data.body);app.resolution_case_detail.pack(fill='x',pady=5)
    app.project_data_fields=ui.Frame(data.body);app.project_data_fields.pack(fill='x')
    ui.Button(data.body,text='Aplicar datos al proyecto',command=lambda:app._guard(lambda:apply_values(app))).pack(anchor='e',pady=8)
    matrix=ScrollPane(tabs);tabs.add(matrix,text='Matriz')
    app.project_matrix_text=tk.StringVar()
    ui.Label(matrix.body,textvariable=app.project_matrix_text,wraplength=420,justify='left').pack(fill='x',pady=8)
    ui.Button(matrix.body,text='Abrir matriz en Word',command=lambda:app._guard(lambda:app._open(app.projects[app.project_index].template) if app.project_index is not None else None)).pack(anchor='w')
    def fit(event=None):
        width=split.winfo_width()
        if width>500:split.sashpos(0,max(240,int(width*getattr(app,'resolution_sash_ratio',.36))))
    split.bind('<Configure>',fit)
    split.bind('<ButtonRelease-1>',lambda event:setattr(app,'resolution_sash_ratio',split.sashpos(0)/max(1,split.winfo_width())))


def show_values(app,project):
    if not hasattr(app,'project_data_fields'):return
    for child in app.project_data_fields.winfo_children():child.destroy()
    app.project_value_vars={}
    for key,value in project.values.items():
        if key.startswith('_'):continue
        ui.Label(app.project_data_fields,text=key.replace('_',' ')).pack(anchor='w')
        variable=tk.StringVar(value=str(value));app.project_value_vars[key]=variable
        ui.Entry(app.project_data_fields,textvariable=variable).pack(fill='x',pady=(0,5))
    app.project_matrix_text.set(f'{project.court} · {project.kind}\n{project.template}\nVersión: {project.template_hash[:16]}')
    app.project_review_status.set('Cambió la base: revisa y confirma la revisión antes de generar.' if project.review_required else '')


def apply_values(app):
    if app.project_index is None:raise ValueError('Selecciona un proyecto preparado.')
    app._capture_project()
    from copy import deepcopy
    index=app.project_index
    project=deepcopy(app.projects[index])
    previous=project.text
    edited=project.text!=project.original_text
    project.values.update({key:variable.get() for key,variable in app.project_value_vars.items()})
    rerender(project)
    if edited:
        project.text=previous;project.review_required=True
    app.projects[index]=project
    app.project_index=None
    app._select_project()
    app._save_session()


def rerender(project):
    from pathlib import Path
    from tempfile import TemporaryDirectory
    from .resolutions import render_project, paragraphs
    with TemporaryDirectory() as directory:
        document=render_project(project,Path(directory)/'vista.docx')
        project.original_text='\n'.join(p.text for p in paragraphs(document))
        project.text=project.original_text
