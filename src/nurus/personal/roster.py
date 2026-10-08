"""Nóminas editables del producto actual, antes de crear archivos."""
from copy import deepcopy
from pathlib import Path
from uuid import uuid4
import tkinter as tk
from tkinter import ttk, simpledialog, messagebox
from .outputs import value, due_value, mail_metric_key, mail_metric_value, program_filename


def snapshot(work, rows, program, kind):
    if kind=='programa_espera':
        headers=['RIT','TRIBUNAL','NOMBRE','DERIVACION','T ESPERA'];keys=('rit','tribunal','nombre','programa','espera')
    elif kind in {'programa_vencido','programa_por_vencer'}:
        headers=['RIT','TRIBUNAL','NOMBRE','DERIVACION','F. VENCIMIENTO'];keys=('rit','tribunal','nombre','programa','vencimiento')
    else:
        metric={'espera':'T ESPERA','vencimiento':'F. VENCIMIENTO','egreso_proy':'F. EGRESO PROYECTADO'}[mail_metric_key(work,kind)]
        headers=['RIT','TRIBUNAL','RUT','NOMBRE','PROGRAMA',metric,'OBSERVACION'];keys=None
    data=[]
    for row in rows:
        cells=([mail_metric_value(work,row,k) or 'Sin dato' if k in ('espera','vencimiento') else value(work,row,k) for k in keys] if keys else
               [value(work,row,k) for k in ('rit','tribunal','rut','nombre','programa')]+[due_value(work,row,kind) or 'Sin dato',row.review.get('OBSERVACION',row.observation)])
        data.append({'id':row.id,'include':True,'cells':[str(v or '') for v in cells],'base':[str(v or '') for v in cells]})
    return {'program':program,'headers':headers,'rows':data,'generated':''}


def generate(draft, folder):
    """Generar exclusivamente las filas aceptadas; escribir sin sobrescribir originales."""
    from .reports import _book
    paths=[]
    from .operations import progress,checkpoint
    for index,group in enumerate(draft.roster):
        checkpoint();progress('Generando nóminas',index,len(draft.roster))
        rows=[r['cells'] for r in group['rows'] if r['include']]
        if not rows:continue
        if any(len(row)!=len(group['headers']) for row in rows):raise ValueError('La nómina contiene una fila con estructura incorrecta.')
        target=Path(folder)/'adjuntos'/uuid4().hex/(program_filename(group['program'])+'.xlsx')
        target.parent.mkdir(parents=True,exist_ok=True)
        paths.append(_book([('Nómina',group['headers'],rows)],target))
    if not paths:raise ValueError('Incluye al menos una fila en la nómina.')
    old={g.get('generated','') for g in draft.roster}
    draft.attachments=[p for p in draft.attachments if p not in old]+paths
    for group in draft.roster:group['generated']=''
    for group,path in zip([g for g in draft.roster if any(r['include'] for r in g['rows'])],paths):group['generated']=path
    draft.roster_reviewed=True
    return paths


def review(app):
    app._capture_mail()
    if app.draft_index is None:raise ValueError('Selecciona un correo preparado.')
    draft=app.drafts[app.draft_index]
    if not draft.roster:raise ValueError('Este correo no tiene una nómina automática pendiente.')
    groups=deepcopy(draft.roster)
    win=tk.Toplevel(app);win.title('Revisar nómina antes de generar adjuntos');win.geometry('950x570');win.transient(app)
    editors=[]
    tabs=ttk.Notebook(win);tabs.pack(fill='both',expand=True,padx=10,pady=10)
    for group in groups:
        page=ttk.Frame(tabs);tabs.add(page,text=group['program'][:35] or 'Sin programa')
        tree=ttk.Treeview(page,columns=['include',*range(len(group['headers']))],show='headings')
        tree.heading('include',text='Incluir');tree.column('include',width=65)
        for n,title in enumerate(group['headers']):tree.heading(str(n),text=title);tree.column(str(n),width=140)
        tree.pack(fill='both',expand=True)
        scroll=ttk.Scrollbar(page,orient='horizontal',command=tree.xview);scroll.pack(fill='x');tree.configure(xscrollcommand=scroll.set)
        vertical=ttk.Scrollbar(page,orient='vertical',command=tree.yview);vertical.pack(side='right',fill='y');tree.configure(yscrollcommand=vertical.set)
        for row in group['rows']:tree.insert('','end',iid=row['id'],values=['Sí' if row['include'] else 'No',*row['cells']])
        from .grid_edit import GridEditor
        editors.append(GridEditor(tree,group,win))
    ttk.Label(win,text='Doble clic / F2: editar. Tab / Enter: siguiente celda. Esc: cancelar. Ctrl+V: revisar pegado. Ctrl+Espacio: incluir/excluir.').pack(pady=4)
    def accept():
        for editor in editors:editor.commit()
        candidate=deepcopy(draft);candidate.roster=groups;candidate.roster_reviewed=False
        try:generate(candidate,Path(app.work.output).parent)
        except (ValueError,OSError) as exc:messagebox.showerror('Nómina',str(exc),parent=win);return
        draft.roster=candidate.roster;draft.attachments=candidate.attachments;draft.roster_reviewed=True
        app.attach.set('\n'.join(draft.attachments));app._save_session();win.destroy()
    ttk.Button(win,text='Aceptar nómina y generar adjuntos',command=accept).pack(pady=10)
