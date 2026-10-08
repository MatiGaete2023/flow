"""Revisión de datos, contactos y preferencias del trabajo actual."""
from .selection import selected_ids, remember, restore_visible
from collections import defaultdict
import tkinter as tk
from tkinter import ttk, messagebox
from .config import emails
from .sitfa import REQUIRED
from .outputs import value
from nurus.rus.columns import normalize
from nurus.rus.rules import as_date, as_int


def quality(work):
    issues=[];identities=defaultdict(list)
    for row in work.rows:
        for field in REQUIRED[work.mode]:
            raw=value(work,row,field)
            if raw is None or str(raw).strip()=='':issues.append((row.id,row.source_row,field,'Campo obligatorio vacío'))
            elif field in ('espera','dias_cumpl','dias_egresar') and as_int(raw) is None:issues.append((row.id,row.source_row,field,'Número no interpretable'))
            elif field=='vencimiento' and as_date(raw) is None:issues.append((row.id,row.source_row,field,'Fecha no interpretable'))
        key=tuple(normalize(value(work,row,k)) for k in ('tribunal','rit','rut','nombre','programa'))
        if all(key[i] for i in (0,1,3,4)):identities[key].append(row)
    for rows in identities.values():
        if len(rows)>1:issues.extend((r.id,r.source_row,'identidad','Identidad repetida; conservar y revisar origen') for r in rows)
    return issues


def show_quality(app):
    work=app._require_work();issues=quality(work)
    win=tk.Toplevel(app);win.title('Calidad del archivo actual');win.geometry('850x550');win.transient(app)
    tree=ttk.Treeview(win,columns=('fila','campo','aviso'),show='headings')
    for key,label in [('fila','Fila origen'),('campo','Campo'),('aviso','Incidencia')]:tree.heading(key,text=label)
    tree.pack(fill='both',expand=True,padx=12,pady=12)
    for n,(rid,fila,campo,aviso) in enumerate(issues):tree.insert('','end',iid=str(n),values=(fila,campo,aviso))
    def locate(event=None):
        selected=tree.selection()
        if selected:
            rid=issues[int(selected[0])][0];app.work_search.set('');app.work_filter.set('Todos');app._apply_work_filter();app.records.selection_set(rid);app._detail();app.tabs.select(app.pages['Trabajo'])
    tree.bind('<Double-1>',locate)
    ttk.Label(win,text=f'{len(issues)} incidencias. Doble clic para abrir la fila. Las filas repetidas no se eliminan automáticamente.').pack(pady=10)


def explain(app):
    work=app._require_work();selected=selected_ids(app,'records')
    if len(selected)!=1:raise ValueError('Selecciona una fila para ver su explicación.')
    row=next(r for r in work.rows if r.id==selected[0])
    lines=['Fila de origen: '+str(row.source_row),'Reglas activadas: '+(', '.join(row.rules) or 'Ninguna'),
           'Acciones: '+(', '.join(row.actions) or 'Ninguna'),'Datos utilizados:']
    lines.extend(f'{key}: {value(work,row,key)}' for key in work.mapping)
    lines+=['Umbrales configurados:']+[f'{key}: {val}' for key,val in work.config['umbrales'].items()]
    lines+=['Propuesta:\n'+row.observation,'Avisos:\n'+'\n'.join(row.warnings)]
    win=tk.Toplevel(app);win.title('Explicación de la sugerencia actual');win.geometry('780x600')
    text=tk.Text(win,wrap='word');text.pack(fill='both',expand=True);text.insert('1.0','\n'.join(lines));text.configure(state='disabled')


def matching_contacts(config,query):
    candidates=list(config.get('contactos',{}).items())
    candidates.extend((item.get('nombre',key),item.get('correo','')) for key,item in config.get('correos',{}).get('tribunales',{}).items())
    result=[];seen=set();wanted=normalize(query)
    for name,address in candidates:
        if wanted not in normalize(name+' '+address):continue
        try:parsed=emails(address)
        except ValueError:continue
        if not parsed:continue
        key=(name,'; '.join(parsed))
        if key not in seen:result.append(key);seen.add(key)
    return sorted(result)


def contact_search(app):
    win=tk.Toplevel(app);win.title('Elegir destinatario');win.geometry('760x470');win.transient(app)
    query=tk.StringVar();ttk.Entry(win,textvariable=query).pack(fill='x',padx=10,pady=10)
    tree=ttk.Treeview(win,columns=('nombre','correo'),show='headings');tree.heading('nombre',text='Contacto');tree.heading('correo',text='Dirección');tree.pack(fill='both',expand=True,padx=10)
    candidates=[]
    def refresh(*_):
        candidates[:]=matching_contacts(app.cfg.data,query.get());tree.delete(*tree.get_children())
        for n,(name,address) in enumerate(candidates):tree.insert('','end',iid=str(n),values=(name,address))
    def apply():
        selected=tree.selection()
        if len(selected)!=1:messagebox.showinfo('Contacto','Elige exactamente una coincidencia.',parent=win);return
        app.to.set(candidates[int(selected[0])][1]);app._capture_mail();win.destroy()
    query.trace_add('write',refresh);refresh();ttk.Button(win,text='Usar dirección seleccionada',command=apply).pack(pady=12)


def preferences(app):
    win=tk.Toplevel(app);win.title('Preferencias de vista');win.transient(app)
    variables={}
    for col in app.records['columns']:
        variable=tk.BooleanVar(value=col in app.records['displaycolumns'] or app.records['displaycolumns']==('#all',));variables[col]=variable
        ttk.Checkbutton(win,text=col,variable=variable).pack(anchor='w',padx=15,pady=4)
    def save():
        selected=[col for col,var in variables.items() if var.get()]
        if not selected:messagebox.showinfo('Vista','Mantén al menos una columna visible.',parent=win);return
        app.records.configure(displaycolumns=selected)
        cfg=dict(app.cfg.data);cfg['vista']={**cfg.get('vista',{}),'columnas_trabajo':selected};app.cfg.save(cfg);win.destroy()
    ttk.Button(win,text='Guardar vista',command=save).pack(pady=12)


def deadlines(work):
    """Separar las fechas que tienen significados distintos, conservando su origen."""
    items=[]
    for row in work.rows:
        if row.excluded:continue
        for field,label in [('vencimiento','Vencimiento de informe'),('egreso_proy','Egreso proyectado')]:
            raw=value(work,row,field)
            if raw is None or str(raw).strip()=='':continue
            parsed=as_date(raw)
            items.append((row.id,label,parsed.isoformat() if parsed else str(raw),value(work,row,'rit'),value(work,row,'nombre'),value(work,row,'tribunal'),row.source_row,'Fecha interpretable' if parsed else 'Revisar fecha'))
    return items


def show_deadlines(app):
    work=app._require_work();items=deadlines(work)
    win=tk.Toplevel(app);win.title('Fechas del trabajo actual');win.geometry('900x500')
    columns=('tipo','fecha','rit','persona','tribunal','fila','estado')
    tree=ttk.Treeview(win,columns=columns,show='headings')
    for col in columns:tree.heading(col,text=col.title());tree.column(col,width=130)
    tree.pack(fill='both',expand=True,padx=12,pady=12)
    for n,item in enumerate(items):tree.insert('','end',iid=str(n),values=item[1:])
    ttk.Label(win,text='El egreso proyectado se muestra separado del vencimiento de informe. Las fechas no se recalculan.').pack(pady=8)
