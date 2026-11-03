"""Ventana real con datos ficticios. Se omite sin pantalla (en Linux: xvfb-run)."""

import os
import sys
import time

import pytest

from conftest import make_xlsx

tk = pytest.importorskip("tkinter")
if sys.platform != "win32" and not os.environ.get("DISPLAY"):
    pytest.skip("Sin pantalla para Tk.", allow_module_level=True)

from anonimizador_csmp import gui  # noqa: E402


@pytest.fixture(scope="session")
def raiz():
    # Una sola Tk por sesión: en Windows, crear muchas Tk() seguidas falla a veces al cargar tk.tcl.
    try:
        root = tk.Tk()
    except tk.TclError:
        # Algunos Python de los runners de Windows no traen Tcl/Tk utilizable (init.tcl ausente).
        pytest.skip("Tk no disponible en este Python")
    root.withdraw()
    yield root
    root.destroy()


@pytest.fixture
def app(raiz):
    root = tk.Toplevel(raiz)
    window = gui.App(root)
    yield window
    root.destroy()


def esperar(app):
    deadline = time.monotonic() + 30
    while app.busy and time.monotonic() < deadline:
        app.root.update()
        time.sleep(0.05)


def test_sin_entradas_avisa(app):
    app.start()
    assert "al menos un archivo" in app.status.get()
    assert app.window.banner.visible
    assert app.go.instate(["disabled"])


def test_proceso_completo_en_hilo(app, tmp_path):
    app.inputs = [make_xlsx(tmp_path / "a.xlsx", [["RIT", "NOMBRE"], ["P-1-2026", "Ana Díaz"]])]
    app.out_dir.set(str(tmp_path / "salida"))
    assert "1 entrada · copia en la carpeta elegida" == app.window.footer.summary.get()
    assert app.go.instate(["!disabled"])
    app.start()
    esperar(app)
    assert "1 copia(s) creada(s)" in app.status.get()
    assert list((tmp_path / "salida").glob("*.xlsx"))
    assert app.list.done == 1 and "INFORME DE ANONIMIZACIÓN" in app.report


def test_dias_invalidos(app):
    app.inputs = ["x"]
    app.shift.set("abc")
    app.start()
    assert "número entero" in app.status.get()


def test_modo_desde_la_barra(app):
    app.modes.set(["conservador"])
    app.modes._changed()
    assert app.mode.get() == "conservador"


def test_desplazar_fechas_con_valor_invalido_no_falla(app, monkeypatch):
    app.shift.set("abc")
    vistos = []
    monkeypatch.setattr(gui.simpledialog, "askinteger", lambda *a, **k: vistos.append(k["initialvalue"]) or 7)
    app.ask_shift()
    assert vistos == [0] and app.shift.get() == "7"
