"""Contrato del diario candidato. Todo el contenido de estas pruebas es inventado."""
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest import TestCase,skipUnless
from unittest.mock import patch
from dataclasses import replace
from datetime import date
import os

from registrador_rus.app.pilot_journal import PilotJournal,PilotPolicy
from registrador_rus.app.journal import JournalError,SimulationJournal
from registrador_rus.app.private_demo_directory import create_private_demo_directory
from registrador_rus.app.journal_protection import StorageProtectionError
from registrador_rus.app.simulation import FakeClock

class PilotGateTests(TestCase):
    def test_disabled_before_file_creation(self):
        with TemporaryDirectory() as tmp:
            folder=Path(tmp)
            path=folder/'unit.pilot.protected.sqlite'
            policy=PilotPolicy(folder,1,'fixture-only',True)
            with self.assertRaisesRegex(JournalError,'PILOTO_REAL_NO_HABILITADO'):
                PilotJournal(path,policy=policy)
            self.assertFalse(path.exists())
            with self.assertRaises(StorageProtectionError):SimulationJournal(path,protected_demo=True)
    def test_no_policy_default(self):
        with self.assertRaises(JournalError):PilotPolicy(Path('relative'),60,'',False)

@skipUnless(os.name=='nt','DPAPI nativa solo Windows')
class PilotProfileTests(TestCase):
    def test_separate_schema_and_policy(self):
        # Solo en el proceso de test se sustituye la puerta de aceptación.
        # No se crea una reserva ni recibo institucional.
        with TemporaryDirectory() as tmp:
            folder=create_private_demo_directory(Path(tmp)/'private')
            path=folder/'unit.pilot.protected.sqlite'
            policy=PilotPolicy(folder,1,'fixture-only',True)
            with patch('registrador_rus.app.pilot_journal._require_accepted'):
                j=PilotJournal(path,policy=policy,clock=FakeClock())
                try:
                    for name in ('write_attempts','readback_receipts'):
                        ddl=j.db.execute('SELECT sql FROM sqlite_master WHERE name=?',(name,)).fetchone()[0]
                        self.assertIn('CHECK(simulated=0)',ddl)
                    self.assertEqual(j.attempt_count(),0)
                    self.assertEqual(j.db.execute('SELECT typeof(policy) FROM pilot_policy').fetchone()[0],'blob')
                    with self.assertRaisesRegex(JournalError,'USAR_REVISION_FLOW_CONFIRMADA'):j.freeze(None)
                finally:j.close()
                with self.assertRaisesRegex(JournalError,'POLITICA_DIARIO_CAMBIADA'):
                    PilotJournal(path,policy=replace(policy,retention_days=2))
            with self.assertRaises(JournalError):PilotJournal(path,policy=policy)
