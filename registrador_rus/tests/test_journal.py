from dataclasses import replace
from pathlib import Path
from tempfile import TemporaryDirectory
import threading
import unittest

from registrador_rus.app.contracts import RegistroFlowV1
from registrador_rus.app.journal import JournalError, SimulationJournal
from test_contracts import sample
from readback_fixture import bind


class JournalTests(unittest.TestCase):
    def setUp(self):
        self.tmp = TemporaryDirectory()
        self.path = Path(self.tmp.name) / 'demo.sqlite'
        self.j = SimulationJournal(self.path)
        self.op = RegistroFlowV1.from_mapping(sample())

    def tearDown(self):
        self.j.close()
        self.tmp.cleanup()

    def ready(self, op):
        self.j.freeze(op)
        self.j.mark_ready(op)
        self.j.authorize_simulation(op, human_actor='demo', confirmed=True)
        bind(self.j, op)

    def test_frozen_intent_and_approval(self):
        self.assertEqual(self.j.freeze(self.op), 'PREPARADA')
        self.assertEqual(self.j.freeze(self.op), 'PREPARADA')
        with self.assertRaisesRegex(JournalError, 'ID_REUTILIZADO'):
            self.j.freeze(replace(self.op, destination='AL_TRIBUNAL'))
        with self.assertRaisesRegex(JournalError, 'INTENCION_EQUIVALENTE'):
            self.j.freeze(replace(self.op, operation_id='otro'))
        with self.assertRaisesRegex(JournalError, 'APROBACION_CAMBIADA'):
            self.j.mark_ready(replace(self.op, approval_version=2))

    def test_human_confirmation_and_one_attempt(self):
        self.j.freeze(self.op)
        self.j.mark_ready(self.op)
        with self.assertRaisesRegex(JournalError, 'AUTORIZACION_REQUERIDA'):
            self.j.reserve_simulated_attempt(self.op)
        with self.assertRaisesRegex(JournalError, 'CONFIRMACION_HUMANA'):
            self.j.authorize_simulation(self.op, human_actor='demo', confirmed=False)
        self.j.authorize_simulation(self.op, human_actor='demo', confirmed=True)
        bind(self.j, self.op)
        self.assertEqual(self.j.reserve_simulated_attempt(self.op), 'ENVIANDO')
        with self.assertRaisesRegex(JournalError, 'INTENTO_PREVIO'):
            self.j.reserve_simulated_attempt(self.op)
        self.assertEqual(self.j.attempt_count(), 1)

    def test_reopen_after_reservation(self):
        self.ready(self.op)
        self.j.reserve_simulated_attempt(self.op)
        self.j.close()
        self.j = SimulationJournal(self.path)
        self.assertEqual(self.j.recover_unresolved(), (self.op.operation_id,))
        self.j.mark_uncertain(self.op.operation_id)
        self.assertEqual(self.j.state(self.op.operation_id), 'INCIERTA')
        with self.assertRaisesRegex(JournalError, 'OMISION_NO_PERMITIDA'):
            self.j.omit(self.op.operation_id)
        self.assertEqual(self.j.recover_unresolved(), (self.op.operation_id,))
        self.assertEqual(self.j.attempt_count(), 1)

    def test_atomic_rollback_before_attempt_commit(self):
        self.ready(self.op)
        self.j.db.execute("CREATE TRIGGER fail_audit BEFORE INSERT ON audit WHEN NEW.action='RESERVA_SIMULADA' BEGIN SELECT RAISE(ABORT,'demo'); END")
        import sqlite3
        with self.assertRaises(sqlite3.IntegrityError):
            self.j.reserve_simulated_attempt(self.op)
        self.assertEqual(self.j.state(self.op.operation_id), 'LISTA')
        self.assertEqual(self.j.attempt_count(), 0)
        self.j.db.execute('DROP TRIGGER fail_audit')
        self.assertEqual(self.j.reserve_simulated_attempt(self.op), 'ENVIANDO')

    def test_two_connections_cannot_reserve_in_parallel(self):
        other = replace(self.op, operation_id='otro', destination='AL_TRIBUNAL')
        self.ready(self.op)
        self.ready(other)
        barrier = threading.Barrier(2)
        results = []
        def run(op):
            j = SimulationJournal(self.path)
            try:
                barrier.wait(timeout=5)
                results.append(j.reserve_simulated_attempt(op))
            except JournalError as e:
                results.append(str(e))
            finally:
                j.close()
        threads = [threading.Thread(target=run, args=(op,)) for op in (self.op, other)]
        for t in threads: t.start()
        for t in threads: t.join(10)
        self.assertCountEqual(results, ['ENVIANDO', 'INTENTO_SIN_RESOLVER'])
        self.assertEqual(self.j.attempt_count(), 1)

    def test_omit_before_attempt(self):
        self.ready(self.op)
        self.j.omit(self.op.operation_id)
        self.assertEqual(self.j.state(self.op.operation_id), 'OMITIDA')
        with self.assertRaisesRegex(JournalError, 'ESTADO_NO_LISTO'):
            self.j.reserve_simulated_attempt(self.op)
        self.assertEqual(self.j.attempt_count(), 0)
