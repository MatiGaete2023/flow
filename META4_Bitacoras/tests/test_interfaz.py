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
from test_meta4_bitacoras import diary, lote_html, save_diaries  # noqa: E402


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
    assert shown[-1][1][1] == 'Elige la carpeta del lote de registros.'
    app.source.set('x')
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
    app.source.set(str(empty))
    app.cutoff.set('2026-10-16')
    app.folder.set(str(tmp_path / 'salidas'))
    app.start()
    esperar(app)
    assert shown[-1][0] == 'showerror' and 'Faltan bitácoras' in shown[-1][1][1]
    assert app.output is None
