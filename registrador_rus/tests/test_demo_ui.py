import tkinter as tk
from unittest.mock import patch
import unittest

from registrador_rus.app.main import DemoApp


class DemoUITests(unittest.TestCase):
    def test_selection_confirmation_and_receipt(self):
        root=tk.Tk(); app=DemoApp(root)
        try:
            root.update(); app.table.selection_set('demo-1'); app.preview()
            self.assertIn('DESMARCADA',app.detail.get('1.0','end'))
            app.prepare()
            with patch('registrador_rus.app.main.messagebox.askyesno',return_value=False): app.authorize()
            self.assertEqual(app.confirmed,set())
            with patch('registrador_rus.app.main.messagebox.askyesno',return_value=True): app.authorize()
            app.execute(); app.refresh()
            self.assertEqual(app.journal.state('demo-1'),'COMPROBADA')
            app.table.selection_set('demo-2'); app.preview()
            self.assertIn('MARCADA',app.detail.get('1.0','end'))
            app.limit.set('1'); app.configure()
            self.assertEqual(app.journal.schedule_eligibility().reason,'ESPERAR_CUPO')
        finally: app.close()

    def test_review_dialog_integrated_does_not_authorize(self):
        from registrador_rus.app.batch_review import BatchSelection
        from registrador_rus.app.batch_store import selection_digest
        from registrador_rus.app.journal import JournalError
        root=tk.Tk(); app=DemoApp(root)
        try:
            root.update()
            dialog=app.review_batch(); root.update()
            operations=tuple(app.ops.values())
            self.assertEqual(dialog.digest,BatchSelection(operations).digest)
            self.assertEqual(dialog.digest,selection_digest(operations))
            self.assertIn(operations[-1].approved_text,dialog.text.get('1.0','end'))
            dialog.accepted.set(True); dialog._toggle(); dialog.confirm_button.invoke()
            self.assertIn('8 operaciones',app.batch_status.get())
            self.assertEqual(app.journal.db.execute('SELECT COUNT(*) FROM simulated_batch_reviews').fetchone()[0],1)
            self.assertEqual(app.journal.db.execute('SELECT COUNT(*) FROM authorizations').fetchone()[0],0)
            app.table.selection_set('demo-1'); app.prepare()
            with self.assertRaises(JournalError):
                app.execute()
            self.assertEqual(app.journal.attempt_count(),0)
        finally: app.close()
