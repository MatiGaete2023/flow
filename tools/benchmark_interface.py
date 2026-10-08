"""Reproducible fictitious Treeview redraw comparison; no Office or user files."""
import argparse,json,platform,statistics,time,sys
from pathlib import Path
import tkinter as tk
from tkinter import ttk
from nurus.personal.table_update import upsert


def measure(rows,repeats):
    root=tk.Tk();root.withdraw();tree=ttk.Treeview(root,columns=('name','note'),show='headings')
    data=[(str(i),('Persona ficticia '+str(i),'Observación revisada')) for i in range(rows)]
    for iid,values in data:tree.insert('','end',iid=iid,values=values)
    def legacy():
        tree.delete(*tree.get_children())
        for iid,values in data:tree.insert('','end',iid=iid,values=values)
    def incremental():
        for iid,values in data:upsert(tree,iid,values)
    timings={}
    for label,fn in [('reconstruccion_completa',legacy),('refresco_sin_cambios',incremental)]:
        samples=[]
        for _ in range(repeats):
            started=time.perf_counter();fn();root.update_idletasks();samples.append(time.perf_counter()-started)
        timings[label]={'mediana_s':statistics.median(samples),'p95_s':sorted(samples)[min(len(samples)-1,__import__('math').ceil(.95*len(samples))-1)],'repeticiones':repeats}
    samples=[]
    for i in range(repeats):
        started=time.perf_counter();upsert(tree,'0',('Persona ficticia 0','Edición '+str(i)));root.update_idletasks();samples.append(time.perf_counter()-started)
    timings['edicion_una_fila']={'mediana_s':statistics.median(samples),'p95_s':sorted(samples)[-1],'repeticiones':repeats}
    root.destroy();return {'filas':rows,'escenario':'Redibujado de tabla; excluye lectura, reglas y Office','tiempos':timings}


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--rows',nargs='+',type=int,default=[1000,10000,50000]);p.add_argument('--repeats',type=int,default=10);p.add_argument('--output',type=Path,default=Path('artifacts/performance.json'));args=p.parse_args()
    if args.repeats<1 or any(n<1 or n>200000 for n in args.rows):p.error('Tamaño o repeticiones fuera de rango.')
    result={'equipo':platform.platform(),'procesador':platform.processor(),'python':sys.version,'nota':'Medición de dibujo, no rendimiento total. Ejecutar también en el Windows institucional.','escenarios':[measure(n,args.repeats) for n in args.rows]}
    args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(result,ensure_ascii=False,indent=2));print(json.dumps(result,ensure_ascii=False,indent=2))
