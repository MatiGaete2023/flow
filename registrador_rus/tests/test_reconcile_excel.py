from dataclasses import replace
from datetime import datetime, timedelta, timezone, date
from pathlib import Path
import sqlite3
from tempfile import TemporaryDirectory
import unittest

from openpyxl import Workbook, load_workbook
from openpyxl.styles import Font
from registrador_rus.app.contracts import RegistroFlowV1, RemoteIdentity
from registrador_rus.app.journal import SimulationJournal
from registrador_rus.app.reconcile_excel import HEADERS, ExcelRequest, ExcelConflict, reconcile_simulated
from test_contracts import sample
from readback_fixture import bind, snapshot

NOW = datetime(2026, 11, 24, 14, tzinfo=timezone.utc)


class ExcelTests(unittest.TestCase):
    def setUp(self):
        self.tmp=TemporaryDirectory(); self.root=Path(self.tmp.name); self.source=self.root/'original.xlsx'; self.dest=self.root/'copia.xlsx'
        self.now=NOW; self.j=SimulationJournal(self.root/'sim.sqlite',clock=lambda:self.now)
        self.op=RegistroFlowV1.from_mapping(sample())
        self.req=ExcelRequest(self.op,'Demo',None,0)
        self.pending(self.op)
        self.make_book()

    def tearDown(self):
        self.j.close();self.tmp.cleanup()

    def pending(self,op,key='1'):
        self.j.freeze(op);self.j.mark_ready(op);self.j.authorize_simulation(op,human_actor='demo',confirmed=True)
        bind(self.j,op,self.now,require_excel=True);self.j.reserve_simulated_attempt(op);self.now+=timedelta(seconds=1)
        row=dict(entry_id=key,texto=op.approved_text,autor='demo',tipo='Administrativa' if not op.cc else 'Al Tribunal',
                 etapa='Etapa ficticia',fecha='2026-11-24T11:00:00',origen='centro')
        self.assertEqual(self.j.resolve_simulated(op,snapshot(op,self.now,rows=[row])),'PENDIENTE_EXCEL')

    def make_book(self, *, duplicate=False, changed=False, cc=0, previous=None, formula=False, merged=False, missing_code=False, extra=None):
        wb=Workbook(); ws=wb.active; ws.title='Demo'; ws.append([*HEADERS,'TT','RES','Formula'])
        if missing_code: ws['A1']='TRIBUNAL'
        i=self.op.remote_identity
        # La fila exacta está después de otra del mismo tribunal y causa, distinto ingreso.
        ws.append([i.tribunal_id,i.causa_id,'OTRO','Texto de otra fila',None,0,9,'Otro','=1+1'])
        values=[i.tribunal_id,i.causa_id,i.ingreso_id,'Texto cambiado' if changed else self.op.approved_text,previous,cc,1,'Mantener','=SUM(G2:G3)']
        ws.append(values); ws['G3'].font=Font(bold=True)
        if formula: ws['F3']='=0'
        if merged: ws.merge_cells('E3:E4')
        if duplicate: ws.append(values)
        if extra:
            ident=extra.remote_identity
            ws.append([ident.tribunal_id,ident.causa_id,ident.ingreso_id,extra.approved_text,None,0,2,'Segundo','=2+2'])
        wb.create_sheet('Otra')['A1']='Intacta'; wb.save(self.source)

    def test_copy_and_preservation(self):
        before=self.source.read_bytes()
        reconcile_simulated(self.j,[self.req],self.source,self.dest)
        self.assertEqual(self.source.read_bytes(),before)
        wb=load_workbook(self.dest); ws=wb['Demo']
        self.assertEqual(ws['E3'].value.date(),date(2026,11,24));self.assertEqual(ws['F3'].value,0)
        self.assertEqual(ws['D3'].value,self.op.approved_text);self.assertEqual(ws['G3'].value,1)
        self.assertTrue(ws['G3'].font.bold);self.assertEqual(ws['H3'].value,'Mantener')
        self.assertEqual(ws['I3'].value,'=SUM(G2:G3)');self.assertEqual(wb['Otra']['A1'].value,'Intacta')
        self.assertIsNone(ws['E2'].value);self.assertEqual(self.j.state(self.op.operation_id),'COMPROBADA')
        self.assertEqual(self.j.attempt_count(),1)

    def test_conflicts_leave_pending(self):
        for kwargs in (dict(duplicate=True),dict(changed=True),dict(cc=1),dict(previous=date(2026,11,23)),
                       dict(formula=True),dict(merged=True),dict(missing_code=True)):
            with self.subTest(kwargs=kwargs):
                self.make_book(**kwargs); before=self.source.read_bytes()
                with self.assertRaises(ExcelConflict): reconcile_simulated(self.j,[self.req],self.source,self.dest)
                self.assertEqual(self.source.read_bytes(),before);self.assertFalse(self.dest.exists())
                self.assertEqual(self.j.state(self.op.operation_id),'PENDIENTE_EXCEL')
        self.assertEqual(self.j.attempt_count(),1)

    def test_batch_and_tribunal(self):
        other=replace(self.op,operation_id='otro',remote_identity=RemoteIdentity('SITFA','T-DEMO','C-DEMO','I-SEGUNDO'),destination='AL_TRIBUNAL')
        self.pending(other,key='2');self.make_book(extra=other)
        reconcile_simulated(self.j,[self.req,ExcelRequest(other,'Demo',None,0)],self.source,self.dest)
        wb=load_workbook(self.dest);self.assertEqual(wb['Demo']['F4'].value,1)
        self.assertEqual(self.j.state(other.operation_id),'COMPROBADA');self.assertEqual(self.j.attempt_count(),2)

    def test_existing_destination_and_commit_failure(self):
        self.dest.write_bytes(b'no reemplazar')
        with self.assertRaises(ExcelConflict): reconcile_simulated(self.j,[self.req],self.source,self.dest)
        self.assertEqual(self.dest.read_bytes(),b'no reemplazar');self.dest.unlink()
        self.j.db.execute("CREATE TRIGGER fail_excel BEFORE INSERT ON audit WHEN NEW.action='EXCEL_SIMULADO_VERIFICADO' BEGIN SELECT RAISE(ABORT,'demo'); END")
        with self.assertRaises(sqlite3.IntegrityError): reconcile_simulated(self.j,[self.req],self.source,self.dest)
        self.assertTrue(self.dest.exists());self.assertEqual(self.j.state(self.op.operation_id),'PENDIENTE_EXCEL');self.assertEqual(self.j.attempt_count(),1)
