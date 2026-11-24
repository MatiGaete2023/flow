"""Diálogo del Asistente con Work/Row y paquete ficticios; sin RUS."""
from pathlib import Path
from tempfile import TemporaryDirectory
import tkinter as tk
from tkinter import ttk
from unittest import TestCase

from nurus.capture import read_capture, write_capture
from nurus.capture.fixtures import fixture
from nurus.personal.work import Work, Row
from registrador_rus.app.contracts import ContractError
from registrador_rus.app.work_preview import open_work_preview, visible_selection
from registrador_rus.app.flow_work import verify_origin, REPOSITORY_REPORT_CATALOG


class WorkPreviewTests(TestCase):
    def setUp(self):
        self.temp=TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        metadata,tables=fixture(2,partial=True)
        for q in tables['consultas']:
            q['court_code']='900000'
        for i,rec in enumerate(tables['registros']):
            rec.update(court_code='900000',cause_id='900001',ingreso_id=str(910000+i))
        path=write_capture(Path(self.temp.name)/'demo.zip',metadata,tables)
        capture=read_capture(path)
        self.work=Work({})
        self.work.source_scope='PAQUETE'
        self.work.path=str(path)
        self.work.capture_digest=capture.digest
        self.work.capture_sources={}
        self.root=tk.Tk()
        self.addCleanup(self.root.destroy)
        self.root._require_work=lambda:self.work
        self.root.records=ttk.Treeview(self.root)
        self.root.records.pack()
        for rec in capture.rows('registros'):
            ref=rec['record_id']
            self.work.capture_sources[ref]=rec
            self.work.rows.append(Row(ref,int(rec['original_row']),
                dict(SITFA_TRIBUNAL_CODIGO=rec['court_code'],SITFA_CAUSA_RUS=rec['cause_id'],SITFA_INGRESO_RUS=rec['ingreso_id']),
                'Texto ficticio',[],[],[]))
            self.root.records.insert('','end',iid=ref)
        self.root.records.selection_set(self.root.records.get_children())
        self.root.update()

    def dialog(self):
        d=open_work_preview(self.root)
        d.origin.set('ESPERA')
        d.destination.set('ADMINISTRATIVA')
        d.state.set('demo')
        d.prepare()
        return d

    def test_preview_and_invalidation(self):
        d=self.dialog()
        self.assertIn('Texto ficticio',d.text.get('1.0','end'))
        with self.assertRaises(ContractError): d.confirm()
        d.destination.set('AL_TRIBUNAL')
        self.assertIsNone(d.frozen)
        d.prepare()
        d.accepted.set(True)
        packet=d.confirm()
        self.assertEqual(len(packet.operations),2)
        self.assertTrue(all(o.cc==1 for o in packet.operations))
        self.assertEqual(self.work.revision,0)
        self.assertEqual([r.review for r in self.work.rows],[{},{}])
        self.assertEqual(len(list(Path(self.temp.name).iterdir())),1)

    def test_selection_change_blocks(self):
        d=self.dialog()
        self.root.records.selection_set(self.root.records.get_children()[0])
        d.accepted.set(True)
        with self.assertRaisesRegex(ContractError,'TRABAJO_O_SELECCION_CAMBIADA'): d.confirm()
        self.assertIsNone(d.preview)

    def test_report_mapping_matches_repository_evidence(self):
        import json
        source=dict(source_system='RUS',court_code='900000',query_id='demo-q',tab='Informes')
        for code,origin in (('1','INFORMES_POR_VENCER'),('2','INFORMES_VENCIDOS')):
            q=source|dict(screen='seguimiento',filters_json=json.dumps({'report':code}))
            verify_origin(source,[q],origin,report_catalog=REPOSITORY_REPORT_CATALOG)
        q=source|dict(screen='seguimiento',filters_json='{"report":"3"}')
        with self.assertRaises(ContractError):
            verify_origin(source,[q],'INFORMES_VENCIDOS',report_catalog=REPOSITORY_REPORT_CATALOG)
