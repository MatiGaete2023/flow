"""Ventana real con SITFA ficticio: conexión, carga automática de opciones, descarga y resultado."""

import os
import sys
import time
from pathlib import Path

import pytest

tk = pytest.importorskip('tkinter')
if sys.platform != 'win32' and not os.environ.get('DISPLAY'):
    pytest.skip('Sin pantalla para Tk.', allow_module_level=True)

ROOT = Path(__file__).resolve().parents[1]
sys.path[:0] = [str(ROOT / 'app'), str(ROOT / 'tests')]
import sitfa_lite  # noqa: E402
from sitfa_ficticio import FakeBridge  # noqa: E402


class Puente(FakeBridge):
    connection_code = '51234-0123456789abcdef0123456789abcdef'

    def __init__(self, emit):
        super().__init__('seguimiento', pages={('902', 'Espera', '1', None): 0})
        self.connected = False

    def call(self, command, payload=None, timeout=75):
        if command == 'catalog':
            self.screen = payload['screen']
        return super().call(command, payload, timeout)

    def close(self):
        pass


@pytest.fixture(scope='session')
def raiz():
    # Una sola Tk por sesión: en Windows, crear muchas Tk() seguidas falla a veces al cargar tk.tcl.
    root = tk.Tk()
    root.withdraw()
    yield root
    root.destroy()


@pytest.fixture
def app(tmp_path, raiz):
    root = tk.Toplevel(raiz)
    window = sitfa_lite.Application(root, bridge_factory=Puente, preferences_path=tmp_path / 'p.json')
    window.dest.set(str(tmp_path / 'descargas'))
    yield window
    root.destroy()


def esperar(app, condicion, segundos=30):
    deadline = time.monotonic() + segundos
    while not condicion() and time.monotonic() < deadline:
        app.root.update()
        time.sleep(0.02)
    assert condicion()


def test_sin_conexion_muestra_solo_conectar(app):
    app.root.update()
    assert app.connect_view.winfo_manager() and not app.form.winfo_manager()
    assert app.download_button.instate(['disabled'])
    assert app.window.top.state_text.get() == 'Sin conexión'


def test_flujo_completo_con_y_sin_pdf(app, tmp_path):
    app.bridge.connected = True
    esperar(app, lambda: app.catalog is not None)
    assert app.form.winfo_manager() and app.window.top.state_text.get() == 'Chrome conectado'
    app.tribunals.set(['901', '902'])
    app.tabs.set(['Espera'])
    app.modalities.set(['1'])
    app.refresh()
    assert app.window.footer.summary.get().startswith('2 consultas · 2 tribunales · sin filtro de fecha')
    assert not app.form.rows['informes'][1].winfo_ismapped()
    app.include.set([])
    app.toggle_empty_pdf()
    assert app.prefs.data['pdf_sin_registros'] is False
    app.primary()
    esperar(app, lambda: app.showing_results and not app.busy)
    assert 'sin registros' in app.window.footer.summary.get()
    values = [app.results.tree.item(i, 'values') for i in app.results.tree.get_children()]
    assert values[1][2] == 'sin registros · sin PDF' and values[0][2].endswith('registros')
    assert not list(app.last_folder.glob('*.pdf'))
    assert app.window.footer.primary.cget('text') == 'Nuevo lote'
    app.primary()
    assert app.form.winfo_manager()


def test_cambio_de_consulta_recarga_opciones(app):
    app.bridge.connected = True
    esperar(app, lambda: app.catalog is not None)
    app.side.select('litigantes', notify=True)
    esperar(app, lambda: app.catalog is not None and app.catalog['pantalla'] == 'litigantes')
    assert app.form.rows['Estado'][1].winfo_ismapped() and not app.form.rows['Pestañas'][1].winfo_ismapped()
    app.period.set(['Por fecha'])
    app.refresh()
    app.root.update()
    assert app.form.rows['rango'][1].winfo_ismapped()


def test_descargar_con_otra_operacion_en_curso_no_marca_un_lote(app):
    app.bridge.connected = True
    esperar(app, lambda: app.catalog is not None)
    app.tribunals.set(['901'])
    app.busy = True
    app.start()
    assert not app.running and app.commands.empty() and app.window.banner.visible
    app.busy = False


def test_refresh_no_reempaqueta_la_vista(app):
    app.bridge.connected = True
    esperar(app, lambda: app.catalog is not None)
    llamadas = []
    original = app.form.pack
    app.form.pack = lambda *a, **k: (llamadas.append(1), original(*a, **k))
    for _ in range(5):
        app.refresh()
    assert not llamadas
