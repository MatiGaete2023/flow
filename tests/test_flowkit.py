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


@pytest.mark.skipif(sys.platform != 'win32' and not os.environ.get('DISPLAY'), reason='Sin pantalla para Tk.')
def test_piezas_de_experiencia(tmp_path):
    tk = pytest.importorskip('tkinter')
    sys.path.insert(0, str(ROOT / 'ui'))
    import flowkit as ui

    root = tk.Tk()
    try:
        pulsado = []
        window = ui.Window(root, 'Prueba', 'Hacer', lambda: pulsado.append(1))
        # Ficha «Todos»: marca y desmarca todas, y refleja la selección.
        chips = ui.Chips(window.body, [('a', 'A'), ('b', 'B')], all_label='Todos')
        chips.pack()
        chips.all_var.set(True)
        chips._toggle_from_all()
        assert chips.get() == ('a', 'b') and chips.all_var.get()
        chips.set(['a'])
        assert not chips.all_var.get()
        # Fichas planas: sin relieve hundido al marcarlas.
        assert ('selected', 'flat') in [tuple(x) for x in ttk_map(root, 'Chip.Toolbutton', 'relief')]
        # Aviso con tono y cierre.
        window.banner.show('Listo', kind='ok')
        assert window.banner.kind == 'ok' and window.banner.visible
        window.banner.close.invoke()
        assert not window.banner.visible
        # Menú: acciones bloqueadas durante el trabajo y nota de versión deshabilitada.
        window.top.add('Agregar', lambda: None, busy_lock=True)
        window.top.add_note('Versión 1')
        window.top.lock(True)
        assert window.top.menu.entrycget(0, 'state') == 'disabled'
        assert window.top.menu.entrycget('end', 'state') == 'disabled'
        window.top.lock(False)
        assert window.top.menu.entrycget(0, 'state') == 'normal'
        # Avance en el título.
        window.set_progress('3 de 8')
        assert root.title() == '3 de 8 · Prueba'
        window.set_progress()
        assert root.title() == 'Prueba'
        # Enter pulsa la acción principal; Esc llama a on_escape.
        escapes = []
        window.on_escape = lambda: escapes.append(1)
        # Se llama al manejador: Windows no entrega eventos de teclado a una ventana sin foco.
        from types import SimpleNamespace

        window._enter(SimpleNamespace(widget=window.body))
        window._enter(SimpleNamespace(widget=ttk_entry(window.body)))
        window.on_escape()
        assert pulsado == [1] and escapes == [1]
        assert '<Key-Escape>' in root.bind() and '<Key-F1>' in root.bind()
        # Manuales: se buscan junto al programa; si falta, aviso en vez de error.
        (tmp_path / 'manuales').mkdir()
        (tmp_path / 'manuales' / 'Guia.pdf').write_bytes(b'%PDF')
        assert ui.find_doc('Guia.pdf', tmp_path / 'app.py') == tmp_path / 'manuales' / 'Guia.pdf'
        window.open_doc('No_existe.pdf', tmp_path / 'app.py')
        assert window.banner.kind == 'warn'
        # Fechas: DD/MM/AAAA y AAAA-MM-DD.
        assert ui.parse_date('16/10/2026') == ui.parse_date('2026-10-16')
        with pytest.raises(ValueError, match='DD/MM/AAAA'):
            ui.parse_date('31/02/2026')
    finally:
        root.destroy()


def ttk_entry(parent):
    from tkinter import ttk

    return ttk.Entry(parent)


def ttk_map(root, style, option):
    from tkinter import ttk

    return ttk.Style(root).map(style, option)
