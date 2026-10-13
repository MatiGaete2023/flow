"""WP-53.6: historia de la causa en Captura_CSMP.zip (alcance CAUSA, contrato aditivo, Asistente intacto)."""

import json
import subprocess
import sys
import threading
from datetime import date
from pathlib import Path

from descargador import package_current_lot
from lectura_historia import LectorHistoria
from nurus.capture import read_capture
from nurus.personal.capture_input import import_capture
from nurus.personal.config import defaults
from test_lectura_historia import LISTADO, PERFIL, QUERY, SELECCION, FakeBridge
from test_reutilizacion_local import bound_lot, diaries

ROOT = Path(__file__).resolve().parents[1]


def historia(tmp_path, bridge=None):
    carpeta = tmp_path / 'lote-historia'
    carpeta.mkdir()
    lector = LectorHistoria(bridge or FakeBridge(), lambda *a: None, threading.Event(), pause=0)
    lector.abrir(carpeta)
    lector.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    lector.cerrar()
    return carpeta / 'historia' / 'historias.json'


def paquete(tmp_path, nombre, **kw):
    lote = bound_lot(tmp_path / ('lote-' + nombre))
    manifiesto = diaries(tmp_path / ('Bitacoras ' + nombre), 2)
    return package_current_lot(lote.folder, manifiesto, tmp_path / (nombre + '.zip'), **kw)['path']


def validar(path):
    r = subprocess.run(
        [sys.executable, str(ROOT / 'tools' / 'flow_capture_cli.py'), 'validate', str(path)],
        capture_output=True,
        text=True,
        cwd=ROOT,
        check=False,
    )
    assert r.returncode == 0, r.stdout + r.stderr


def test_historia_en_paquete_alcance_causa(tmp_path):
    ruta = paquete(tmp_path, 'con', historia=historia(tmp_path))
    captura = read_capture(ruta)
    filas = captura.rows('historia_tramites')
    assert [f['folio'] for f in filas] == ['9120', '9119', '9117']
    assert all(
        f['scope'] == 'CAUSA'
        and f['ingreso_id'] == ''
        and f['identity_status'] == 'CAUSA'
        and f['source_system'] == 'RUS'
        and f['cause_id'] == '100'
        and f['annex_visible'] in ('true', 'false')
        and f['original_file'].startswith('respaldo/lote-historia/historia/historia-')
        for f in filas
    )
    assert json.loads(filas[0]['raw_json'])['Fecha Tram'] == '13/10/2099'
    assert captura.manifest['table_states']['historia_tramites'] == 'COMPLETA'
    consulta = next(q for q in captura.manifest['queries'] if q['screen'] == 'historia_tramites')
    filtros = json.loads(consulta['filters_json'])
    assert 'remote_identity' not in filtros and filtros['historia_causa']['cause_id'] == '100'
    assert filtros['ventana']['criterio'] == 'ventana_4_meses'
    assert any(i['code'] == 'HISTORIA_FILA_INCOMPLETA' for i in captura.rows('incidencias'))
    validar(ruta)


def test_sin_historia_el_paquete_no_cambia(tmp_path):
    ruta = paquete(tmp_path, 'sin')
    captura = read_capture(ruta)
    assert captura.manifest['table_states']['historia_tramites'] == 'NO_SOLICITADA'
    assert len(captura.rows('historia_tramites')) == 0
    assert all(q['screen'] != 'historia_tramites' for q in captura.manifest['queries'])
    import zipfile

    with zipfile.ZipFile(ruta) as z:
        assert 'data/historia_tramites.csv' not in z.namelist()


def test_historia_fallida_no_es_vacia_y_el_paquete_se_crea(tmp_path):
    ruta = paquete(tmp_path, 'fallida', historia=historia(tmp_path, FakeBridge(b'<html><table></table></html>')))
    captura = read_capture(ruta)
    assert len(captura.rows('historia_tramites')) == 0
    assert captura.manifest['table_states']['historia_tramites'] == 'FALLIDA'
    assert any(i['code'] == 'HISTORIA_FALLIDA' for i in captura.rows('incidencias'))
    validar(ruta)


def test_asistente_produce_el_mismo_trabajo_con_y_sin_historia(tmp_path):
    con = import_capture(paquete(tmp_path, 'a', historia=historia(tmp_path)), defaults(), as_of=date(2026, 10, 8))
    sin = import_capture(paquete(tmp_path, 'b'), defaults(), as_of=date(2026, 10, 8))
    assert len(con.rows) == len(sin.rows)
    assert [(r.observation, r.review, r.decisions) for r in con.rows] == [
        (r.observation, r.review, r.decisions) for r in sin.rows
    ]
