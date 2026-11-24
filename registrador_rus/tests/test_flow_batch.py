"""Flujo Work -> revisión -> transferencia en memoria; solo fixtures."""
from hashlib import sha256
import json
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest import TestCase
from unittest.mock import patch

from nurus.capture import read_capture, write_capture
from nurus.capture.fixtures import fixture
from nurus.personal.work import Work, Row
from registrador_rus.app.contracts import ContractError
from registrador_rus.app.flow_batch import ReviewRequest, build_review, confirm_review
from registrador_rus.app.transfer import PreviewBatch, MAX_BYTES


class FlowBatchTests(TestCase):
    def setUp(self):
        self.tmp = TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        metadata,tables = fixture(400,partial=True)
        for q in tables['consultas']:
            q['court_code']='900000'
        for i,rec in enumerate(tables['registros']):
            rec.update(court_code='900000',cause_id=str(900000+i//2),ingreso_id=str(910000+i))
        path=write_capture(Path(self.tmp.name)/'demo.zip',metadata,tables)
        capture=read_capture(path)
        self.work=Work({})
        self.work.source_scope='PAQUETE'
        self.work.path=str(path)
        self.work.capture_digest=capture.digest
        self.work.capture_sources={}
        self.requests=[]
        for rec in capture.rows('registros'):
            ref=rec['record_id']
            self.work.capture_sources[ref]=rec
            self.work.rows.append(Row(ref,int(rec['original_row']),
                dict(SITFA_TRIBUNAL_CODIGO=rec['court_code'],SITFA_CAUSA_RUS=rec['cause_id'],SITFA_INGRESO_RUS=rec['ingreso_id']),
                'Constancia ficticia.',[],[],[]))
            self.requests.append(ReviewRequest(ref,'ESPERA','ADMINISTRATIVA','demo'))
        self.requests=tuple(self.requests)

    def confirm(self,frozen,requests=None,**changes):
        args=dict(displayed_digest=frozen.digest,confirmed=True,actor='ficticio',
                  stamp='2026-11-24T12:00:00Z',version=1)
        return confirm_review(self.work,frozen,self.requests if requests is None else requests,**(args|changes))

    def test_400_reads_once_per_phase_and_round_trip(self):
        with patch('nurus.capture.read_capture',wraps=read_capture) as reader:
            frozen=build_review(self.work,self.requests,work_id='demo')
            self.assertEqual(reader.call_count,1)
            operations=self.confirm(frozen)
            self.assertEqual(reader.call_count,2)
        packet=PreviewBatch(operations,self.work.capture_digest,self.work.revision)
        self.assertEqual(PreviewBatch.decode(packet.encode()),packet)
        self.assertEqual(len(operations),400)
        self.assertEqual(self.confirm(frozen),operations)

    def test_selection_and_source_changes_block(self):
        frozen=build_review(self.work,self.requests,work_id='demo')
        for selection in (self.requests[:-1],self.requests[::-1]):
            with self.assertRaises(ContractError): self.confirm(frozen,selection)
        with self.assertRaises(ContractError): self.confirm(frozen,confirmed=False)
        self.work.rows[-1].review['OBSERVACION']='Otro texto ficticio'
        with self.assertRaises(ContractError): self.confirm(frozen)
        self.work.rows[-1].review.clear()
        Path(self.work.path).write_bytes(b'cambio')
        with self.assertRaises(ContractError): self.confirm(frozen)

    def test_transfer_rejects_tampering_and_wrong_shape(self):
        frozen=build_review(self.work,self.requests[:2],work_id='demo')
        operations=self.confirm(frozen,self.requests[:2])
        packet=PreviewBatch(operations,self.work.capture_digest,self.work.revision)
        original=json.loads(packet.encode())
        for change in ('text','permission','extra','count'):
            data=json.loads(packet.encode())
            if change=='text': data['body']['operations'][0]['approved_text']='Otro'
            if change=='permission': data['body']['preview_only']=False
            if change=='extra': data['body']['write_one']=True
            if change=='count': data['body']['operations']=[]
            if change!='text':
                data['sha256']=sha256(json.dumps(data['body'],ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()).hexdigest()
            with self.assertRaises(ContractError): PreviewBatch.decode(json.dumps(data).encode())
        for bad in (b'{"body":{},"body":{},"sha256":"x"}',b'x'*(MAX_BYTES+1),b'[]',b'\xff'):
            with self.assertRaises(ContractError): PreviewBatch.decode(bad)
        with self.assertRaises(ContractError): PreviewBatch((operations[0],operations[0]),self.work.capture_digest,0)
        self.assertTrue(original['body']['preview_only'])
