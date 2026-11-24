"""DPAPI nativa: diario, WAL y SHM de datos inventados; no RUS."""
from dataclasses import replace
from pathlib import Path
from tempfile import TemporaryDirectory
import os
import sqlite3
import unittest
from unittest.mock import patch

from registrador_rus.app.journal import SimulationJournal, JournalError
from registrador_rus.app.journal_protection import StorageProtectionError
from registrador_rus.app.simulation import FakeClock, FakeRus, demo_operations

PRIOR='Texto previo ficticio SECRET_FIXTURE_038292'
ACTOR='Actor ficticio SECRET_ACTOR_038292'
ENTRY='entry-fixture-SECRET_038292'


class ProtectedJournalTests(unittest.TestCase):
    def test_platform_and_mode_gate(self):
        with TemporaryDirectory() as tmp:
            path=Path(tmp)/'demo.protected.sqlite'
            with patch('registrador_rus.app.protected_demo_transfer.os.name','posix'):
                with self.assertRaisesRegex(StorageProtectionError,'DPAPI_DIARIO_NO_DISPONIBLE'):
                    SimulationJournal(path,protected_demo=True)
            self.assertFalse(path.exists())
            with self.assertRaisesRegex(StorageProtectionError,'EXTENSION_DIARIO_INCOMPATIBLE'):
                SimulationJournal(path)
            self.assertFalse(path.exists())


@unittest.skipUnless(os.name=='nt','DPAPI nativa solo Windows')
class NativeProtectedJournalTests(unittest.TestCase):
    def setUp(self):
        self.tmp=TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.path=Path(self.tmp.name)/'demo.protected.sqlite'
        self.clock=FakeClock()
        self.ops=demo_operations(2)
        self.op=self.ops[0]
        self.fake=FakeRus(self.clock,self.ops)
        self.j=SimulationJournal(self.path,clock=self.clock,protected_demo=True)
        self.addCleanup(lambda:self.j.close())

    def ready(self,op,prior=False):
        self.j.freeze(op);self.j.mark_ready(op)
        self.j.authorize_simulation(op,human_actor=ACTOR,confirmed=True)
        if prior:
            self.fake._append(op)
            self.fake.rows[op.remote_identity][0].update(texto=PRIOR,autor=ACTOR,entry_id=ENTRY)
        self.assertEqual(self.j.bind_baseline(op,self.fake.snapshot(op),author='actor-ficticio',
            stage='Etapa ficticia',stage_code='1',server_day=self.clock().date()),'SIN_PREVIA')

    def scan(self,require_wal=False):
        files=list(Path(self.tmp.name).glob('demo.protected.sqlite*'))
        if require_wal:
            self.assertTrue(Path(str(self.path)+'-wal').is_file())
            self.assertTrue(Path(str(self.path)+'-shm').is_file())
        for file in files:
            raw=file.read_bytes()
            for text in (self.op.approved_text,PRIOR,ACTOR,ENTRY,'actor-ficticio','910000'):
                for encoding in ('utf-8','utf-16-le','utf-16-be'):
                    self.assertNotIn(text.encode(encoding),raw,file.name)

    def test_wal_and_reopen_no_resend(self):
        self.ready(self.op,prior=True)
        self.assertEqual(self.j.db.execute('SELECT typeof(payload) FROM operations').fetchone()[0],'blob')
        self.assertEqual(self.j.db.execute('SELECT typeof(evidence) FROM readback_baselines').fetchone()[0],'blob')
        self.assertEqual(self.j.db.execute('SELECT typeof(human_actor) FROM authorizations').fetchone()[0],'blob')
        self.assertEqual(self.j.reserve_simulated_attempt(self.op),'ENVIANDO')
        self.scan(require_wal=True)
        self.j.close()
        self.j=SimulationJournal(self.path,clock=self.clock,protected_demo=True)
        self.assertEqual(self.j.recover_unresolved(),(self.op.operation_id,))
        with self.assertRaisesRegex(JournalError,'INTENTO_PREVIO'):
            self.j.reserve_simulated_attempt(self.op)
        self.clock.tick()
        self.fake._append(self.op)
        self.fake.rows[self.op.remote_identity][-1]['entry_id']='receipt-SECRET_038292'
        self.assertEqual(self.j.resolve_simulated(self.op,self.fake.snapshot(self.op)),'COMPROBADA')
        key=self.j.db.execute('SELECT entry_id FROM readback_receipts').fetchone()[0]
        self.assertRegex(key,r'^[0-9a-f]{64}$')
        self.assertEqual(self.j.attempt_count(),1)
        self.scan(require_wal=True)
        self.j.close()
        self.scan()
        self.j=SimulationJournal(self.path,clock=self.clock,protected_demo=True)
        self.assertEqual(self.j.state(self.op.operation_id),'COMPROBADA')

    def test_plaintext_and_cross_operation_rejected(self):
        self.ready(self.op);self.ready(self.ops[1])
        value=self.j.db.execute('SELECT evidence FROM readback_baselines WHERE operation_id=?',(self.ops[1].operation_id,)).fetchone()[0]
        self.j.db.execute('UPDATE readback_baselines SET evidence=? WHERE operation_id=?',(value,self.op.operation_id))
        with self.assertRaisesRegex(StorageProtectionError,'DESCIFRADO_DIARIO_FALLIDO'):
            self.j.reserve_simulated_attempt(self.op)
        self.assertEqual(self.j.attempt_count(),0)
        self.j.db.execute('UPDATE readback_baselines SET evidence=? WHERE operation_id=?',('not encrypted',self.op.operation_id))
        with self.assertRaisesRegex(StorageProtectionError,'CAMPO_DIARIO_NO_CIFRADO'):
            self.j.reserve_simulated_attempt(self.op)
        self.assertEqual(self.j.state(self.op.operation_id),'LISTA')

    def test_corrupt_payload_and_actor(self):
        self.ready(self.op)
        original=self.j.db.execute('SELECT payload FROM operations').fetchone()[0]
        self.j.db.execute('UPDATE operations SET payload=?',(original[:-1],))
        with self.assertRaises(StorageProtectionError):self.j.reserve_simulated_attempt(self.op)
        self.j.db.execute('UPDATE operations SET payload=?',(original,))
        self.j.db.execute('UPDATE authorizations SET human_actor=?',(b'bad',))
        with self.assertRaises(StorageProtectionError):self.j.reserve_simulated_attempt(self.op)
        self.assertEqual(self.j.attempt_count(),0)

    def test_mode_mismatch_and_no_migration(self):
        self.j.close()
        renamed=self.path.with_name('renamed.sqlite')
        self.path.rename(renamed)
        with self.assertRaisesRegex(StorageProtectionError,'DIARIO_PROTEGIDO_REQUIERE_MODO_EXPLICITO'):
            SimulationJournal(renamed)
        old=self.path.with_name('old.sqlite')
        oldj=SimulationJournal(old);oldj.freeze(self.op);oldj.close()
        legacy=self.path.with_name('legacy.protected.sqlite')
        old.rename(legacy)
        with self.assertRaisesRegex(StorageProtectionError,'MIGRACION_DIARIO_NO_PERMITIDA'):
            SimulationJournal(legacy,protected_demo=True)
        self.j=SimulationJournal(self.path,protected_demo=True,clock=self.clock)
        with self.assertRaisesRegex(StorageProtectionError,'SOLO_DIARIO_FICTICIO_GENERADO'):
            self.j.freeze(replace(self.op,approval_version=2))

    def test_damaged_key_blocks_reopen(self):
        self.ready(self.op)
        self.j.db.execute('UPDATE journal_storage SET wrapped_key=?',(b'bad',))
        self.j.close()
        with self.assertRaisesRegex(StorageProtectionError,'CLAVE_DIARIO_NO_DISPONIBLE'):
            SimulationJournal(self.path,protected_demo=True)
        self.j=sqlite3.connect(self.path)
        self.assertEqual(self.j.execute('SELECT COUNT(*) FROM write_attempts').fetchone()[0],0)
