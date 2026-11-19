"""Ventana «Revisión de bitácoras» del Asistente (P2): separa los registros con observación, sin observación y no
verificables en un período, permite seleccionarlos en Trabajo y exportar Excel + JSON. No cambia propuestas ni el motor."""

import tkinter as tk
from datetime import date, datetime
from tkinter import filedialog, messagebox, ttk

from .outputs import value
from .revision_bitacoras import NOMBRES, exportar, periodo, periodo_por_defecto, resumen, revisar

TODOS = 'Todos'


def abrir(app):
    work = app._require_work()
    corte = date.fromisoformat(work.as_of) if getattr(work, 'as_of', '') else datetime.now().astimezone().date()
    desde, hasta = periodo_por_defecto(corte)
    win = tk.Toplevel(app)
    win.title('Revisión de bitácoras')
    win.transient(app)
    win.geometry('980x560')
    arriba = ttk.Frame(win, padding=10)
    arriba.pack(fill='x')
    ttk.Label(arriba, text='Período · desde').pack(side='left')
    var_desde = tk.StringVar(value=desde.strftime('%d/%m/%Y'))
    ttk.Entry(arriba, textvariable=var_desde, width=12).pack(side='left', padx=4)
    ttk.Label(arriba, text='hasta').pack(side='left')
    var_hasta = tk.StringVar(value=hasta.strftime('%d/%m/%Y'))
    ttk.Entry(arriba, textvariable=var_hasta, width=12).pack(side='left', padx=4)
    filtro = tk.StringVar(value=NOMBRES['SIN_OBSERVACION'])
    ttk.Label(arriba, text='Mostrar').pack(side='left', padx=(16, 4))
    caja = ttk.Combobox(arriba, textvariable=filtro, values=[TODOS, *NOMBRES.values()], state='readonly', width=18)
    caja.pack(side='left')
    conteo = tk.StringVar()
    ttk.Label(win, textvariable=conteo, padding=(10, 0)).pack(fill='x')
    ttk.Label(
        win,
        padding=(10, 0),
        text='«Sin observación» exige bitácora leída completa, sin indicios de recorte y leída en o después de «hasta». '
        'La propuesta de cada registro no cambia.',
        wraplength=940,
    ).pack(fill='x')
    columnas = ('clase', 'rit', 'nombre', 'programa', 'motivo')
    tabla = ttk.Treeview(win, columns=columnas, show='headings', selectmode='extended')
    for col, titulo, ancho in zip(columnas, ('Clasificación', 'RIT', 'Nombre', 'Programa', 'Motivo'), (130, 110, 200, 180, 340)):
        tabla.heading(col, text=titulo)
        tabla.column(col, width=ancho, anchor='w')
    tabla.pack(fill='both', expand=True, padx=10, pady=6)
    estado = {'items': [], 'periodo': (desde, hasta)}

    def mostrar(*_):
        tabla.delete(*tabla.get_children())
        elegido = filtro.get()
        for item in estado['items']:
            nombre = NOMBRES[item['clasificacion']]
            if elegido not in (TODOS, nombre):
                continue
            fila = item['row']
            tabla.insert(
                '',
                'end',
                iid=fila.id,
                values=(nombre, value(work, fila, 'rit'), value(work, fila, 'nombre'), value(work, fila, 'programa'), item['motivo']),
            )

    def clasificar():
        try:
            estado['periodo'] = periodo(var_desde.get(), var_hasta.get())
        except ValueError as exc:
            messagebox.showinfo('Revisión de bitácoras', str(exc), parent=win)
            return
        estado['items'] = revisar(work, *estado['periodo'])
        cuenta = resumen(estado['items'])
        conteo.set(' · '.join(f'{NOMBRES[k]}: {v}' for k, v in cuenta.items()))
        mostrar()

    def visibles():
        return list(tabla.selection()) or list(tabla.get_children())

    def seleccionar_en_trabajo():
        ids = [rid for rid in visibles() if app.records.exists(rid)]
        if not ids:
            messagebox.showinfo('Revisión de bitácoras', 'No hay registros visibles en Trabajo para seleccionar.', parent=win)
            return
        app.records.selection_set(ids)
        app.records.see(ids[0])
        app.status.set(f'{len(ids)} registros seleccionados en Trabajo desde la revisión de bitácoras.')

    def exportar_productos():
        if not estado['items']:
            clasificar()
        carpeta = filedialog.askdirectory(parent=win, title='Carpeta donde crear la revisión')
        if not carpeta:
            return
        from pathlib import Path

        destino = Path(carpeta) / ('Revision_bitacoras_' + datetime.now().astimezone().strftime('%Y-%m-%d_%H%M%S'))
        ids = set(visibles())
        items = [i for i in estado['items'] if i['row'].id in ids]
        try:
            excel, _ = exportar(work, items, *estado['periodo'], destino)
        except (ValueError, OSError) as exc:
            messagebox.showerror('Revisión de bitácoras', str(exc), parent=win)
            return
        app.status.set(f'Revisión de bitácoras exportada: {excel.parent}')
        messagebox.showinfo('Revisión de bitácoras', f'Se creó {excel.parent}', parent=win)

    abajo = ttk.Frame(win, padding=10)
    abajo.pack(fill='x')
    ttk.Button(abajo, text='Clasificar', command=clasificar).pack(side='left')
    ttk.Button(abajo, text='Seleccionar en Trabajo', command=seleccionar_en_trabajo).pack(side='left', padx=6)
    ttk.Button(abajo, text='Exportar Excel y JSON…', command=exportar_productos).pack(side='left')
    ttk.Button(abajo, text='Cerrar', command=win.destroy).pack(side='right')
    caja.bind('<<ComboboxSelected>>', mostrar)
    clasificar()
