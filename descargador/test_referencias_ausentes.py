"""Regresión del ZIP bloqueado por un control antiguo invisible; datos ficticios."""

from pathlib import Path
import tempfile
import time
import tkinter as tk
import unittest
from unittest.mock import patch

import pytest
from archivo_local import discover, remember, diary_references
from preferencias import Preferences
from test_reutilizacion_local import diaries, bound_lot
from gui_test_isolation import isolated_gui
from descargador import Application
from test_lotes import dispose_interface
from ventana_resultados import ResultsWindow


def test_missing_and_invalid_controls_remain_visible_without_rewriting_history(tmp_path):
    valid = diaries(tmp_path / 'available', 2)
    missing = diaries(tmp_path / 'moved', 1)
    invalid = diaries(tmp_path / 'invalid', 1)
    prefs = Preferences(tmp_path / 'prefs.json')
    for p in (valid, missing, invalid):
        remember(prefs, p.parent)
    original = (tmp_path / 'prefs.json').read_bytes()
    missing.rename(tmp_path / 'moved/control-renamed.json')
    invalid.write_text('{', encoding='utf-8')
    entries = {folder.name: (label, state) for folder, label, state, _ in discover(tmp_path / 'elsewhere', prefs)}
    assert entries['moved'] == ('Bitácoras', 'NO_DISPONIBLE')
    assert entries['invalid'] == ('Bitácoras', 'ERROR_CONTROL')
    assert entries['available'] == ('Bitácoras', 'COMPLETA')
    assert (tmp_path / 'prefs.json').read_bytes() == original
    assert len(diary_references(prefs, [missing])) == 2


def test_shared_importer_explains_missing_control(tmp_path):
    from nurus.capture.diaries import import_lotes

    with pytest.raises(ValueError, match='Recupera bitacoras.json'):
        import_lotes(tmp_path / 'missing/bitacoras.json')


class MissingReferenceInterfaceTests(unittest.TestCase):
    def settle(self, root, window):
        deadline = time.monotonic() + 15
        while window.busy and time.monotonic() < deadline:
            root.update()
            time.sleep(0.02)
        self.assertFalse(window.busy, window.status.get())

    @isolated_gui
    def test_export_requires_explicit_exclusion_then_preserves_valid_diaries(self):
        from nurus.capture import read_capture

        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            valid = diaries(base / 'valid', 64)
            missing = [base / f'old_{i:02d}' / 'bitacoras.json' for i in range(20)]
            lot = bound_lot(base / 'sheets')
            root = tk.Tk()
            root.withdraw()
            app = Application(root, start_worker=False)
            try:
                app.prefs = Preferences(base / 'prefs.json')
                remember(app.prefs, valid.parent)
                app.prefs.data['historial_bitacoras'].extend(str(p) for p in missing)
                app.prefs.save()
                prefs_before = app.prefs.path.read_bytes()
                originals = {p: p.read_bytes() for p in valid.parent.iterdir()}
                app.dest.set(str(base / 'different'))
                app.last_folder = lot.folder
                win = ResultsWindow(app)
                win.withdraw()
                self.settle(root, win)
                self.assertIn('20 referencia(s) NO_DISPONIBLE', win.collection.get())
                self.assertTrue(
                    any(win.history.item(i, 'values')[2] == 'NO_DISPONIBLE' for i in win.history.get_children())
                )
                target = base / 'package.zip'
                with patch('ventana_resultados.filedialog.asksaveasfilename', return_value=str(target)) as save:
                    win.capture_package()
                    self.assertIsNotNone(win.missing_window)
                    save.assert_not_called()
                    win.missing_window.close()
                    self.assertFalse(target.exists())
                    self.assertTrue(all(p.resolve() in win.package_diaries() for p in missing))
                    win.capture_package()
                    review = win.missing_window
                    review.exclude_all()
                    review.finish()
                    self.settle(root, win)
                capture = read_capture(target)
                self.assertEqual(len(capture.rows('registros')), 2)
                self.assertEqual(len(capture.rows('bitacoras')), 2)
                self.assertIn('Paquete creado:', win.status.get())
                self.assertEqual(app.prefs.path.read_bytes(), prefs_before)
                self.assertEqual({p: p.read_bytes() for p in valid.parent.iterdir()}, originals)
                self.assertEqual(win.package_folders, [lot.folder])
                win.destroy()
            finally:
                dispose_interface(app, root)

    @isolated_gui
    def test_relocating_old_reference_keeps_history_and_exports_recovered_text(self):
        from nurus.capture import read_capture

        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            manifest = diaries(base / 'moved', 2)
            lot = bound_lot(base / 'sheets')
            root = tk.Tk()
            root.withdraw()
            app = Application(root, start_worker=False)
            try:
                app.prefs = Preferences(base / 'prefs.json')
                remember(app.prefs, manifest.parent)
                relocated = base / 'new-location'
                manifest.parent.rename(relocated)
                app.dest.set(str(base / 'different'))
                app.last_folder = lot.folder
                win = ResultsWindow(app)
                win.withdraw()
                self.settle(root, win)
                win.capture_package()
                review = win.missing_window
                review.tree.selection_set('0')
                with patch(
                    'ventana_resultados.filedialog.askopenfilename', return_value=str(relocated / 'bitacoras.json')
                ):
                    review.locate()
                self.assertEqual(win.missing_package_diaries(), [])
                self.assertIn(str(manifest.resolve()), app.prefs.data['historial_bitacoras'])
                target = base / 'recovered.zip'
                with patch('ventana_resultados.filedialog.asksaveasfilename', return_value=str(target)):
                    review.finish()
                self.settle(root, win)
                capture = read_capture(target)
                self.assertEqual(len(capture.rows('bitacoras')), 2)
                self.assertTrue((relocated / 'bitacora_0.html').is_file())
                win.destroy()
            finally:
                dispose_interface(app, root)

    @isolated_gui
    def test_multiselection_excludes_missing_references_and_excel_never_recurses(self):
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            lot = bound_lot(base / 'sheets')
            missing = [base / name / 'bitacoras.json' for name in ('old1', 'old2')]
            root = tk.Tk()
            root.withdraw()
            app = Application(root, start_worker=False)
            try:
                app.prefs = Preferences(base / 'prefs.json')
                app.prefs.data['historial_bitacoras'] = [str(p) for p in missing]
                app.dest.set(str(base / 'different'))
                app.last_folder = lot.folder
                win = ResultsWindow(app)
                win.withdraw()
                self.settle(root, win)
                ids = [i for i in win.history.get_children() if win.history.item(i, 'values')[1] == 'Bitácoras']
                win.history.selection_set(ids)
                win.remove_package_source()
                self.assertEqual(win.package_diaries(), [])
                self.assertEqual(win.package_folders, [lot.folder])
                win.folder = missing[0].parent
                win.loaded_kind = 'Bitácoras'
                with patch('ventana_resultados.filedialog.asksaveasfilename') as save:
                    win.diary_excel()
                    save.assert_not_called()
                self.assertIn('selecciona una descarga', win.status.get())
                win.destroy()
            finally:
                dispose_interface(app, root)

    @isolated_gui
    def test_restoring_original_path_does_not_exclude_the_recovered_control(self):
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            manifest = diaries(base / 'old', 1)
            lot = bound_lot(base / 'sheets')
            root = tk.Tk()
            root.withdraw()
            app = Application(root, start_worker=False)
            try:
                app.prefs = Preferences(base / 'prefs.json')
                remember(app.prefs, manifest.parent)
                moved = base / 'moved'
                manifest.parent.rename(moved)
                app.dest.set(str(base / 'different'))
                app.last_folder = lot.folder
                win = ResultsWindow(app)
                win.withdraw()
                self.settle(root, win)
                win.review_missing_diaries()
                review = win.missing_window
                review.tree.selection_set('0')
                moved.rename(manifest.parent)
                with patch('ventana_resultados.filedialog.askopenfilename', return_value=str(manifest)):
                    review.locate()
                self.assertEqual(win.package_diaries(), [manifest.resolve()])
                self.assertEqual(win.missing_package_diaries(), [])
                review.finish()
                win.destroy()
            finally:
                dispose_interface(app, root)
