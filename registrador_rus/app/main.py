"""Interfaz de demostración: solo datos generados, no importación ni RUS real."""
from pathlib import Path
from tempfile import TemporaryDirectory
import tkinter as tk
from tkinter import ttk, messagebox

from .bridge import LocalBridge
from .journal import SimulationJournal, JournalError
from .scheduler import ScheduleConfig, ScheduleError
from .simulation import demo_operations, FakeClock, FakeRus, SimulationController, FAULTS


class DemoApp:
    def __init__(self, root):
        self.root = root
        root.title('Registrador Flow — SIMULACIÓN EXCLUSIVAMENTE FICTICIA')
        root.minsize(850, 580)
        self.temp = TemporaryDirectory(prefix='registrador-demo-')
        self.clock = FakeClock()
        self.ops = {op.operation_id: op for op in demo_operations()}
        self.fake = FakeRus(self.clock, self.ops.values())
        self.journal = SimulationJournal(Path(self.temp.name) / 'demo.sqlite', clock=self.clock)
        self.controller = SimulationController(self.journal, self.fake)
        self.bridge = None
        self.confirmed = set()
        self.closed = False
        self.status = tk.StringVar(value='Sin selección. No se conecta a RUS.')
        self.browser_status = tk.StringVar(value='Puente de demostración desconectado')
        self.fault = tk.StringVar(value='OK')
        self.limit = tk.StringVar(value='')
        self.mode = tk.StringVar(value='none')
        self.interval = tk.StringVar(value='0')
        self.maximum = tk.StringVar(value='0')
        self.origin = tk.StringVar(value='')
        self.credentials = tk.StringVar(value='')
        body = ttk.Frame(root, padding=12); body.pack(fill='both', expand=True)
        ttk.Label(body, text='Solo expedientes ficticios · sin escritura real · diario temporal', font=('TkDefaultFont', 12, 'bold')).pack(anchor='w')
        cols = ('origen', 'ingreso', 'destino', 'estado')
        self.table = ttk.Treeview(body, columns=cols, show='headings', selectmode='browse', height=8)
        for key, label, width in zip(cols, ('Origen', 'Ingreso ficticio', 'Destino', 'Estado simulado'), (220, 140, 140, 180)):
            self.table.heading(key, text=label); self.table.column(key, width=width)
        self.table.pack(fill='x', pady=6)
        self.table.bind('<<TreeviewSelect>>', lambda _: self.preview())
        self.detail = tk.Text(body, height=5, wrap='word', state='disabled'); self.detail.pack(fill='x')
        controls = ttk.Frame(body); controls.pack(fill='x', pady=6)
        for label, action in [('Preparar', self.prepare), ('Autorizar simulación', self.authorize),
                              ('Simular intento', self.execute), ('Recuperar por lectura', self.recover)]:
            ttk.Button(controls, text=label, command=lambda f=action: self.run(f)).pack(side='left', padx=2)
        ttk.Combobox(controls, textvariable=self.fault, values=FAULTS, state='readonly', width=14).pack(side='left', padx=5)
        schedule = ttk.LabelFrame(body, text='Ritmo simulado — reloj virtual'); schedule.pack(fill='x', pady=3)
        for label, variable, width in [('Cupo/h (vacío = sin límite)', self.limit, 6), ('Mínimo/fijo s', self.interval, 5), ('Máximo s', self.maximum, 5)]:
            ttk.Label(schedule, text=label).pack(side='left', padx=2); ttk.Entry(schedule, textvariable=variable, width=width).pack(side='left')
        ttk.Combobox(schedule, textvariable=self.mode, values=('none','fixed','bounded_variable'), state='readonly', width=16).pack(side='left')
        ttk.Button(schedule, text='Aplicar', command=lambda: self.run(self.configure)).pack(side='left')
        actions = ttk.Frame(body); actions.pack(fill='x', pady=3)
        for label, command in [('Pausar','PAUSE'),('Reanudar','RESUME'),('Detener','STOP')]:
            ttk.Button(actions,text=label,command=lambda c=command:self.run(lambda:self.journal.control_schedule(c))).pack(side='left',padx=2)
        ttk.Button(actions,text='Avanzar reloj ficticio 60 s',command=lambda:self.run(lambda:self.clock.tick(60))).pack(side='left',padx=2)
        bridge = ttk.LabelFrame(body, text='Visualización Chrome ficticia (opcional; no autoriza ni guarda)'); bridge.pack(fill='x', pady=4)
        ttk.Entry(bridge, textvariable=self.origin, width=42).pack(side='left')
        ttk.Button(bridge, text='Abrir puente', command=lambda:self.run(self.connect)).pack(side='left')
        ttk.Button(bridge, text='Preparar en demo Chrome', command=lambda:self.run(self.send)).pack(side='left')
        ttk.Entry(body, textvariable=self.credentials, state='readonly').pack(fill='x')
        ttk.Label(body, textvariable=self.browser_status).pack(anchor='w')
        ttk.Label(body, textvariable=self.status, wraplength=800).pack(anchor='w', pady=4)
        root.protocol('WM_DELETE_WINDOW', self.close)
        self.refresh(); self.poll_result()

    def selected(self):
        ids = self.table.selection()
        if len(ids) != 1:
            raise JournalError('SELECCION_UNICA_REQUERIDA')
        return self.ops[ids[0]]

    def preview(self):
        if not self.table.selection(): return
        op = self.selected(); i = op.remote_identity
        text = f'{op.operation_id} | Tribunal {i.tribunal_id} / Causa {i.causa_id} / Ingreso {i.ingreso_id}\n{op.origin} | {op.destination} | Estado: {op.observation_state} | Casilla: {"MARCADA" if op.cc else "DESMARCADA"}\n{op.approved_text}'
        self.detail.configure(state='normal'); self.detail.delete('1.0','end'); self.detail.insert('1.0',text); self.detail.configure(state='disabled')

    def refresh(self):
        for op in self.ops.values():
            try: state = self.journal.state(op.operation_id)
            except JournalError: state = 'SIN PREPARAR'
            if state == 'LISTA' and op.operation_id in self.confirmed: state = 'LISTA / AUTORIZADA SIM.'
            values = (op.origin,op.remote_identity.ingreso_id,op.destination,state)
            if self.table.exists(op.operation_id): self.table.item(op.operation_id,values=values)
            else: self.table.insert('', 'end', iid=op.operation_id, values=values)
        eligibility = self.journal.schedule_eligibility()
        self.status.set(f'Intentos ficticios: {self.journal.attempt_count()} | Reloj: {self.clock().isoformat()} | {eligibility.reason}')

    def run(self, action):
        try: action()
        except (ValueError, RuntimeError) as exc: messagebox.showwarning('Simulación detenida', str(exc), parent=self.root)
        finally: self.refresh()

    def prepare(self): self.controller.prepare(self.selected())

    def authorize(self):
        op = self.selected()
        self.controller.fake.verify_prepared(op)
        if messagebox.askyesno('Confirmación exclusivamente ficticia', f'Autorizar solo {op.operation_id}, ingreso {op.remote_identity.ingreso_id}, {op.destination}?\n\n{op.approved_text}', parent=self.root):
            self.controller.authorize(op,confirmed=True); self.confirmed.add(op.operation_id)

    def execute(self):
        op = self.selected(); self.controller.execute(op, self.fault.get())

    def recover(self): self.controller.recover(self.selected())

    def configure(self):
        limit = int(self.limit.get()) if self.limit.get().strip() else None
        mode = self.mode.get()
        config = ScheduleConfig(limit is not None, limit, mode,
            int(self.interval.get()) if mode=='fixed' else None,
            int(self.interval.get()) if mode=='bounded_variable' else None,
            int(self.maximum.get()) if mode=='bounded_variable' else None)
        self.journal.configure_schedule(config)

    def connect(self):
        if self.bridge is not None: raise JournalError('PUENTE_YA_ABIERTO')
        self.bridge = LocalBridge(self.origin.get().strip())
        self.credentials.set(f'{self.bridge.server.server_port} | {self.bridge.mailbox.session_id} | {self.bridge.token}')
        self.browser_status.set('Copiar puerto | sesión | token a la extensión ficticia. No compartir.')

    def send(self):
        if self.bridge is None: raise JournalError('PUENTE_NO_ABIERTO')
        op = self.selected(); self.bridge.mailbox.submit('prepare',frozen=op,current=op)
        self.browser_status.set('Orden ficticia en tránsito; recepción no equivale a autorización.')

    def poll_result(self):
        if self.closed: return
        if self.bridge:
            result = self.bridge.mailbox.take_result()
            if result: self.browser_status.set('Recepción ficticia: ' + result['status'] + ' (sin guardado)')
        self.root.after(300,self.poll_result)

    def close(self):
        if self.closed: return
        self.closed = True
        if self.bridge: self.bridge.close()
        self.journal.close(); self.temp.cleanup(); self.root.destroy()


def main():
    root = tk.Tk(); DemoApp(root); root.mainloop()


if __name__ == '__main__': main()
