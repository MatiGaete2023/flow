from dataclasses import replace
from datetime import datetime, timedelta, timezone
from pathlib import Path
import sqlite3
from tempfile import TemporaryDirectory
import unittest

from registrador_rus.app.contracts import RegistroFlowV1
from registrador_rus.app.journal import JournalError, SimulationJournal
from registrador_rus.app.scheduler import ScheduleConfig
from test_contracts import sample
from readback_fixture import bind, snapshot

NOW = datetime(2026, 11, 24, 14, tzinfo=timezone.utc)


class StoreTests(unittest.TestCase):
    def setUp(self):
        self.tmp = TemporaryDirectory(); self.path = Path(self.tmp.name)/'sim.sqlite'
        self.now = NOW
        self.j = SimulationJournal(self.path,clock=lambda:self.now)
        self.op = RegistroFlowV1.from_mapping(sample())

    def tearDown(self):
        self.j.close();self.tmp.cleanup()

    def ready(self,op):
        self.j.freeze(op);self.j.mark_ready(op);self.j.authorize_simulation(op,human_actor='demo',confirmed=True)

    def row(self,op,key='1'):
        return dict(entry_id=key,texto=op.approved_text,autor='demo',tipo='Administrativa' if not op.cc else 'Al Tribunal',
                    etapa='Etapa ficticia',fecha=self.now.replace(tzinfo=None).isoformat(),origen='centro')

    def test_mandatory_recent_baseline(self):
        self.ready(self.op)
        with self.assertRaisesRegex(JournalError,'BASELINE_REQUERIDA'):
            self.j.reserve_simulated_attempt(self.op)
        bind(self.j,self.op,self.now)
        self.now+=timedelta(minutes=6)
        with self.assertRaisesRegex(JournalError,'BASE_NO_RECIENTE'):
            self.j.reserve_simulated_attempt(self.op)
        self.assertEqual(self.j.attempt_count(),0)
        bind(self.j,self.op,self.now)
        self.assertEqual(self.j.reserve_simulated_attempt(self.op),'ENVIANDO')

    def test_close_quota_and_reopen(self):
        self.j.configure_schedule(ScheduleConfig(True,1,mode='fixed',fixed_seconds=7))
        self.ready(self.op);bind(self.j,self.op,self.now)
        self.j.reserve_simulated_attempt(self.op)
        self.now+=timedelta(seconds=1)
        self.j.close();self.j=SimulationJournal(self.path,clock=lambda:self.now)
        self.assertEqual(self.j.resolve_simulated(self.op,snapshot(self.op,self.now,rows=[self.row(self.op)])),'COMPROBADA')
        self.assertEqual(self.j.db.execute('SELECT simulated FROM readback_receipts').fetchone()[0],1)
        self.assertEqual(self.j.recover_unresolved(),())
        self.assertEqual(self.j.schedule_eligibility().reason,'ESPERAR_CUPO')
        other=replace(self.op,operation_id='otro',destination='AL_TRIBUNAL')
        self.ready(other);bind(self.j,other,self.now)
        with self.assertRaisesRegex(JournalError,'ESPERAR_CUPO'):
            self.j.reserve_simulated_attempt(other)
        self.now=NOW+timedelta(hours=1);bind(self.j,other,self.now)
        self.assertEqual(self.j.reserve_simulated_attempt(other),'ENVIANDO')
        self.assertEqual(self.j.attempt_count(),2)

    def test_uncertain_and_ambiguous_block_then_read_only(self):
        self.ready(self.op);bind(self.j,self.op,self.now,require_excel=True)
        self.j.reserve_simulated_attempt(self.op);self.now+=timedelta(seconds=1)
        self.assertEqual(self.j.resolve_simulated(self.op,snapshot(self.op,self.now,partial=True)),'INCIERTA')
        self.assertEqual(self.j.resolve_simulated(self.op,snapshot(self.op,self.now)),'NO_HALLADA')
        self.assertEqual(self.j.recover_unresolved(),(self.op.operation_id,))
        with self.assertRaisesRegex(JournalError,'INTENTO_PREVIO'):
            self.j.reserve_simulated_attempt(self.op)
        self.now+=timedelta(seconds=1)
        self.assertEqual(self.j.resolve_simulated(self.op,snapshot(self.op,self.now,rows=[self.row(self.op)])),'PENDIENTE_EXCEL')
        self.assertEqual(self.j.attempt_count(),1)
        with self.assertRaisesRegex(JournalError,'ESTADO_NO_RECUPERABLE'):
            self.j.resolve_simulated(self.op,snapshot(self.op,self.now))

    def test_ambiguity_requires_review_and_invalid_baseline_clears_old(self):
        self.ready(self.op);bind(self.j,self.op,self.now)
        self.assertEqual(self.j.bind_baseline(self.op,{},author='demo',stage='Etapa ficticia',stage_code='1',server_day=self.now.date()),'INCIERTA')
        with self.assertRaisesRegex(JournalError,'BASELINE_REQUERIDA'):
            self.j.reserve_simulated_attempt(self.op)
        bind(self.j,self.op,self.now);self.j.reserve_simulated_attempt(self.op)
        self.now+=timedelta(seconds=1)
        self.assertEqual(self.j.resolve_simulated(self.op,snapshot(self.op,self.now,rows=[self.row(self.op),self.row(self.op,'2')])),'AMBIGUA')
        self.now+=timedelta(seconds=1)
        with self.assertRaisesRegex(JournalError,'AMBIGUEDAD_REQUIERE_REVISION_HUMANA'):
            self.j.resolve_simulated(self.op,snapshot(self.op,self.now,rows=[self.row(self.op)]))
        self.assertEqual(self.j.attempt_count(),1)
        self.assertEqual(self.j.recover_unresolved(),(self.op.operation_id,))

    def test_previous_prevents_reserve(self):
        self.ready(self.op)
        result=self.j.bind_baseline(self.op,snapshot(self.op,self.now,rows=[self.row(self.op)]),author='demo',stage='Etapa ficticia',stage_code='1',server_day=self.now.date())
        self.assertEqual(result,'REUTILIZABLE')
        self.assertEqual(self.j.state(self.op.operation_id),'REVISAR_PREVIAS')
        with self.assertRaisesRegex(JournalError,'ESTADO_NO_LISTO'):
            self.j.reserve_simulated_attempt(self.op)
        self.assertEqual(self.j.attempt_count(),0)

    def test_legacy_schema_preserves_attempt(self):
        self.ready(self.op);bind(self.j,self.op,self.now);self.j.reserve_simulated_attempt(self.op)
        self.j.close()
        db=sqlite3.connect(self.path,isolation_level=None)
        db.execute('PRAGMA foreign_keys=OFF');db.execute('BEGIN IMMEDIATE')
        db.execute("CREATE TABLE old_ops(operation_id TEXT PRIMARY KEY,payload TEXT NOT NULL,digest TEXT NOT NULL,identity_key TEXT NOT NULL,intent_key TEXT NOT NULL,state TEXT NOT NULL CHECK(state IN ('PREPARADA','LISTA','ENVIANDO','INCIERTA','OMITIDA')))")
        db.execute('INSERT INTO old_ops SELECT * FROM operations');db.execute('DROP TABLE operations');db.execute('ALTER TABLE old_ops RENAME TO operations');db.execute('COMMIT');db.close()
        self.j=SimulationJournal(self.path,clock=lambda:self.now)
        self.assertEqual(self.j.attempt_count(),1);self.assertEqual(self.j.recover_unresolved(),(self.op.operation_id,))
        self.assertIsNone(self.j.db.execute('PRAGMA foreign_key_check').fetchone())
        self.now+=timedelta(seconds=1)
        self.assertEqual(self.j.resolve_simulated(self.op,snapshot(self.op,self.now,rows=[self.row(self.op)])),'COMPROBADA')
