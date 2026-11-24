from datetime import datetime, timezone
from pathlib import Path
import sqlite3
from tempfile import TemporaryDirectory
import tkinter as tk
import unittest
from unittest.mock import patch
from dataclasses import replace
from registrador_rus.app.batch_dialog import BatchReviewDialog
from registrador_rus.app.batch_store import SimulationBatchStore
from test_batch_store import ops


class BatchDialogTests(unittest.TestCase):
    def setUp(self):
        self.root = tk.Tk(); self.root.withdraw()
        self.tmp = TemporaryDirectory()
        self.db = sqlite3.connect(Path(self.tmp.name)/'demo.sqlite', isolation_level=None)
        self.store = SimulationBatchStore(self.db); self.store.initialize()
        self.current = ops()

    def tearDown(self):
        self.root.destroy(); self.db.close(); self.tmp.cleanup()

    def open_dialog(self):
        def confirm(digest):
            self.db.execute('BEGIN IMMEDIATE')
            try:
                self.store.record('demo-review', self.current, displayed_digest=digest, actor='demo', confirmed=True,
                                  now=datetime(2026,11,24,15,tzinfo=timezone.utc))
                self.db.commit()
            except BaseException:
                self.db.rollback(); raise
        dialog = BatchReviewDialog(self.root, self.current, confirm)
        self.root.update()
        return dialog

    def test_confirmation_and_persistence(self):
        dialog = self.open_dialog()
        self.assertIn(self.current[0].approved_text, dialog.text.get('1.0','end'))
        self.assertIn('AL_TRIBUNAL', dialog.text.get('1.0','end'))
        self.assertEqual(str(dialog.confirm_button['state']), 'disabled')
        dialog.confirm()
        self.assertEqual(self.db.execute('SELECT COUNT(*) FROM simulated_batch_reviews').fetchone()[0], 0)
        dialog.accepted.set(True); dialog._toggle(); dialog.confirm_button.invoke()
        self.assertTrue(self.store.require_current('demo-review', self.current))

    def test_changed_selection_and_cancel(self):
        dialog = self.open_dialog()
        self.current = (replace(self.current[0],destination='AL_TRIBUNAL'), self.current[1])
        dialog.accepted.set(True); dialog._toggle()
        with patch('registrador_rus.app.batch_dialog.messagebox.showwarning') as warning:
            dialog.confirm_button.invoke(); warning.assert_called_once()
        self.assertTrue(dialog.window.winfo_exists())
        dialog.window.destroy()
        self.assertEqual(self.db.execute('SELECT COUNT(*) FROM simulated_batch_reviews').fetchone()[0], 0)
