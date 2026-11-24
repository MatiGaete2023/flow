"""P7/P8: escenario cerrado y ficticio, sin acceso a RUS ni escritor remoto."""
from datetime import datetime, timezone, timedelta
from hashlib import sha256
from itertools import cycle

from .contracts import RegistroFlowV1, RemoteIdentity, Source
from .journal import JournalError, SimulationJournal
from .readback import METHOD, TYPES

ORIGINS = ('ESPERA', 'CUMPLIMIENTO', 'INFORMES_POR_VENCER', 'INFORMES_VENCIDOS')
FAULTS = ('OK', '403', '429', 'NO_ROW', 'AFTER_SAVE', 'AMBIGUOUS', 'PARTIAL', 'BEFORE_SAVE')


def demo_operations(count=8):
    if type(count) is not int or not 1 <= count <= 400:
        raise ValueError('CANTIDAD_FICTICIA_INVALIDA')
    results = []
    for index, origin in zip(range(count), cycle(ORIGINS)):
        text = f'Constancia ficticia {index + 1}. Sin expediente real.'
        results.append(RegistroFlowV1('RegistroFlowV1', f'demo-{index+1}', Source('DEMO', 'lote-demo', str(index+1)),
            RemoteIdentity('SITFA', '900000', str(900000 + index // 2), str(910000 + index)), origin,
            'AL_TRIBUNAL' if index % 2 else 'ADMINISTRATIVA', 'demo', text, sha256(text.encode()).hexdigest(),
            'actor-ficticio', '2026-11-23T00:00:00Z', 1))
    return tuple(results)


class FakeClock:
    def __init__(self):
        self.now = datetime(2026, 11, 24, 12, tzinfo=timezone.utc)

    def __call__(self):
        return self.now

    def tick(self, seconds=1):
        self.now += timedelta(seconds=seconds)


class FakeRus:
    """No URL, red, navegador ni credenciales. Almacén ficticio para probar fallos."""
    def __init__(self, clock, operations):
        self.clock = clock
        self.operations = {op.operation_id: op for op in operations}
        self.rows = {}
        self.calls = 0
        self.session_valid = True
        self.prepared = None

    def check(self, op):
        if not self.session_valid or self.operations.get(op.operation_id) != op:
            raise JournalError('SESION_O_IDENTIDAD_FICTICIA_INVALIDA')

    def prepare(self, op):
        self.check(op)
        self.prepared = (op, op.approved_text, op.tribunal_checked, op.observation_state)
        return self.prepared

    def verify_prepared(self, op):
        self.check(op)
        if self.prepared != (op, op.approved_text, op.tribunal_checked, op.observation_state):
            raise JournalError('FORMULARIO_FICTICIO_CAMBIADO')

    def snapshot(self, op, *, partial=False):
        self.check(op)
        i = op.remote_identity
        ident = dict(tribunal_codigo=i.tribunal_id, causa_id=i.causa_id, ingreso_id=i.ingreso_id)
        rows = self.rows.get(i, [])
        return dict(ident, captured_at=self.clock().isoformat(), stage_code='1',
            entries=[dict(ident, **row) for row in rows],
            row_metadata=[dict(entry_id=row['entry_id'], state=row['state']) for row in rows],
            alcance_metodo=METHOD, alcance='PARCIAL' if partial else ('COMPLETA' if rows else 'VACIA_COMPROBADA'))

    def _append(self, op):
        rows = self.rows.setdefault(op.remote_identity, [])
        rows.append(dict(entry_id=f'demo-{len(rows)+1}', texto=op.approved_text, autor='actor-ficticio',
            tipo=TYPES[op.destination], etapa='Etapa ficticia', fecha=self.clock().replace(tzinfo=None).isoformat(),
            origen='centro', state=op.observation_state))

    def simulate_save(self, op, fault):
        self.verify_prepared(op)
        if fault not in FAULTS:
            raise ValueError('FALLO_FICTICIO_INVALIDO')
        self.calls += 1
        self.clock.tick()
        self.prepared = None
        if fault in ('403', '429', 'BEFORE_SAVE'):
            raise JournalError(fault)
        if fault != 'NO_ROW':
            self._append(op)
        if fault == 'AMBIGUOUS':
            self._append(op)
        if fault == 'AFTER_SAVE':
            raise JournalError('CORTE_FICTICIO')
        return self.snapshot(op, partial=fault == 'PARTIAL')


class SimulationController:
    """Las confirmaciones son simuladas, no equivalen a autorización institucional."""
    def __init__(self, journal, fake):
        if type(journal) is not SimulationJournal or type(fake) is not FakeRus:
            raise TypeError('SOLO_SIMULACION')
        self.journal, self.fake = journal, fake

    def prepare(self, op):
        self.fake.prepare(op)
        state = self.journal.freeze(op)
        if state == 'PREPARADA':
            self.journal.mark_ready(op)
        elif state != 'LISTA':
            raise JournalError('ESTADO_NO_LISTO')
        return 'LISTA'

    def authorize(self, op, *, confirmed):
        self.fake.verify_prepared(op)
        self.journal.authorize_simulation(op, human_actor='actor-ficticio', confirmed=confirmed)

    def execute(self, op, fault='OK', *, require_excel=False):
        # Nunca aceptar otro adaptador, URL o callback de escritura.
        if fault not in FAULTS:
            raise ValueError('FALLO_FICTICIO_INVALIDO')
        self.fake.verify_prepared(op)
        result = self.journal.bind_baseline(op, self.fake.snapshot(op), author='actor-ficticio',
            stage='Etapa ficticia', stage_code='1', server_day=self.fake.clock().date(), require_excel=require_excel)
        if result != 'SIN_PREVIA':
            return result
        self.fake.verify_prepared(op)
        self.journal.reserve_simulated_attempt(op)
        try:
            raw = self.fake.simulate_save(op, fault)
        except Exception:
            self.journal.mark_uncertain(op.operation_id)
            if fault in ('403', '429'):
                self.journal.control_schedule('CRITICAL')
            return 'INCIERTA'
        return self.journal.resolve_simulated(op, raw)

    def recover(self, op):
        # La recuperación no llama a simulate_save ni reserva otro intento.
        self.fake.clock.tick()
        return self.journal.resolve_simulated(op, self.fake.snapshot(op))
