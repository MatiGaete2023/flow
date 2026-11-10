"""Ventana del Descargador con fichas, una acción principal y retomar sin buscar. Datos ficticios."""

import json
from pathlib import Path
import tempfile
import tkinter as tk
import unittest

from gui_test_isolation import isolated_gui

from descargador import Application
from test_flujos import catalog
from test_lotes import dispose_interface


class FichasTests(unittest.TestCase):
    def ventana(self):
        root = tk.Tk()
        root.withdraw()
        app = Application(root, start_worker=False)
        app.process_event('connected', 'Chrome conectado.')
        app.process_event('catalog', catalog('seguimiento'))
        return root, app

    @isolated_gui
    def test_fichas_y_estado_van_juntos(self):
        root, app = self.ventana()
        try:
            self.assertEqual(set(app.trib_chips.get()), set(app.catalog['tribunales']))
            app.trib_chips.set(['49'])
            app._chips_tribunals()
            self.assertEqual(app.selected_tribunals, ('49',))
            app.tab_chips.set(['Espera'])
            app._chips_tab()
            self.assertEqual(app.tab.get(), 'Espera')
            self.assertFalse(app.form.rows['filtro'][1].winfo_manager())
            app.period_chips.set(['Por fecha'])
            app._chips_period()
            self.assertTrue(app.use_dates.get() and app.form.rows['rango'][1].winfo_manager())
            app.from_date.set('31/02/2026')
            app.refresh()
            self.assertEqual(app.date_error.cget('text'), 'Fecha DD/MM/AAAA')
        finally:
            dispose_interface(app, root)

    @isolated_gui
    def test_una_accion_principal_segun_la_consulta(self):
        root, app = self.ventana()
        try:
            self.assertEqual(app.pipeline_button.cget('text'), 'Consultar y crear paquete')
            llamadas = []
            app.start_pipeline = lambda: llamadas.append('paquete')
            app.start_download = lambda: llamadas.append('descarga')
            app.primary_action()
            app.screen.set('Seguimiento de litigantes / órdenes')
            app.catalog = {**app.catalog, 'pantalla': 'litigantes'}
            app.refresh()
            self.assertEqual(app.pipeline_button.cget('text'), 'Descargar')
            app.primary_action()
            self.assertEqual(llamadas, ['paquete', 'descarga'])
            # Las acciones secundarias están en «Más» con el estado de su botón.
            app._sync_menu()
            index = dict((name, i) for i, name in app.menu_actions)['package_button']
            self.assertEqual(app.topbar.menu.entrycget(index, 'state'), 'disabled')
        finally:
            dispose_interface(app, root)

    @isolated_gui
    def test_ofrece_retomar_un_paquete_cortado(self):
        with tempfile.TemporaryDirectory() as temp:
            root, app = self.ventana()
            try:
                app.dest.set(temp)
                self.assertEqual(app.offer_resume(), [])
                for nombre, estado in (
                    ('Paquete CSMP 2026-11-01 08-00-00-0', 'COMPLETO'),
                    ('Paquete CSMP 2026-11-02 08-00-00-0', 'INCOMPLETO'),
                ):
                    control = Path(temp) / nombre / 'respaldo' / 'proceso.json'
                    control.parent.mkdir(parents=True)
                    control.write_text(json.dumps({'estado': estado, 'fase': 'bitacoras', 'carpetas': {}}))
                encontrados = app.offer_resume()
                self.assertEqual([c.name for c, _ in encontrados], ['Paquete CSMP 2026-11-02 08-00-00-0'])
                self.assertTrue(app.banner.visible and app.banner.kind == 'warn')
                self.assertIn('bitácoras', app.banner.text.get())
            finally:
                dispose_interface(app, root)

    @isolated_gui
    def test_sin_conexion_solo_la_conexion_guiada(self):
        root = tk.Tk()
        root.withdraw()
        app = Application(root, start_worker=False)
        try:
            root.update()
            self.assertTrue(app.connect_box.winfo_manager())
            self.assertFalse(app.form.winfo_manager())
            app.copy_code()
            self.assertEqual(app.step_marks[2].cget('text'), '✓')
            app.process_event('query', {'index': 2, 'total': 4, 'label': 'x'})
            self.assertTrue(root.title().startswith('2 de 4 · Descargador SITFA'))
            app.process_event('idle', None)
            self.assertEqual(root.title(), app.title)
        finally:
            dispose_interface(app, root)


if __name__ == '__main__':
    unittest.main()
