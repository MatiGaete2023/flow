"""Inline roster editing and transactional rectangular clipboard pasting."""
import csv
from io import StringIO
import tkinter as tk
from tkinter import ttk,messagebox


def paste_plan(group,visible_ids,start_id,start_column,text):
    rows={r['id']:r for r in group['rows']}
    if start_id not in visible_ids or start_column<0:raise ValueError('Selecciona una celda de datos.')
    matrix=list(csv.reader(StringIO(text),delimiter='\t'))
    if not matrix or not matrix[0]:raise ValueError('El portapapeles no contiene celdas.')
    width=len(matrix[0])
    if any(len(row)!=width for row in matrix):raise ValueError('Todas las filas pegadas deben tener el mismo número de columnas.')
    start=visible_ids.index(start_id)
    if start+len(matrix)>len(visible_ids) or start_column+width>len(group['headers']):raise ValueError('El pegado excede las filas o columnas de la nómina.')
    plan=[]
    for offset,cells in enumerate(matrix):
        rid=visible_ids[start+offset]
        if rid not in rows:raise ValueError('La fila ya no está disponible.')
        for col,value in enumerate(cells,start_column):
            if '\x00' in value or len(value)>32767:raise ValueError('Una celda contiene texto no permitido por Excel.')
            plan.append((rid,col,value))
    return plan


def apply_plan(group,plan):
    rows={r['id']:r for r in group['rows']}
    # Validate the entire plan before touching even the first cell.
    for rid,col,value in plan:
        if rid not in rows or not 0<=col<len(rows[rid]['cells']):raise ValueError('El destino del pegado cambió.')
    for rid,col,value in plan:rows[rid]['cells'][col]=value


class GridEditor:
    def __init__(self,tree,group,parent):
        self.tree=tree;self.group=group;self.parent=parent;self.editor=None;self.current=None
        tree.bind('<Double-1>',self.edit_at);tree.bind('<F2>',self.edit_selected);tree.bind('<Return>',self.edit_selected)
        tree.bind('<Control-v>',self.paste);tree.bind('<Control-space>',self.toggle)

    def render(self,rid):
        row=next(r for r in self.group['rows'] if r['id']==rid)
        self.tree.item(rid,values=['Sí' if row['include'] else 'No',*row['cells']])

    def cancel(self,event=None):
        if self.editor:self.editor.destroy();self.editor=None
        self.tree.focus_set();return 'break'

    def commit(self,event=None):
        if not self.editor:return 'break'
        rid,col=self.current;value=self.editor.get()
        if '\x00' in value or len(value)>32767:messagebox.showerror('Celda','Texto no permitido por Excel.',parent=self.parent);return 'break'
        apply_plan(self.group,[(rid,col,value)]);self.cancel();self.render(rid);return 'break'

    def begin(self,rid,col):
        self.commit();bbox=self.tree.bbox(rid,f'#{col+2}')
        if not bbox:return 'break'
        row=next(r for r in self.group['rows'] if r['id']==rid);self.current=(rid,col)
        x,y,w,h=bbox;self.editor=ttk.Entry(self.tree);self.editor.place(x=x,y=y,width=w,height=h)
        self.editor.insert(0,row['cells'][col]);self.editor.select_range(0,'end');self.editor.focus_set()
        self.editor.bind('<Control-v>',self.paste);self.editor.bind('<Escape>',self.cancel);self.editor.bind('<Return>',lambda e:self.advance(False));self.editor.bind('<Tab>',lambda e:self.advance(True));return 'break'

    def advance(self,horizontal):
        rid,col=self.current;self.commit();ids=list(self.tree.get_children());index=ids.index(rid)
        if horizontal:
            col+=1
            if col==len(self.group['headers']):col=0;index+=1
        else:index+=1
        if index<len(ids):self.tree.see(ids[index]);self.tree.selection_set(ids[index]);self.begin(ids[index],col)
        return 'break'

    def edit_at(self,event):
        rid=self.tree.identify_row(event.y);column=self.tree.identify_column(event.x)
        if not rid or not column:return 'break'
        col=int(column[1:])-2;self.tree.selection_set(rid)
        if col<0:return self.toggle()
        return self.begin(rid,col)

    def edit_selected(self,event=None):
        selected=self.tree.selection()
        if selected:return self.begin(selected[0],0)
        return 'break'

    def toggle(self,event=None):
        self.commit()
        for rid in self.tree.selection():
            row=next(r for r in self.group['rows'] if r['id']==rid);row['include']=not row['include'];self.render(rid)
        return 'break'

    def paste(self,event=None):
        selected=self.tree.selection()
        if not selected:return 'break'
        rid,col=self.current if self.editor else (selected[0],0)
        self.commit()
        try:plan=paste_plan(self.group,list(self.tree.get_children()),rid,col,self.tree.clipboard_get())
        except (ValueError,tk.TclError) as exc:messagebox.showerror('Revisar pegado',str(exc),parent=self.parent);return 'break'
        preview=tk.Toplevel(self.parent);preview.title('Revisar celdas antes de aplicar');preview.geometry('750x440');preview.transient(self.parent)
        text=tk.Text(preview,wrap='word');text.pack(fill='both',expand=True,padx=12,pady=12)
        for target,column,value in plan:text.insert('end',self.group['headers'][column]+' · '+target[:10]+' → '+value+'\n')
        text.configure(state='disabled')
        def accept():
            apply_plan(self.group,plan)
            for target in dict.fromkeys(p[0] for p in plan):self.render(target)
            preview.destroy()
        ttk.Button(preview,text=f'Aplicar {len(plan)} celdas',command=accept).pack(pady=10)
        ttk.Button(preview,text='Cancelar',command=preview.destroy).pack(pady=(0,10));return 'break'
