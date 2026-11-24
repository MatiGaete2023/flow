from dataclasses import replace
from datetime import datetime, timezone
from hashlib import sha256
from pathlib import Path
import sqlite3
from tempfile import TemporaryDirectory
import unittest
from registrador_rus.app.contracts import ContractError, RegistroFlowV1, Source, RemoteIdentity
from registrador_rus.app.batch_store import SimulationBatchStore, selection_digest

NOW = datetime(2026, 11, 24, 15, tzinfo=timezone.utc)


def ops():
    text = 'Ficticia, sin expediente real.'
    first = RegistroFlowV1('RegistroFlowV1','demo-1',Source('DEMO','demo','1'),RemoteIdentity('SITFA','90','91','92'),
                           'ESPERA','ADMINISTRATIVA','demo',text,sha256(text.encode()).hexdigest(),'demo','2026-11-24T12:00:00Z',1)
    return (first, replace(first, operation_id='demo-2', remote_identity=RemoteIdentity('SITFA','90','91','93'), destination='AL_TRIBUNAL'))


class BatchStoreTests(unittest.TestCase):
    def setUp(self):
        self.tmp = TemporaryDirectory(); self.path=Path(self.tmp.name)/'demo.sqlite'
        self.db=sqlite3.connect(self.path, isolation_level=None); self.store=SimulationBatchStore(self.db); self.store.initialize()
        self.ops=ops(); self.digest=selection_digest(self.ops)

    def tearDown(self):
        self.db.close(); self.tmp.cleanup()

    def record(self, **changes):
        kw=dict(displayed_digest=self.digest,actor='demo',confirmed=True,now=NOW)
        return self.store.record('revision-1',self.ops,**(kw|changes))

    def test_persistent_exact_confirmation(self):
        self.db.execute('BEGIN IMMEDIATE'); self.record(); self.db.commit()
        self.db.close(); self.db=sqlite3.connect(self.path,isolation_level=None); self.store=SimulationBatchStore(self.db);self.store.initialize()
        self.assertTrue(self.store.require_current('revision-1',self.ops))
        for modified in (self.ops[::-1], self.ops[:1], (replace(self.ops[0],destination='AL_TRIBUNAL'), self.ops[1])):
            with self.assertRaises(ContractError): self.store.require_current('revision-1', modified)
        self.assertEqual(self.db.execute('SELECT simulated,count FROM simulated_batch_reviews').fetchone(),(1,2))

    def test_no_implicit_confirmation_or_overwrite(self):
        with self.assertRaises(ContractError): self.record()
        self.db.execute('BEGIN IMMEDIATE')
        for change in (dict(confirmed=False),dict(displayed_digest='wrong'),dict(actor=''),dict(now=NOW.replace(tzinfo=None))):
            with self.assertRaises(ContractError): self.record(**change)
        self.record()
        with self.assertRaises(ContractError): self.record()
        self.db.commit()
        self.assertEqual(self.db.execute('SELECT COUNT(*) FROM simulated_batch_reviews').fetchone()[0],1)

    def test_rollback_and_privacy(self):
        self.db.execute('BEGIN IMMEDIATE'); self.record(); self.db.rollback()
        with self.assertRaises(ContractError): self.store.require_current('revision-1',self.ops)
        self.db.execute('BEGIN IMMEDIATE'); self.record(); self.db.commit()
        data=str(self.db.execute('SELECT * FROM simulated_batch_reviews').fetchall())
        for value in (self.ops[0].approved_text,self.ops[0].operation_id,self.ops[1].operation_id): self.assertNotIn(value,data)
        with self.assertRaises(ContractError): selection_digest((self.ops[0],self.ops[0]))

    def test_does_not_create_attempts_or_authorizations(self):
        self.db.execute('BEGIN IMMEDIATE');self.record();self.db.commit()
        names={x[0] for x in self.db.execute("SELECT name FROM sqlite_master WHERE type='table'")}
        self.assertEqual(names,{'simulated_batch_reviews'})
