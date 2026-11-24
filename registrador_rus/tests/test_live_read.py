"""Contrato del puente de lectura: prueba local, sin RUS y sin escritura."""
from dataclasses import replace
from hashlib import sha256
from tempfile import TemporaryDirectory
from pathlib import Path
from unittest import TestCase
from unittest.mock import patch
from registrador_rus.app.contracts import RegistroFlowV1,Source
from registrador_rus.app.bridge import BridgeError
from registrador_rus.app.live_read import LiveReadMailbox
from registrador_rus.app.simulation import demo_operations
from registrador_rus.app.build_live_extension import build

class LiveReadTests(TestCase):
    def setUp(self):
        self.op=replace(demo_operations(1)[0],source=Source('FLOW','fixture','r0'),operation_id=sha256(b'fixture').hexdigest(),observation_state='4')
        self.box=LiveReadMailbox(self.op)
        self.poll=dict(schema='BridgePollV1',session_id=self.box.session_id)
        i=self.op.remote_identity
        self.context=dict(COD_Tribunal=i.tribunal_id,CRR_IdCausa=i.causa_id,ID_Ingreso=i.ingreso_id,COD_Etapa='1',TIP_Consulta='2',tipo_popUp='11')
        self.ack=dict(schema='LiveReadAckV1',session_id=self.box.session_id,command_id=self.box.command_id,status='OK',context=self.context,html='<html></html>')
    def test_once_and_invalid_context(self):
        with self.assertRaises(BridgeError):self.box.ack(self.ack)
        self.assertFalse(self.box.poll(self.poll)['command']['write_enabled'])
        self.assertIsNone(self.box.poll(self.poll)['command'])
        self.ack['context']=dict(self.context,ID_Ingreso='9999')
        self.box.ack(self.ack)
        self.assertEqual(self.box.take_result(),dict(status='LECTURA_NO_VERIFICADA',write_enabled=False))
        with self.assertRaises(BridgeError):self.box.ack(self.ack)
    def test_blocked_and_no_write(self):
        self.box.poll(self.poll)
        with self.assertRaises(BridgeError):self.box.ack(dict(self.ack,status='WRITE'))
        self.box.ack({k:v for k,v in dict(self.ack,status='BLOCKED').items() if k not in ('html','context')})
        self.assertEqual(self.box.take_result(),dict(status='BLOQUEADA',write_enabled=False))
    def test_parser_result_without_data(self):
        from registrador_rus.app.readback import Snapshot
        from datetime import datetime,timezone
        self.box.poll(self.poll)
        with patch('nurus.bitacora_html.read_snapshot',return_value={}),patch('registrador_rus.app.live_read.Snapshot.from_reader',
             return_value=Snapshot(self.op.remote_identity,datetime.now(timezone.utc),'1',(),True)):
            self.box.ack(self.ack)
        self.assertEqual(self.box.take_result(),dict(status='LECTURA_COMPLETA',entries=0,operation_id=self.op.operation_id,write_enabled=False))
        self.assertIsNone(self.box.take_result())
    def test_prepare_mode_requires_exact_report(self):
        from registrador_rus.app.pilot_bridge import expected_report
        from registrador_rus.app.readback import Snapshot
        from datetime import datetime,timezone
        box=LiveReadMailbox(self.op,prepare_only=True)
        command=box.poll(dict(schema='BridgePollV1',session_id=box.session_id))['command']
        self.assertEqual(command['command'],'validate_prepare_only')
        body=dict(self.ack,session_id=box.session_id,command_id=box.command_id,report=expected_report(self.op))
        with patch('nurus.bitacora_html.read_snapshot',return_value={}),patch('registrador_rus.app.live_read.Snapshot.from_reader',return_value=Snapshot(self.op.remote_identity,datetime.now(timezone.utc),'1',(),True)):
            box.ack(body)
        self.assertTrue(box.take_result()['prepared'])

    def test_build_exclusive(self):
        with TemporaryDirectory() as d:
            path=build(Path(d)/'extension')
            self.assertEqual({x.name for x in path.iterdir()},{'manifest.json','background.js','verify.html','verify.js','browser.js','prepare.js','observed_form.js','live_adapter.js'})
            with self.assertRaises(FileExistsError):build(path)
