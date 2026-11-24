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
        d.prepare()
        return d

    def test_preview_and_invalidation(self):
        d=self.dialog()
        self.assertIn('Texto ficticio',d.text.get('1.0','end'))
        self.assertIn('Gestión Previa (4)',d.text.get('1.0','end'))
        self.assertEqual(d.state_label.get(),'Gestión Previa (4)')
        with self.assertRaises(ContractError): d.confirm()
        d.destination.set('AL_TRIBUNAL')
        self.assertIsNone(d.frozen)
        d.prepare()
        d.accepted.set(True)
        packet=d.confirm()
        self.assertEqual(len(packet.operations),2)
        self.assertEqual(d.revalidate_confirmed(),packet)
        self.work.rows[0].review['OBSERVACION']='Cambio posterior ficticio'
        with self.assertRaisesRegex(ContractError,'LOTE_CAMBIADO_REVISAR'): d.revalidate_confirmed()
        self.assertIsNone(d.confirmed_preview)
        self.work.rows[0].review.clear()
        self.assertTrue(all(o.cc==1 and o.observation_state=='4' for o in packet.operations))
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

    def test_no_package_blocks_before_dialog(self):
        self.work.source_scope='SOLO_PLANILLA'
        with self.assertRaisesRegex(ContractError,'SE_REQUIERE_TRABAJO_DE_PAQUETE_VERIFICADO'):
            open_work_preview(self.root)

    def test_real_assistant_menu(self):
        # Aislar Tcl/CustomTkinter tal como tests/test_integral_ui.py.
        import os
        import subprocess
        import sys
        script = """
from pathlib import Path
from unittest.mock import patch
from nurus.personal.config import Configuration
from nurus.personal.app import App
import sys
app=App(Configuration(Path(sys.argv[1])/'config'))
try:
    app.update_idletasks()
    menu=app.work_more_menu
    matches=[i for i in range(menu.index('end')+1) if menu.type(i)=='command' and menu.entrycget(i,'label')=='Vista previa de lote RUS…']
    assert len(matches)==1, matches
    with patch('registrador_rus.app.work_preview.open_work_preview') as opened:
        menu.invoke(matches[0])
        opened.assert_called_once_with(app)
finally:
    app.destroy()
"""
        env=dict(os.environ,PYTHONPATH=os.pathsep.join([str(Path.cwd()/'src'),str(Path.cwd())]))
        result=subprocess.run([sys.executable,'-c',script,self.temp.name],env=env,capture_output=True,text=True,timeout=45)
        self.assertEqual(result.returncode,0,result.stdout+'\n'+result.stderr)

    def test_real_import_to_preview(self):
        """Recorrido GUI real; nunca se limpia artificialmente row.warnings."""
        import os
        import subprocess
        import sys
        script = """
from datetime import date
from pathlib import Path
from unittest.mock import patch
from nurus.capture import write_capture
from nurus.capture.fixtures import fixture
from nurus.personal.config import Configuration
from nurus.personal.app import App
from registrador_rus.app.work_preview import open_work_preview
from registrador_rus.app.flow_export import TEMPORAL_NOTICE
import sys,time
root=Path(sys.argv[1])/'import'
root.mkdir()
m,t=fixture(1,partial=True)
for q in t['consultas']:q['court_code']='900000'
for rec in t['registros']:rec.update(court_code='900000',cause_id='900001',ingreso_id='900002')
path=write_capture(root/'fixture.zip',m,t)
app=App(Configuration(root/'config'))
try:
    app._run=lambda label,action,done=None:done(action()) if done else action()
    with patch.object(app,'_ask_cut_date',return_value=date(2026,10,8)):
        app._import_capture(str(path))
    deadline=time.monotonic()+10
    while getattr(app,'rendering',False) and time.monotonic()<deadline:app.update()
    app.update()
    assert not getattr(app,'rendering',False)
    assert len(app.work.rows)==1
    row=app.work.rows[0]
    assert row.warnings==[TEMPORAL_NOTICE],row.warnings
    assert app.work.output and Path(app.work.output).is_file()
    app.records.selection_set(row.id);app._detail()
    app.observation_editor.delete('1.0','end');app.observation_editor.insert('1.0','Constancia ficticia revisada.')
    d=open_work_preview(app)
    d.origin.set('ESPERA');d.destination.set('ADMINISTRATIVA')
    d.prepare()
    assert TEMPORAL_NOTICE in d.text.get('1.0','end')
    assert 'Gestión Previa (4)' in d.text.get('1.0','end')
    assert 'Constancia ficticia revisada.' in d.text.get('1.0','end')
    d.accepted.set(True);packet=d.confirm()
    assert len(packet.operations)==1 and packet.operations[0].approved_text=='Constancia ficticia revisada.'
    assert packet.operations[0].observation_state=='4' and packet.operations[0].cc==0
    assert row.warnings==[TEMPORAL_NOTICE]
    d.window.destroy()
finally:
    app.destroy()
"""
        env=dict(os.environ,PYTHONPATH=os.pathsep.join([str(Path.cwd()/'src'),str(Path.cwd())]))
        result=subprocess.run([sys.executable,'-c',script,self.temp.name],env=env,capture_output=True,text=True,timeout=45)
        self.assertEqual(result.returncode,0,result.stdout+'\n'+result.stderr)
