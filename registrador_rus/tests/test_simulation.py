from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

from registrador_rus.app.journal import SimulationJournal, JournalError
from registrador_rus.app.simulation import demo_operations, FakeClock, FakeRus, SimulationController


class SimulationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = TemporaryDirectory()
        self.path = Path(self.tmp.name) / 'demo.sqlite'
        self.clock = FakeClock()
        self.ops = demo_operations()
        self.fake = FakeRus(self.clock, self.ops)
        self.j = SimulationJournal(self.path, clock=self.clock)
        self.c = SimulationController(self.j, self.fake)

    def tearDown(self):
        self.j.close()
        self.tmp.cleanup()

    def approve(self, op):
        self.c.prepare(op)
        self.c.authorize(op, confirmed=True)

    def test_eight_operations_four_origins_and_destinations(self):
        for op in self.ops:
            self.approve(op)
            self.assertEqual(self.c.execute(op), 'COMPROBADA')
        self.assertEqual(self.fake.calls, 8)
        self.assertEqual(self.j.attempt_count(), 8)
        self.assertEqual(self.j.recover_unresolved(), ())

    def test_confirmation_and_changed_form(self):
        op = self.ops[0]
        self.c.prepare(op)
        with self.assertRaises(JournalError): self.c.authorize(op, confirmed=False)
        with self.assertRaises(JournalError): self.c.execute(op)
        self.c.authorize(op, confirmed=True)
        self.fake.prepared = None
        with self.assertRaises(JournalError): self.c.execute(op)
        self.assertEqual(self.fake.calls, 0)
        self.assertEqual(self.j.attempt_count(), 0)

    def test_crash_after_save_and_read_only(self):
        op = self.ops[0]
        self.approve(op)
        self.assertEqual(self.c.execute(op, 'AFTER_SAVE'), 'INCIERTA')
        self.j.close()
        self.j = SimulationJournal(self.path, clock=self.clock)
        self.c = SimulationController(self.j, self.fake)
        self.assertEqual(self.c.recover(op), 'COMPROBADA')
        self.assertEqual(self.fake.calls, 1)
        self.assertEqual(self.j.attempt_count(), 1)
        with self.assertRaises(JournalError): self.c.prepare(op)

    def test_no_row_blocks_next(self):
        op, other = self.ops[:2]
        self.approve(op)
        self.assertEqual(self.c.execute(op, 'NO_ROW'), 'NO_HALLADA')
        self.approve(other)
        with self.assertRaisesRegex(JournalError, 'INTENTO_SIN_RESOLVER'): self.c.execute(other)
        self.assertEqual(self.fake.calls, 1)
        self.assertEqual(self.c.recover(op), 'NO_HALLADA')

    def test_critical_and_pause(self):
        op = self.ops[0]
        self.approve(op)
        self.j.control_schedule('PAUSE')
        with self.assertRaisesRegex(JournalError, 'PAUSED'): self.c.execute(op)
        self.j.control_schedule('RESUME')
        self.assertEqual(self.c.execute(op, '429'), 'INCIERTA')
        self.assertEqual(self.j.schedule_eligibility().reason, 'CRITICAL')
        self.assertEqual(self.fake.calls, 1)

    def test_ambiguous_blocks_recovery(self):
        op = self.ops[0]
        self.approve(op)
        self.assertEqual(self.c.execute(op, 'AMBIGUOUS'), 'AMBIGUA')
        with self.assertRaisesRegex(JournalError, 'AMBIGUEDAD_REQUIERE_REVISION_HUMANA'): self.c.recover(op)
        self.assertEqual(self.fake.calls, 1)

    def test_partial_then_readback(self):
        op = self.ops[0]
        self.approve(op)
        self.assertEqual(self.c.execute(op, 'PARTIAL'), 'INCIERTA')
        self.assertEqual(self.c.recover(op), 'COMPROBADA')
        self.assertEqual(self.fake.calls, 1)

    def test_403_and_failure_before_save(self):
        op = self.ops[0]
        self.approve(op)
        self.assertEqual(self.c.execute(op, '403'), 'INCIERTA')
        self.assertEqual(self.j.schedule_eligibility().reason, 'CRITICAL')
        self.assertEqual(self.c.recover(op), 'NO_HALLADA')
        self.assertEqual(self.fake.calls, 1)

    def test_expired_session_blocks_before_attempt(self):
        op = self.ops[0]
        self.approve(op)
        self.fake.session_valid = False
        with self.assertRaises(JournalError): self.c.execute(op)
        self.assertEqual(self.fake.calls, 0)
        self.assertEqual(self.j.attempt_count(), 0)
