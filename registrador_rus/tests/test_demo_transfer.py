"""Transferencia entre procesos: solamente la familia de operaciones generada por la demo."""
from dataclasses import replace
from pathlib import Path
from tempfile import TemporaryDirectory
import subprocess
import sys
import tkinter as tk
from unittest import TestCase
from unittest.mock import patch

from registrador_rus.app.contracts import ContractError
from registrador_rus.app.demo_transfer import export_demo, import_demo, make_demo_batch, require_demo_batch
from registrador_rus.app.main import DemoApp
from registrador_rus.app.journal import JournalError
from registrador_rus.app.transfer import PreviewBatch


class DemoTransferTests(TestCase):
    def setUp(self):
        self.temp=TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path=Path(self.temp.name)/'lote.flow-demo.json'

    def test_producer_process_and_consumer_400(self):
        result=subprocess.run([sys.executable,'-m','registrador_rus.app.demo_transfer',str(self.path),'--count','400'],
                              capture_output=True,text=True,timeout=20)
        self.assertEqual(result.returncode,0,result.stderr)
        batch=import_demo(self.path)
        self.assertEqual(batch,make_demo_batch(400))
        before=self.path.read_bytes()
        with self.assertRaises(FileExistsError):
            export_demo(self.path,8)
        self.assertEqual(self.path.read_bytes(),before)
        self.assertEqual(list(Path(self.temp.name).iterdir()),[self.path])

    def test_rejects_valid_but_non_demo_contract(self):
        batch=make_demo_batch(2)
        changed=replace(batch.operations[0],observation_state='NO_DEMO')
        non_demo=PreviewBatch((changed,batch.operations[1]),batch.source_digest,batch.revision)
        self.path.write_bytes(non_demo.encode())
        with self.assertRaisesRegex(ContractError,'SOLO_LOTE_FICTICIO_GENERADO'):
            import_demo(self.path)
        for invalid in (replace(batch,revision=1),replace(batch,source_digest='a'*64)):
            with self.assertRaises(ContractError): require_demo_batch(invalid)
        self.path.write_bytes(b'no-json')
        with self.assertRaises(ContractError): import_demo(self.path)
        with self.assertRaises(ContractError): export_demo(self.path.with_suffix('.txt'))

    def test_loaded_ui_still_requires_individual_authorization(self):
        export_demo(self.path,12)
        root=tk.Tk()
        app=DemoApp(root,demo_batch=import_demo(self.path))
        try:
            root.update()
            self.assertEqual(len(app.table.get_children()),12)
            self.assertEqual(app.journal.attempt_count(),0)
            self.assertEqual(app.journal.db.execute('SELECT COUNT(*) FROM authorizations').fetchone()[0],0)
            app.table.selection_set('demo-12')
            app.prepare()
            with self.assertRaises(JournalError): app.execute()
            self.assertEqual(app.journal.attempt_count(),0)
            with patch('registrador_rus.app.main.messagebox.askyesno',return_value=True): app.authorize()
            app.execute()
            self.assertEqual(app.journal.state('demo-12'),'COMPROBADA')
            self.assertEqual(app.journal.attempt_count(),1)
        finally:
            app.close()

    def test_cancelled_producer_dialog_and_export(self):
        from registrador_rus.app.demo_transfer import export_demo_dialog
        with patch('tkinter.simpledialog.askinteger',return_value=None),patch('tkinter.filedialog.asksaveasfilename') as dialog:
            export_demo_dialog(None)
            dialog.assert_not_called()
        with patch('tkinter.simpledialog.askinteger',return_value=3),patch('tkinter.filedialog.asksaveasfilename',return_value=str(self.path)),patch('tkinter.messagebox.showinfo'):
            export_demo_dialog(None)
        self.assertEqual(import_demo(self.path),make_demo_batch(3))
