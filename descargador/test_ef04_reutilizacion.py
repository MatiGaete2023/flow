"""EF-04: reutilizar lecturas de bitácoras de otros lotes del mismo día, con identidad, parámetros y hash."""

import hashlib
import json
import threading
from datetime import datetime, timedelta
from pathlib import Path
from zoneinfo import ZoneInfo

import lectura_bitacoras as lb
from lotes import Batch
from test_lectura_bitacoras import CATALOG, Bridge
from csmp_shared import parser

SANTIAGO = ZoneInfo('America/Santiago')
BATCH = Batch(('49',), ('2',), ('Cumplimiento',))


def _clock(monkeypatch, start):
    # Reloj ficticio creciente: las carpetas de lote llevan microsegundos en su nombre.
    state = {'t': start}

    def now():
        state['t'] += timedelta(microseconds=1)
        return state['t']

    monkeypatch.setattr(lb.m, 'now', now)
    return state


def _run(tmp_path, bridge, reuse=True):
    parser()
    destination = tmp_path / 'destino'
    destination.mkdir(exist_ok=True)
    runner = lb.DiaryRunner(
        bridge, CATALOG, lambda *a: None, threading.Event(), pause=0, reuse_base=destination if reuse else None
    )
    result = runner.run(BATCH, destination)
    return result, json.loads(Path(result['manifest']).read_text(encoding='utf-8'))


def test_segundo_lote_del_dia_reutiliza_sin_abrir_sitfa(tmp_path, monkeypatch):
    # La extensión ficticia captura a las 12:00 UTC (09:00 en Santiago).
    _clock(monkeypatch, datetime(2026, 10, 8, 15, 0, tzinfo=SANTIAGO))
    first_bridge = Bridge()
    first, _ = _run(tmp_path, first_bridge)
    assert first['estado'] == 'COMPLETA' and first_bridge.openings
    second_bridge = Bridge()
    second, data = _run(tmp_path, second_bridge)
    assert second['estado'] == 'COMPLETA'
    assert second_bridge.openings == []
    read = [r for r in data['registros'] if r['estado'] == 'LEIDA']
    assert read and all(r.get('reutilizada') and r['reutilizada_de']['carpeta'] for r in read)
    folder = Path(second['manifest']).parent
    for record in read:
        copy = folder / record['archivo']
        assert copy.is_file() and hashlib.sha256(copy.read_bytes()).hexdigest() == record['sha256']


def test_sin_reutilizacion_vuelve_a_leer_todo(tmp_path, monkeypatch):
    _clock(monkeypatch, datetime(2026, 10, 8, 15, 0, tzinfo=SANTIAGO))
    _run(tmp_path, Bridge())
    again = Bridge()
    _run(tmp_path, again, reuse=False)
    assert again.openings


def test_lectura_de_otro_dia_no_se_reutiliza(tmp_path, monkeypatch):
    clock = _clock(monkeypatch, datetime(2026, 10, 8, 15, 0, tzinfo=SANTIAGO))
    first = Bridge()
    _run(tmp_path, first)
    clock['t'] = datetime(2026, 10, 9, 8, 0, tzinfo=SANTIAGO)  # menos de 24 h, pero otro día
    again = Bridge()
    _run(tmp_path, again)
    assert len(again.openings) == len(first.openings)


def test_archivo_alterado_en_el_lote_anterior_se_vuelve_a_leer(tmp_path, monkeypatch):
    _clock(monkeypatch, datetime(2026, 10, 8, 15, 0, tzinfo=SANTIAGO))
    first_result, first_data = _run(tmp_path, Bridge())
    record = next(r for r in first_data['registros'] if r['estado'] == 'LEIDA')
    target = Path(first_result['manifest']).parent / record['archivo']
    target.write_bytes(target.read_bytes() + b'<!-- alterado -->')
    again = Bridge()
    _, data = _run(tmp_path, again)
    assert record['params'] in again.openings
    reread = next(r for r in data['registros'] if r['clave'] == record['clave'])
    assert reread['estado'] == 'LEIDA' and not reread.get('reutilizada')


def test_carpetas_previas_limita_a_diez_y_excluye_la_actual(tmp_path):
    base = tmp_path / 'destino'
    folders = []
    for i in range(12):
        folder = base / f'Bitacoras 2026-10-08 10-00-{i:02d}'
        folder.mkdir(parents=True)
        (folder / lb.MANIFEST).write_text('{}', encoding='utf-8')
        folders.append(folder.resolve())
    packaged = base / 'Paquete CSMP x' / 'respaldo' / 'Bitacoras y'
    packaged.mkdir(parents=True)
    (packaged / lb.MANIFEST).write_text('{}', encoding='utf-8')
    found = lb.carpetas_previas(base, actual=folders[0])
    assert len(found) == 10 and folders[0] not in found
    assert packaged.resolve() in lb.carpetas_previas(base, limite=20)
