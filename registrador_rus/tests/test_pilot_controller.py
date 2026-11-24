"""Pruebas focalizadas del controlador: objetos ficticios, ninguna orden real."""
from datetime import date
from dataclasses import replace
from hashlib import sha256
from unittest import TestCase
from unittest.mock import Mock,patch
from registrador_rus.app.contracts import Source
from registrador_rus.app.simulation import demo_operations
from registrador_rus.app.pilot_bridge import PilotMailbox,expected_report
from registrador_rus.app.pilot_controller import PilotController
from registrador_rus.app.journal import JournalError
from registrador_rus.app.bridge import BridgeError

class ControllerTests(TestCase):
    def setUp(self):
        self.op=replace(demo_operations(1)[0],source=Source('FLOW','fixture','r0'),operation_id=sha256(b'fixture').hexdigest(),observation_state='4')
        self.report=expected_report(self.op)
        i=self.op.remote_identity
        self.ctx=dict(COD_Tribunal=i.tribunal_id,CRR_IdCausa=i.causa_id,ID_Ingreso=i.ingreso_id,COD_Etapa='1',TIP_Consulta='2',tipo_popUp='11')
        self.ticket=dict(operation_id=self.op.operation_id,approved_text_sha256=self.op.approved_text_sha256,
                         approval_version=self.op.approval_version,state='ENVIANDO',reservation_id='a'*64)
        self.j=Mock();self.j.state.return_value='PREPARADA';self.j.reserve_creation.return_value=self.ticket
        self.j.reservation_context.return_value=self.ctx
        self.mb=Mock();self.mb.op=self.op
        self.current=Mock(return_value=self.op)
        self.c=PilotController(self.j,self.mb,self.op,revalidate=self.current)
    def ready(self):
        self.c.prepare(confirmed=True)
        self.mb.take_result.return_value=dict(command='prepare',status='OK',report=self.report,context=self.ctx)
        self.assertEqual(self.c.poll(),'PREPARADA')
        self.c.authorize_and_verify(author='fixture',stage='Etapa ficticia',server_day=date(2026,11,24),confirmed=True)
        self.mb.take_result.return_value=dict(command='verify',status='OK',report=self.report,context=self.ctx,raw={})
    def test_order_and_no_ack_success(self):
        self.ready()
        order=[]
        self.j.reserve_creation.side_effect=lambda *a,**k:(order.append('commit'),self.ticket)[1]
        self.mb.submit.side_effect=lambda *a,**k:order.append(a[0])
        self.assertEqual(self.c.poll(),'ENVIANDO')
        self.assertEqual(order,['commit','create_once'])
        self.j.state.return_value='ENVIANDO'
        self.mb.take_result.return_value=dict(command='create_once',status='INCIERTA')
        self.assertEqual(self.c.poll(),'INCIERTA')
        self.j.mark_uncertain.assert_called_once_with(self.op.operation_id)
        self.c.request_readback(confirmed=True)
        self.j.resolve_creation.return_value='COMPROBADA'
        self.mb.take_result.return_value=dict(command='readback',status='OK',raw={})
        self.assertEqual(self.c.poll(),'COMPROBADA')
        self.j.reserve_creation.assert_called_once()
    def test_change_and_refusal(self):
        with self.assertRaises(JournalError):self.c.prepare(confirmed=False)
        self.ready();self.current.return_value=replace(self.op,approval_version=2)
        with self.assertRaises(ValueError):self.c.poll()
        self.j.reserve_creation.assert_not_called()
    def test_mailbox_creation_once(self):
        with self.assertRaises(BridgeError):PilotMailbox(self.op)
        with patch('registrador_rus.app.pilot_bridge._enabled'):
            mb=PilotMailbox(self.op)
            mb.submit('create_once',reservation=self.ticket)
            query=dict(schema='BridgePollV1',session_id=mb.session_id)
            command=mb.poll(query)['command']
            self.assertIsNone(mb.poll(query)['command'])
            mb.ack(dict(schema='PilotAckV1',session_id=mb.session_id,command_id=command['command_id'],status='OK'))
            self.assertEqual(mb.take_result()['status'],'INCIERTA')
            with self.assertRaisesRegex(BridgeError,'CREACION_YA_EMITIDA'):mb.submit('create_once',reservation=self.ticket)
