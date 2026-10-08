"""Resultados y recuperación del flujo conjunto, sin mezclar los períodos."""
from pathlib import Path
import tkinter as tk
from tkinter import ttk,messagebox
from resultados import read_json,inside,digest_file
from ventana_resultados import open_local
from flujo_csmp import source_folder
import motor as m


class JointWindow(tk.Toplevel):
    def __init__(self,app):
        super().__init__(app.root);self.busy=False;app.result_windows.append(self)
        self.title('Resultados · descarga conjunta CSMP');self.geometry('820x560')
        self.root_folder=Path(app.last_folder).resolve();data=read_json(self.root_folder/'flujo.json')
        ttk.Label(self,text=data['estado'],wraplength=760).pack(anchor='w',padx=12,pady=10)
        scope=data['seleccion']
        ttk.Label(self,text=f"{scope['modo']} completo · corte {scope['corte']} · informes del mes actual y siguiente",wraplength=760).pack(anchor='w',padx=12)
        ttk.Label(self,text='La consulta de resoluciones firmadas se retiró de la descarga conjunta. No se acredita ausencia de movimientos.',wraplength=760).pack(anchor='w',padx=12,pady=8)
        tree=ttk.Treeview(self,columns=('fuente','estado'),show='headings');tree.heading('fuente',text='Fuente');tree.heading('estado',text='Estado')
        tree.pack(fill='both',expand=True,padx=12,pady=5)
        for index,(key,phase) in enumerate(data['fases'].items()):tree.insert('','end',iid=str(index),values=(key,phase['estado']))
        def open_phase(event=None):
            selected=tree.selection()
            if selected:
                phase=list(data['fases'].values())[int(selected[0])]
                if phase.get('carpeta'):open_local(source_folder(self.root_folder,phase['carpeta']))
        tree.bind('<Double-1>',open_phase)
        buttons=ttk.Frame(self);buttons.pack(fill='x',padx=12,pady=12)
        def open_result():
            try:
                path=inside(self.root_folder,data['archivo'])
                if digest_file(path)!=data.get('sha256'):raise m.PocError('El resultado cambió; genera una copia nueva desde las fuentes.')
                open_local(path)
            except (m.PocError,OSError) as exc:messagebox.showerror('Resultado no disponible',str(exc),parent=self)
        if data.get('archivo'):ttk.Button(buttons,text='Abrir libro para CSMP',command=open_result).pack(side='left',padx=4)
        ttk.Button(buttons,text='Abrir carpeta',command=lambda:open_local(self.root_folder)).pack(side='left',padx=4)
        ttk.Button(buttons,text='Retomar pendientes',command=lambda:app.resume_joint(self.root_folder)).pack(side='left',padx=4)
