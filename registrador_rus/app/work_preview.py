"""Vista previa local en el Asistente. No exporta archivos ni se conecta a RUS."""
from datetime import datetime, timezone
import tkinter as tk
from tkinter import ttk, messagebox

from .contracts import ContractError, ORIGINS, DESTINATIONS
from .flow_batch import ReviewRequest, build_review, confirm_review
from .flow_work import REPOSITORY_REPORT_CATALOG
from .transfer import PreviewBatch


def visible_selection(app):
    # No incluir selecciones lógicas ocultas por filtros del Asistente.
    visible = set(app.records.get_children())
    ids = tuple(r for r in app.records.selection() if r in visible)
    if not 1 <= len(ids) <= 400:
        raise ContractError('SELECCIONA_ENTRE_1_Y_400_FILAS_VISIBLES')
    return ids


class WorkPreviewDialog:
    def __init__(self, app):
        self.app = app
        self.work = app._require_work()
        self.ids = visible_selection(app)
        self.work_id = getattr(self.work,'capture_id','') or self.work.capture_digest
        self.frozen = None
        self.preview = None
        self.window = tk.Toplevel(app)
        self.window.title('Vista previa local de lote — sin envío')
        self.window.transient(app)
        self.window.minsize(740, 490)
        self.origin = tk.StringVar(master=self.window)
        self.destination = tk.StringVar(master=self.window)
        self.state = tk.StringVar(master=self.window)
        self.accepted = tk.BooleanVar(master=self.window,value=False)
        self.status = tk.StringVar(master=self.window,value='No se guarda ningún archivo ni se autoriza escritura.')
        body = ttk.Frame(self.window,padding=12)
        body.pack(fill='both',expand=True)
        ttk.Label(body,text=f'{len(self.ids)} filas visibles seleccionadas. Un origen, destino y Estado por grupo.').pack(anchor='w')
        controls = ttk.Frame(body)
        controls.pack(fill='x',pady=6)
        for label, var, values in (
                ('Origen',self.origin,sorted(ORIGINS)),('Destino',self.destination,sorted(DESTINATIONS))):
            ttk.Label(controls,text=label).pack(side='left',padx=3)
            ttk.Combobox(controls,textvariable=var,values=values,state='readonly',width=23).pack(side='left')
        state_row=ttk.Frame(body)
        state_row.pack(fill='x',pady=4)
        ttk.Label(state_row,text='Código de Estado (no validado en RUS)').pack(side='left')
        ttk.Entry(state_row,textvariable=self.state,width=16).pack(side='left',padx=6)
        ttk.Button(state_row,text='Preparar revisión',command=lambda:self.run(self.prepare)).pack(side='left')
        panel=ttk.Frame(body)
        panel.pack(fill='both',expand=True,pady=6)
        self.text=tk.Text(panel,wrap='word',height=14,state='disabled')
        scroll=ttk.Scrollbar(panel,orient='vertical',command=self.text.yview)
        self.text.configure(yscrollcommand=scroll.set)
        scroll.pack(side='right',fill='y')
        self.text.pack(side='left',fill='both',expand=True)
        ttk.Checkbutton(body,text='Confirmo el contenido exacto mostrado, solo para previsualización.',
                        variable=self.accepted,command=self.toggle).pack(anchor='w')
        row=ttk.Frame(body)
        row.pack(fill='x',pady=5)
        self.confirm_button=ttk.Button(row,text='Confirmar vista previa',state='disabled',command=lambda:self.run(self.confirm))
        self.confirm_button.pack(side='right')
        ttk.Button(row,text='Cerrar',command=self.window.destroy).pack(side='right',padx=4)
        ttk.Label(body,textvariable=self.status,wraplength=700).pack(anchor='w')
        for var in (self.origin,self.destination,self.state):
            var.trace_add('write',self.invalidate)
        self.window.grab_set()

    def run(self, action):
        try:
            return action()
        except (ValueError,RuntimeError) as exc:
            self.invalidate()
            messagebox.showwarning('Vista previa bloqueada',str(exc),parent=self.window)

    def invalidate(self,*_):
        self.frozen=None
        self.preview=None
        self.accepted.set(False)
        self.toggle()
        self.text.configure(state='normal')
        self.text.delete('1.0','end')
        self.text.configure(state='disabled')
        self.status.set('Preparar revisión del grupo vigente. No se guarda ni envía.')

    def toggle(self):
        self.confirm_button.configure(state='normal' if self.frozen is not None and self.accepted.get() else 'disabled')

    def current_requests(self):
        work=self.app._require_work()
        ids=visible_selection(self.app)
        if work is not self.work or ids != self.ids:
            raise ContractError('TRABAJO_O_SELECCION_CAMBIADA_CERRAR')
        return tuple(ReviewRequest(r,self.origin.get(),self.destination.get(),self.state.get()) for r in ids)

    def prepare(self):
        self.invalidate()
        frozen=build_review(self.work,self.current_requests(),work_id=self.work_id,report_catalog=REPOSITORY_REPORT_CATALOG)
        self.text.configure(state='normal')
        for c in frozen.candidates:
            i=c.identity
            self.text.insert('end',f'{c.row_ref} | {c.origin} | Tribunal {i.tribunal_id} / Causa {i.causa_id} / Ingreso {i.ingreso_id}\n'
                             f'Destino: {c.destination} | Estado: {c.observation_state}\n{c.text}\n\n')
        self.text.configure(state='disabled')
        self.frozen=frozen
        self.status.set('Revisa cada operación. El Estado y el catálogo vivo todavía requieren comprobación.')

    def confirm(self):
        if self.frozen is None or not self.accepted.get():
            raise ContractError('CONFIRMACION_EXACTA_REQUERIDA')
        frozen=self.frozen
        ops=confirm_review(self.work,frozen,self.current_requests(),displayed_digest=frozen.digest,confirmed=True,
                           actor='revision-local-no-autenticada',stamp=datetime.now(timezone.utc).isoformat().replace('+00:00','Z'),
                           version=frozen.candidates[0].revision+1,report_catalog=REPOSITORY_REPORT_CATALOG)
        packet=PreviewBatch(ops,frozen.candidates[0].source_digest,frozen.candidates[0].revision)
        self.preview=PreviewBatch.decode(packet.encode())
        self.frozen=None
        self.accepted.set(False)
        self.toggle()
        self.status.set(f'{len(self.preview.operations)} operaciones verificadas en memoria. No exportadas; sin autorización de envío.')
        return self.preview


def open_work_preview(app):
    return WorkPreviewDialog(app)
