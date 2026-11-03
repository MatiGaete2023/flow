"""Pantallazo opcional en consultas sin registros: el lote, el paquete y el Asistente lo reconocen."""

import json
import threading

from nurus.capture import read_capture

from export_csmp_capture import export_capture
from lotes import Batch, BatchRunner
from resultados import export_book, load_lot
from test_lotes import CATALOG, FakeBridge


def correr(tmp_path, evidencia):
    bridge, events = FakeBridge(empty_first=True), []
    result = BatchRunner(
        bridge, CATALOG, lambda n, v: events.append((n, v)), threading.Event(), evidencia=evidencia
    ).run(Batch(('49',), ('1', '2'), ('Espera',)), tmp_path)
    folder = next(v for n, v in events if n == 'folder')
    return result, load_lot(folder), bridge, events


def test_sin_pdf_no_abre_la_captura_y_marca_la_consulta_vacia(tmp_path):
    result, lot, bridge, events = correr(tmp_path, evidencia=False)
    assert result[0]['estado'] == 'SIN_RESULTADOS' and result[0]['evidencia_omitida'] is True
    assert 'evidencia' not in result[0]
    assert not {'evidence_open', 'evidence_capture', 'evidence_close'} & {c for c, _ in bridge.events}
    assert not list(lot.folder.glob('*.pdf')) and len(list(lot.folder.glob('*.xls'))) == 3
    info = json.loads((lot.folder / 'verificacion 001.json').read_text(encoding='utf-8'))
    assert info['estado'] == 'SIN_RESULTADOS' and info['evidencia_omitida'] is True and 'evidencia' not in info
    assert ('empty', {'consulta': result[0]['consulta']}) in events
    assert events[-1][1]['pdfs'] == 0 and events[-1][1]['empty'] == 1


def test_paquete_declara_vacia_comprobada_sin_pdf(tmp_path):
    _, lot, _, _ = correr(tmp_path / 'lote', evidencia=False)
    capture = read_capture(export_capture([lot], tmp_path / 'Captura_CSMP.zip'))
    vacia = [q for q in capture.rows('consultas') if q['coverage'] == 'VACIA_COMPROBADA']
    assert len(vacia) == 1
    avisos = [i for i in capture.rows('incidencias') if i['code'] == 'VACIA_SIN_PDF']
    assert [(i['query_id'], i['severity']) for i in avisos] == [(vacia[0]['query_id'], 'INFORMACION')]


def test_con_pdf_sigue_igual_y_sin_incidencia(tmp_path):
    result, lot, _, _ = correr(tmp_path / 'lote', evidencia=True)
    assert result[0]['evidencia']['tipo'] == 'PDF_CAPTURA_SITFA' and 'evidencia_omitida' not in result[0]
    capture = read_capture(export_capture([lot], tmp_path / 'Captura_CSMP.zip'))
    assert not [i for i in capture.rows('incidencias') if i['code'] == 'VACIA_SIN_PDF']


def test_libro_de_resultados_dice_sin_pdf(tmp_path):
    from openpyxl import load_workbook

    _, lot, _, _ = correr(tmp_path / 'lote', evidencia=False)
    path = export_book([lot], tmp_path / 'resultado.xlsx')
    book = load_workbook(path, read_only=True)
    filas = [r for ws in book.worksheets for r in ws.iter_rows(values_only=True) if r and r[0] == 'SIN_RESULTADOS']
    assert filas and filas[0][3] == 'Sin PDF'


def test_asistente_muestra_la_consulta_vacia_sin_pdf(tmp_path):
    from types import SimpleNamespace

    from nurus.personal.capture_products import evidence_text

    _, lot, _, _ = correr(tmp_path / 'lote', evidencia=False)
    capture = read_capture(export_capture([lot], tmp_path / 'Captura_CSMP.zip'))
    work = SimpleNamespace(
        capture_digest='a' * 64,
        capture_id=capture.manifest['capture_id'],
        as_of='2026-11-03',
        ruleset_version='r',
        capture_manifest=capture.manifest,
        capture_incidents=capture.rows('incidencias'),
    )
    assert 'VACIA_SIN_PDF · INFORMACION · Consulta sin registros; no se pidió pantallazo PDF.' in evidence_text(work)


def test_menu_mas_sin_separadores_dobles_y_con_abrir_sitfa():
    import os
    import sys

    import pytest

    tk = pytest.importorskip('tkinter')
    if sys.platform != 'win32' and not os.environ.get('DISPLAY'):
        pytest.skip('Sin pantalla para Tk.')
    from descargador import Application
    from test_lotes import dispose_interface

    root = tk.Tk()
    root.withdraw()
    app = Application(root, start_worker=False)
    try:
        menu = app.topbar.menu
        tipos = [menu.type(i) for i in range(menu.index('end') + 1)]
        etiquetas = [menu.entrycget(i, 'label') for i, t in enumerate(tipos) if t != 'separator']
        assert 'Abrir mi Chrome en SITFA' in etiquetas
        assert all(not (a == b == 'separator') for a, b in zip(tipos, tipos[1:]))
    finally:
        dispose_interface(app, root)
