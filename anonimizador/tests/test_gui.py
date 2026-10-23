"""Humo de la ventana con un Tkinter simulado (la prueba con pantalla real está en test_engine)."""

import importlib
import sys
import time
from types import ModuleType
from unittest.mock import MagicMock

import pytest

from conftest import make_xlsx


class Var:
    def __init__(self, value=None, **_):
        self.value = value

    def get(self):
        return self.value

    def set(self, value):
        self.value = value


@pytest.fixture
def gui(monkeypatch):
    tk, ttk, dialog = ModuleType("tkinter"), ModuleType("tkinter.ttk"), ModuleType("tkinter.filedialog")
    tk.StringVar = tk.BooleanVar = Var
    tk.Tk, tk.Listbox, tk.Text = MagicMock(), MagicMock(), MagicMock()
    for name in ("Frame", "Label", "Button", "Radiobutton", "Checkbutton", "Entry", "LabelFrame", "Progressbar"):
        setattr(ttk, name, MagicMock())
    dialog.askopenfilenames = dialog.askdirectory = MagicMock(return_value=())
    tk.ttk, tk.filedialog = ttk, dialog
    monkeypatch.setitem(sys.modules, "tkinter", tk)
    monkeypatch.setitem(sys.modules, "tkinter.ttk", ttk)
    monkeypatch.setitem(sys.modules, "tkinter.filedialog", dialog)
    sys.modules.pop("anonimizador_csmp.gui", None)
    module = importlib.import_module("anonimizador_csmp.gui")
    yield module
    sys.modules.pop("anonimizador_csmp.gui", None)


def test_sin_entradas_avisa(gui):
    app = gui.App(MagicMock())
    app.start()
    assert "al menos un archivo" in app.status.get()


def test_proceso_completo_en_hilo(gui, tmp_path):
    app = gui.App(MagicMock())
    app.inputs = [make_xlsx(tmp_path / "a.xlsx", [["RIT", "NOMBRE"], ["P-1-2026", "Ana Díaz"]])]
    app.out_dir.set(str(tmp_path / "salida"))
    app.start()
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        if not app.events.empty():
            break
        time.sleep(0.05)
    app.root.after = MagicMock()
    while not app.events.empty():
        app.poll()
    assert "1 copia(s) creada(s)" in app.status.get()
    assert list((tmp_path / "salida").glob("*.xlsx"))


def test_dias_invalidos(gui):
    app = gui.App(MagicMock())
    app.inputs = ["x"]
    app.shift.set("abc")
    app.start()
    assert "número entero" in app.status.get()
