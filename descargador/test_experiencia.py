"""Interfaz experimental: resultados offline, filtros y selección conservada."""
from pathlib import Path
import tempfile
import time
import tkinter as tk
import unittest

from descargador import Application,TribunalPicker
from test_lotes import CATALOG,dispose_interface
from test_resultados import fixture
from ventana_resultados import ResultsWindow


class ExperienceTests(unittest.TestCase):
    def test_search_preserves_hidden_choices_and_cancel(self):
        root=tk.Tk();root.withdraw();chosen=[]
        try:
            picker=TribunalPicker(root,CATALOG['tribunales'],('49',),chosen.append)
            picker.search.set('Laja');picker.set_visible(True)
            self.assertTrue(picker.choices['49'].get());self.assertTrue(picker.choices['113'].get())
            picker.destroy();self.assertEqual(chosen,[])
        finally:root.destroy()
    def test_offline_results_load_and_generate(self):
        with tempfile.TemporaryDirectory() as temp:
            root=tk.Tk();root.withdraw();app=Application(root,start_worker=False)
            try:
                folder=fixture(Path(temp)/'lote');app.dest.set(temp);app.last_folder=folder
                popup=ResultsWindow(app);popup.withdraw()
                deadline=time.monotonic()+5
                while popup.busy and time.monotonic()<deadline:root.update();time.sleep(.02)
                self.assertFalse(popup.busy);self.assertIsNotNone(popup.lot);self.assertEqual(len(popup.detail.get_children()),1)
                popup.busy=True;app.close_app();self.assertFalse(app.quitting);popup.busy=False
                from resultados import export_book
                popup.run(lambda:export_book([popup.lot],csmp=True))
                deadline=time.monotonic()+5
                while popup.busy and time.monotonic()<deadline:root.update();time.sleep(.02)
                self.assertFalse(popup.busy);self.assertEqual(len(list(folder.glob('Para CSMP*.xlsx'))),1)
                popup.destroy()
            finally:dispose_interface(app,root)


if __name__=='__main__':unittest.main()
