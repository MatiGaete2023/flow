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


@pytest.fixture
def ventana(monkeypatch):
    shown = []
    for name in ('showinfo', 'showwarning', 'showerror'):
        monkeypatch.setattr(Meta4.messagebox, name, lambda *a, _n=name, **k: shown.append((_n, a)))
    root = tk.Tk()
    app = Meta4.App(root)
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
    assert shown[-1][1][1] == 'Agrega al menos un lote de registros.'
    app.sources = ['x']
    app.cutoff.set('2026-13-01')
    app.start()
    assert 'AAAA-MM-DD' in shown[-1][1][1]


def test_generates_and_lists_lot_tribunals(ventana, tmp_path, monkeypatch):
    app, shown = ventana
    source = lote(tmp_path)
    monkeypatch.setattr(Meta4.filedialog, 'askdirectory', lambda **k: str(source))
    app.pick_source()
    assert app.courts.get(0, 'end') == ('LAJA',)
    app.cutoff.set(date(2026, 10, 16).isoformat())
    app.folder.set(str(tmp_path / 'salidas'))
    app.courts.selection_set(0)
    app.start()
    esperar(app)
    assert shown[-1][0] == 'showinfo', shown[-1]
    assert 'PARCIAL: LAJA' in app.status.get()
    assert (Path(app.output) / 'META 4 - actualizado 2026-10 - parcial.xlsx').is_file()


def test_generation_error_is_shown_without_output(ventana, tmp_path):
    app, shown = ventana
    empty = tmp_path / 'vacio'
    empty.mkdir()
    app.set_sources([empty])
    app.cutoff.set('2026-10-16')
    app.folder.set(str(tmp_path / 'salidas'))
    app.start()
    esperar(app)
    assert shown[-1][0] == 'showerror' and 'Faltan bitácoras' in shown[-1][1][1]
    assert app.output is None


def test_adds_several_lots_and_generates_one_meta4(ventana, tmp_path, monkeypatch):
    app, shown = ventana
    lotes, diaries = cuatro_lotes(tmp_path / 'descargas')
    for path in lotes + [lotes[0]]:
        monkeypatch.setattr(Meta4.filedialog, 'askdirectory', lambda _p=path, **k: str(_p))
        app.pick_source()
    assert app.lots.size() == 4 and '4 lote(s)' in app.lots_info.get()
    app.lots.selection_set(3)
    app.remove_sources()
    assert app.lots.size() == 3
    app.set_sources(lotes)
    app.diaries.set(str(diaries))
    app.cutoff.set('2026-10-16')
    app.folder.set(str(tmp_path / 'salidas'))
    app.start()
    esperar(app)
    # Las bitácoras ficticias no declaran lectura COMPLETA: el aviso es el esperado.
    assert shown[-1][0] == 'showwarning' and 'no quedó COMPLETA' in shown[-1][1][1], shown[-1]
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
