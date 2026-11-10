"""Ventana Tk con datos ficticios. Se omite si no hay tkinter o pantalla (en Linux: xvfb-run)."""

from datetime import date
import json
import os
from pathlib import Path
import sys
import time

import pytest

tk = pytest.importorskip('tkinter')
if sys.platform != 'win32' and not os.environ.get('DISPLAY'):
    pytest.skip('Sin pantalla para Tk.', allow_module_level=True)

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import Meta4  # noqa: E402
from test_meta4_bitacoras import cuatro_lotes, diary, lote_html, save_diaries  # noqa: E402


@pytest.fixture(scope='session')
def raiz():
    # Una sola Tk por sesión: en Windows, crear muchas Tk() seguidas falla a veces al cargar tk.tcl.
    root = tk.Tk()
    root.withdraw()
    yield root
    root.destroy()


@pytest.fixture
def ventana(monkeypatch, raiz, tmp_path):
    shown = []
    for name in ('showinfo', 'showwarning', 'showerror'):
        monkeypatch.setattr(Meta4.messagebox, name, lambda *a, _n=name, **k: shown.append((_n, a)))
    root = tk.Toplevel(raiz)
    app = Meta4.App(root, prefs_path=tmp_path / 'prefs.json')
    yield app, shown
    if not app.busy:
        app.close()


def esperar(app):
    deadline = time.time() + 60
    while (app.busy or app.events.qsize()) and time.time() < deadline:
        app.root.update()
        app.poll()
        time.sleep(0.02)


def lote(tmp_path):
    source = tmp_path / 'registros'
    source.mkdir()
    lote_html(source, [{}])
    data = json.loads((source / 'resumen.json').read_text())
    data['consultas'][0]['seleccion']['tribunal_nombre'] = 'Juzgado de Familia de Laja'
    (source / 'resumen.json').write_text(json.dumps(data))
    save_diaries(source, [diary(rit='X-1-2026')])
    path = source / 'bitacoras_meta4.json'
    path.write_text(json.dumps(dict(json.loads(path.read_text()), estado='COMPLETA')))
    return source


def test_requires_source_and_valid_date(ventana):
    app, shown = ventana
    app.start()
    assert app.window.banner.text.get() == 'Agrega al menos un lote de registros.'
    app.sources = ['x']
    app.cutoff.set('2026-13-01')
    app.start()
    assert 'DD/MM/AAAA' in app.window.banner.text.get()
    assert not shown


def test_generates_and_lists_lot_tribunals(ventana, tmp_path, monkeypatch):
    app, shown = ventana
    source = lote(tmp_path)
    monkeypatch.setattr(Meta4.filedialog, 'askdirectory', lambda **k: str(source))
    app.pick_source()
    assert tuple(app.courts.buttons) == ('LAJA',) and app.courts.get() == ('LAJA',)
    assert app.lots.item('0', 'values')[1] == '1 lote · bitácoras ✓'
    app.cutoff.set(date(2026, 10, 16).isoformat())
    app.folder.set(str(tmp_path / 'salidas'))
    app.start()
    esperar(app)
    assert not app.warnings, app.warnings
    assert 'todos los tribunales' in app.status.get()
    assert (Path(app.output) / 'META 4 - actualizado 2026-10.xlsx').is_file()
    app.courts.toggle_all()
    app.start()
    assert 'al menos un tribunal' in app.window.banner.text.get()


def test_generation_error_is_shown_without_output(ventana, tmp_path):
    app, shown = ventana
    empty = tmp_path / 'vacio'
    empty.mkdir()
    app.set_sources([empty])
    app.cutoff.set('2026-10-16')
    app.folder.set(str(tmp_path / 'salidas'))
    app.start()
    esperar(app)
    assert 'Faltan bitácoras' in app.window.banner.text.get()
    assert app.output is None and not shown


def test_adds_several_lots_and_generates_one_meta4(ventana, tmp_path, monkeypatch):
    app, shown = ventana
    lotes, diaries = cuatro_lotes(tmp_path / 'descargas')
    for path in lotes + [lotes[0]]:
        monkeypatch.setattr(Meta4.filedialog, 'askdirectory', lambda _p=path, **k: str(_p))
        app.pick_source()
    assert len(app.lots.get_children()) == 4 and '4 lote(s)' in app.lots_info
    app.lots.selection_set('3')
    app.remove_sources()
    assert len(app.lots.get_children()) == 3
    app.set_sources(lotes)
    app.diaries.set(str(diaries))
    app.cutoff.set('2026-10-16')
    app.folder.set(str(tmp_path / 'salidas'))
    app.start()
    esperar(app)
    # Las bitácoras ficticias no declaran lectura COMPLETA: el aviso es el esperado.
    assert 'La lectura de bitácoras no quedó COMPLETA.' in app.warnings
    assert app.notices.tree.get_children() and not shown
    assert 'Generados 5 registros de 4 lote(s)' in app.status.get()
    assert (Path(app.output) / 'META 4 - actualizado 2026-10.xlsx').is_file()


def test_regenerates_from_reviewed_registros(ventana, tmp_path, monkeypatch):
    app, shown = ventana
    lotes, diaries = cuatro_lotes(tmp_path / 'descargas')
    first = tmp_path / 'salidas' / 'Resultado META4 1'
    Meta4.generar(lotes, first, date(2026, 10, 16), bitacoras=diaries)
    monkeypatch.setattr(Meta4.filedialog, 'askopenfilename', lambda **k: str(first / 'Registros.xlsx'))
    app.folder.set(str(tmp_path / 'salidas'))
    app.start_regenerate()
    esperar(app)
    assert 'Regenerado desde Registros revisado. Generados 5 registros' in app.status.get(), shown[-1]
    assert Path(app.output).name.endswith(' regenerado')
    assert (Path(app.output) / 'META 4 - actualizado 2026-10.xlsx').is_file()


def test_output_folder_name_is_short_and_unique(tmp_path):
    first = Meta4.nueva_salida(tmp_path, ' regenerado')
    assert len(first.name) == len('Resultado META4 2026-10-29 15-09-27 regenerado')
    first.mkdir()
    assert Meta4.nueva_salida(tmp_path, ' regenerado').name in (
        first.name + ' (2)',
        Meta4.nueva_salida(tmp_path, ' regenerado').name,
    )


def test_escribir_la_fecha_no_recorre_las_carpetas(ventana, tmp_path, monkeypatch):
    app, _ = ventana
    lotes, _ = cuatro_lotes(tmp_path / 'descargas')
    app.set_sources(lotes)
    llamadas = []
    monkeypatch.setattr(Meta4, 'buscar_archivos', lambda *a: llamadas.append(a) or [])
    for valor in ('2026-10-1', '2026-10-16'):
        app.cutoff.set(valor)
    assert not llamadas and '4 lotes' in app.window.footer.summary.get()


def test_corte_estado_vacio_y_ultimo_lote(ventana, tmp_path, monkeypatch):
    app, shown = ventana
    # Sin lotes: la fila dice qué hacer, con el botón dentro.
    app.root.update()
    assert app.empty.winfo_manager() and not app.lots_tree.winfo_manager()
    # Corte con formato del resto de Flow y aviso en el lugar.
    assert app.cutoff.get() == date.today().strftime('%d/%m/%Y')
    app.cutoff.set('31/02/2026')
    assert app.cutoff_error.cget('text') == 'Fecha DD/MM/AAAA'
    app.cutoff.set('01/01/2999')
    assert 'futuro' in app.cutoff_error.cget('text')
    app.cutoff.set('16/10/2026')
    assert app.cutoff_error.cget('text') == ''
    # El último lote agregado se ofrece la próxima vez.
    source = lote(tmp_path)
    monkeypatch.setattr(Meta4.filedialog, 'askdirectory', lambda **k: str(source))
    app.pick_source()
    assert app.lots_tree.winfo_manager() or not app.empty.winfo_manager()
    otra = Meta4.App(Meta4.tk.Toplevel(app.root), prefs_path=tmp_path / 'prefs.json')
    try:
        assert otra.last_lot() == str(source) and 'registros' in otra.last_button.cget('text')
        otra.use_last()
        assert otra.sources == [str(source)]
    finally:
        otra.root.destroy()
    # Mientras genera, las acciones del menú que cambian los lotes se ven deshabilitadas.
    app.busy = True
    app.refresh()
    assert app.window.top.menu.entrycget(0, 'state') == 'disabled'
    app.busy = False
    app.refresh()
