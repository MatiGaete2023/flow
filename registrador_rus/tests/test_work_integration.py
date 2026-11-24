"""Prueba de puerta Work con clase y paquete reales; fuentes exclusivamente ficticias."""
from hashlib import sha256
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

from nurus.capture import read_capture, write_capture
from nurus.capture.fixtures import fixture
from nurus.personal.work import Work, Row
from nurus.personal.record_edits import encode
from registrador_rus.app.contracts import ContractError
from registrador_rus.app.flow_work import candidate_from_work, approve_from_work


class WorkIntegrationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = TemporaryDirectory()
        root = Path(self.tmp.name)
        metadata, tables = fixture(1, partial=True)
        # El fixture general usa identificadores alfanuméricos; para este contrato
        # se emplean valores numéricos inventados exclusivamente para la prueba.
        for q in tables['consultas']:
            q['court_code'] = '900000'
        for record in tables['registros']:
            record.update(court_code='900000', cause_id='900001', ingreso_id='900002')
        package = write_capture(root/'demo.zip', metadata, tables)
        capture = read_capture(package)
        source = capture.rows('registros')[0]
        row = Row(source['record_id'], 2,
                  dict(SITFA_TRIBUNAL_CODIGO='900000',SITFA_CAUSA_RUS='900001',SITFA_INGRESO_RUS='900002'),
                  'Propuesta ficticia', [], [], [])
        row.review['OBSERVACION'] = 'Texto ficticio revisado'
        row.sync_state = encode(row)
        work = Work({})
        work.rows = [row]
        work.source_scope = 'PAQUETE'
        work.capture_digest = capture.digest
        work.capture_sources = {row.id: source}
        work.path = str(package)
        work.output = str(root/'demo-copia.xlsx')
        Path(work.output).write_bytes(b'fixture de huella, no se analiza Excel')
        work.output_hash = sha256(Path(work.output).read_bytes()).hexdigest()
        self.work, self.row = work, row

    def tearDown(self):
        self.tmp.cleanup()

    def candidate(self):
        return candidate_from_work(self.work, self.row.id, work_id='demo-work', origin='ESPERA',
                                   destination='ADMINISTRATIVA', observation_state='demo')

    def approve(self, c):
        return approve_from_work(self.work,c,displayed_digest=c.digest,destination='ADMINISTRATIVA',
                                 observation_state='demo',actor='ficticio',stamp='2026-11-24T12:00:00Z',
                                 version=1,operation_id='demo-op',confirmed=True)

    def test_work_and_copy(self):
        c = self.candidate()
        self.assertEqual(self.approve(c).approved_text,'Texto ficticio revisado')
        Path(self.work.output).write_bytes(b'copia modificada')
        with self.assertRaisesRegex(ContractError,'FUENTE_O_COPIA_NO_VIGENTE'): self.approve(c)

    def test_source_changed(self):
        c = self.candidate()
        self.work.capture_sources[self.row.id] = self.work.capture_sources[self.row.id] | dict(ingreso_id='999999')
        with self.assertRaisesRegex(ContractError,'PROCEDENCIA_CAMBIADA'): self.approve(c)
        Path(self.work.path).write_bytes(b'fuente modificada')
        with self.assertRaisesRegex(ContractError,'FUENTE_O_COPIA_NO_VIGENTE'): self.approve(c)
