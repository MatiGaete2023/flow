"""Ensayo integral del candidato real con ZIP, parser y DPAPI nativa.

Solo datos generados, HTTP loopback, sin RUS ni Chrome institucional.
La puerta de aceptación se sustituye únicamente dentro de este test.
"""
from datetime import date,datetime,timezone,timedelta
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest import TestCase,skipUnless
from unittest.mock import patch
import json
import os
from urllib.request import Request,urlopen

from nurus.capture import read_capture,write_capture
from nurus.capture.fixtures import fixture
from nurus.personal.work import Work,Row
from registrador_rus.app.flow_batch import ReviewRequest,build_review,confirm_review
from registrador_rus.app.confirmed_preview import ConfirmedPreview
from registrador_rus.app.transfer import PreviewBatch
from registrador_rus.app.pilot_journal import PilotJournal,PilotPolicy
from registrador_rus.app.pilot_bridge import open_pilot_bridge,expected_report
from registrador_rus.app.pilot_controller import PilotController
from registrador_rus.app.private_demo_directory import create_private_demo_directory
from registrador_rus.app.reconcile_excel import HEADERS,ExcelRequest,reconcile_pilot
from registrador_rus.app.journal import JournalError
from test_reader_integration import fixture as html_fixture,params

ORIGIN='chrome-extension://'+'a'*32

@skipUnless(os.name=='nt','DPAPI nativa solo Windows')
class PilotIntegralTests(TestCase):
    def test_candidate_work_to_durable_receipt_and_excel(self):
        from openpyxl import Workbook,load_workbook
        with TemporaryDirectory() as tmp,patch('registrador_rus.app.pilot_journal._require_accepted'),patch('registrador_rus.app.pilot_bridge._enabled'):
            root=Path(tmp);folder=create_private_demo_directory(root/'private')
            m,t=fixture(1,partial=True)
            for q in t['consultas']:q['court_code']='900000'
            for rec in t['registros']:rec.update(court_code='900000',cause_id='900001',ingreso_id='900002')
            path=write_capture(root/'fixture.zip',m,t)
            capture=read_capture(path);rec=capture.rows('registros')[0];ref=rec['record_id']
            w=Work({});w.source_scope='PAQUETE';w.path=str(path);w.capture_digest=capture.digest;w.capture_sources={ref:rec}
            w.rows.append(Row(ref,int(rec['original_row']),dict(SITFA_TRIBUNAL_CODIGO='900000',SITFA_CAUSA_RUS='900001',SITFA_INGRESO_RUS='900002'),'Constancia ficticia integral.',[],[],[]))
            requests=(ReviewRequest(ref,'ESPERA','AL_TRIBUNAL'),)
            frozen=build_review(w,requests,work_id='fixture')
            ops=confirm_review(w,frozen,requests,displayed_digest=frozen.digest,confirmed=True,actor='fixture',
                stamp='2026-11-24T12:00:00Z',version=1)
            preview=ConfirmedPreview(frozen,PreviewBatch(ops,capture.digest,w.revision))
            policy=PilotPolicy(folder,1,'fixture-only',True)
            journal_path=folder/'registrador.pilot.protected.sqlite'
            j=PilotJournal(journal_path,policy=policy)
            bridge=None
            def attach(op):
                return open_pilot_bridge(ORIGIN,op)
            def rpc(route,body):
                request=Request('http://'+bridge.host+'/v1/'+route,data=json.dumps(body).encode(),
                    headers={'Content-Type':'application/json','Origin':ORIGIN,'Authorization':'Bearer '+bridge.token},method='POST')
                with urlopen(request,timeout=10) as response:return json.loads(response.read())
            def poll():
                return rpc('poll',dict(schema='BridgePollV1',session_id=bridge.mailbox.session_id))['command']
            def ack(cmd,status='OK',**body):
                return rpc('ack',dict(schema='PilotAckV1',session_id=bridge.mailbox.session_id,command_id=cmd['command_id'],status=status,**body))
            try:
                op=j.freeze_confirmed(preview,w,requests,report_catalog=None)
                bridge=attach(op);c=PilotController(j,bridge.mailbox,op,revalidate=lambda:preview.revalidate(w,requests).operations[0])
                ctx=params(op);report=expected_report(op)
                c.prepare(confirmed=True);cmd=poll();self.assertEqual(cmd['command'],'prepare')
                ack(cmd,report=report,context=ctx);self.assertEqual(c.poll(),'PREPARADA')
                c.authorize_and_verify(author='actor-ficticio',stage='Etapa ficticia',server_day=date(2026,11,24),confirmed=True,require_excel=True)
                cmd=poll();self.assertEqual(cmd['command'],'verify')
                ack(cmd,report=report,context=ctx,html=html_fixture(op))
                self.assertEqual(c.poll(),'ENVIANDO');self.assertEqual(j.attempt_count(),1)
                cmd=poll();self.assertEqual(cmd['command'],'create_once')
                self.assertEqual(j.db.execute('SELECT COUNT(*) FROM pilot_reservations').fetchone()[0],1)
                self.assertIsNone(poll())
                # Pérdida de ACK: no se ejecuta ninguna acción en RUS; se inventa la respuesta para probar recuperación.
                bridge.close();bridge=None;j.close()
                j=PilotJournal(journal_path,policy=policy)
                self.assertEqual(j.load_unresolved(),op)
                bridge=attach(op);c=PilotController(j,bridge.mailbox,op,recovery=True)
                self.assertEqual(j.state(op.operation_id),'INCIERTA')
                c.request_readback(confirmed=True);cmd=poll()
                ack(cmd,context=ctx,html=html_fixture(op,entries=('900001',),partial=True))
                self.assertEqual(c.poll(),'INCIERTA')
                c.request_readback(confirmed=True);cmd=poll()
                ack(cmd,context=ctx,html=html_fixture(op,entries=('900001',)))
                self.assertEqual(c.poll(),'PENDIENTE_EXCEL')
                self.assertEqual(j.db.execute('SELECT simulated FROM readback_receipts').fetchone()[0],0)
                wb=Workbook();ws=wb.active;ws.title='Fixture';ws.append([*HEADERS,'TT'])
                i=op.remote_identity;ws.append([i.tribunal_id,i.causa_id,i.ingreso_id,op.approved_text,None,0,7])
                source=root/'source.xlsx';wb.save(source);before=source.read_bytes()
                destination=folder/'copy.xlsx'
                reconcile_pilot(j,ExcelRequest(op,'Fixture',None,0),source,destination)
                self.assertEqual(source.read_bytes(),before)
                check=load_workbook(destination);self.assertEqual(check['Fixture']['F2'].value,1)
                self.assertEqual(check['Fixture']['G2'].value,7);check.close()
                self.assertEqual(j.state(op.operation_id),'COMPROBADA');self.assertEqual(j.attempt_count(),1)
                self.assertEqual(j.db.execute('SELECT simulated FROM excel_reconciliations').fetchone()[0],0)
            finally:
                if bridge:bridge.close()
                j.close()
