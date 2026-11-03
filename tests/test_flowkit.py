"""El kit de interfaz tiene una sola fuente y cada producto lleva una copia idéntica."""

import os
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
import sync_flowkit  # noqa: E402


def test_copias_identicas():
    assert sync_flowkit.main(['--check']) == 0


@pytest.mark.skipif(sys.platform != 'win32' and not os.environ.get('DISPLAY'), reason='Sin pantalla para Tk.')
def test_componentes_basicos():
    tk = pytest.importorskip('tkinter')
    sys.path.insert(0, str(ROOT / 'ui'))
    import flowkit as ui

    root = tk.Tk()
    try:
        window = ui.Window(root, 'Prueba', 'Hacer', lambda: None)
        window.top.set_state('Conectado')
        chips = ui.Chips(window.body, [('a', 'A'), ('b', 'B')], command=lambda: None)
        chips.pack()
        chips.set(['a'])
        assert chips.get() == ('a',)
        chips.toggle_all()
        assert chips.get() == ('a', 'b')
        chips.toggle_all()
        assert chips.get() == ()
        single = ui.Chips(window.body, [('x', 'X'), ('y', 'Y')], multi=False)
        assert single.get() == ('x',)
        progress = ui.ProgressList(window.body)
        progress.plan([(1, 'uno'), (2, 'dos')])
        progress.set(1, 'ok', '3')
        assert progress.done == 1
        window.footer.set('2 consultas', enabled=False)
        assert window.footer.primary.instate(['disabled'])
        window.banner.show('Aviso')
        assert window.banner.visible
        window.banner.hide()
        assert not window.banner.visible
        root.update()
    finally:
        root.destroy()


@pytest.mark.skipif(sys.platform != 'win32' and not os.environ.get('DISPLAY'), reason='Sin pantalla para Tk.')
def test_ficha_unica_ignora_un_valor_que_ya_no_existe():
    tk = pytest.importorskip('tkinter')
    sys.path.insert(0, str(ROOT / 'ui'))
    import flowkit as ui

    root = tk.Tk()
    try:
        chips = ui.Chips(root, [('a', 'A'), ('b', 'B')], multi=False)
        chips.set_options([('a', 'A'), ('b', 'B')], selected=['ya-no-existe', 'b'])
        assert chips.get() == ('b',)
        chips.set_options([('c', 'C')], selected=['b'])
        assert chips.get() == ('c',)
    finally:
        root.destroy()
