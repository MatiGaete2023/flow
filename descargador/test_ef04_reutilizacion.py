"""EF-04: reutilizar lecturas de bitácoras de otros lotes del mismo día, con identidad, parámetros y hash."""

import hashlib
import json
import threading
from datetime import datetime, timedelta
from pathlib import Path
from zoneinfo import ZoneInfo

import pytest

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


def _manifest(folder, records):
    folder.mkdir(parents=True, exist_ok=True)
    (folder / lb.MANIFEST).write_text(
        json.dumps({'version': 1, 'tipo': 'BITACORAS_LECTURA', 'registros': records}), encoding='utf-8'
    )


def _leida(clave, capturada):
    return {
        'clave': clave,
        'estado': 'LEIDA',
        'capturada': capturada,
        'archivo': 'x.html',
        'sha256': '0' * 64,
        'entradas': 1,
        'params': {},
    }


@pytest.mark.parametrize(
    'contents',
    [
        '[]',
        '{"version": 1, "tipo": "BITACORAS_LECTURA", "registros": {"no": "lista"}}',
        '{"version": 1, "tipo": "BITACORAS_LECTURA", "registros": [{"estado": "LEIDA", "clave": ["no", "hashable"]}]}',
        '{"version": 1, "tipo": "BITACORAS_LECTURA", "registros": [{"estado": "LEIDA"}]}',
    ],
)
def test_manifiesto_previo_malformado_no_bloquea_el_lote(tmp_path, monkeypatch, contents):
    # E3 WP-93 #1: un lote ajeno inválido se omite; el lote actual se lee normalmente.
    _clock(monkeypatch, datetime(2026, 10, 8, 15, 0, tzinfo=SANTIAGO))
    bad = tmp_path / 'destino' / 'Bitacoras malformada'
    bad.mkdir(parents=True)
    (bad / lb.MANIFEST).write_text(contents, encoding='utf-8')
    bridge = Bridge()
    result, _ = _run(tmp_path, bridge)
    assert result['estado'] == 'COMPLETA' and bridge.openings


def test_gana_la_captura_mas_reciente_no_la_carpeta_tocada_al_final(tmp_path):
    # E3 WP-93 #2: A (08:00) se modificó después que B (13:00); debe ganar B.
    now = datetime(2026, 10, 8, 15, 0, tzinfo=SANTIAGO)
    a, b = tmp_path / 'A', tmp_path / 'B'
    _manifest(b, [_leida('k', '2026-10-08T13:00:00-03:00')])
    _manifest(a, [_leida('k', '2026-10-08T08:00:00-03:00')])
    found = lb.lecturas_reutilizables([a, b], now)
    assert found['k'][0] == b


@pytest.mark.parametrize(
    'capturada,now,expected',
    [
        ('2026-10-08T23:30:00-03:00', datetime(2026, 10, 9, 0, 30, tzinfo=SANTIAGO), False),  # 1 h, otro día
        ('2026-10-08T07:00:00-03:00', datetime(2026, 10, 8, 20, 0, tzinfo=SANTIAGO), False),  # 13 h, mismo día
        ('2026-10-08T09:00:00-03:00', datetime(2026, 10, 8, 20, 0, tzinfo=SANTIAGO), True),  # 11 h, mismo día
        ('2026-10-08T16:00:00-03:00', datetime(2026, 10, 8, 15, 0, tzinfo=SANTIAGO), False),  # futura
    ],
)
def test_guardas_de_mismo_dia_y_doce_horas_por_separado(tmp_path, capturada, now, expected):
    _manifest(tmp_path / 'A', [_leida('k', capturada)])
    assert ('k' in lb.lecturas_reutilizables([tmp_path / 'A'], now)) is expected


def test_misma_clave_con_otros_parametros_se_vuelve_a_leer(tmp_path, monkeypatch):
    _clock(monkeypatch, datetime(2026, 10, 8, 15, 0, tzinfo=SANTIAGO))
    first_result, first_data = _run(tmp_path, Bridge())
    manifest = Path(first_result['manifest'])
    data = json.loads(manifest.read_text(encoding='utf-8'))
    target = next(r for r in data['registros'] if r['estado'] == 'LEIDA')
    target['params'] = {**target['params'], 'COD_Etapa': 'otra-etapa-ficticia'}
    manifest.write_text(json.dumps(data), encoding='utf-8')
    again = Bridge()
    _, second = _run(tmp_path, again)
    reread = next(r for r in second['registros'] if r['clave'] == target['clave'])
    assert not reread.get('reutilizada') and reread['estado'] == 'LEIDA'


def test_copia_previa_ilegible_se_lee_en_sitfa_y_no_queda_fallida(tmp_path, monkeypatch):
    # E3 WP-93 #4: hash correcto pero contenido que el lector rechaza → se abre en SITFA, no queda FALLIDA.
    _clock(monkeypatch, datetime(2026, 10, 8, 15, 0, tzinfo=SANTIAGO))
    first_result, _ = _run(tmp_path, Bridge())
    manifest = Path(first_result['manifest'])
    data = json.loads(manifest.read_text(encoding='utf-8'))
    garbage = b'contenido ficticio ilegible'
    for record in data['registros']:
        if record['estado'] == 'LEIDA':
            (manifest.parent / record['archivo']).write_bytes(garbage)
            record['sha256'] = hashlib.sha256(garbage).hexdigest()
    manifest.write_text(json.dumps(data), encoding='utf-8')
    real = lb.parser()

    def reader(content, params):
        if content == garbage:
            raise ValueError('ilegible')
        return real(content, params)

    monkeypatch.setattr(lb, 'parser', lambda: reader)
    again = Bridge()
    result, second = _run(tmp_path, again)
    assert result['estado'] == 'COMPLETA' and again.openings
    assert all(r['estado'] == 'LEIDA' and not r.get('reutilizada') for r in second['registros'] if r.get('params'))


def test_respaldo_enlazado_fuera_del_destino_se_ignora(tmp_path):
    # E3 WP-93 #5.
    base = tmp_path / 'destino'
    outside = tmp_path / 'fuera' / 'respaldo' / 'Bitacoras ajena'
    _manifest(outside, [])
    (base / 'Paquete CSMP x').mkdir(parents=True)
    (base / 'Paquete CSMP x' / 'respaldo').symlink_to(outside.parent, target_is_directory=True)
    assert lb.carpetas_previas(base) == []


@pytest.mark.parametrize(
    'campo,valor',
    [
        ('archivo', None),
        ('archivo', ['x.html']),
        ('sha256', None),
        ('entradas', '3'),
        ('params', None),
        ('capturada', 5),
    ],
)
def test_registro_previo_con_tipos_invalidos_se_relee(tmp_path, monkeypatch, campo, valor):
    # Revisión externa H4: un campo previo de tipo inválido no aborta el lote; se lee en SITFA.
    _clock(monkeypatch, datetime(2026, 10, 8, 15, 0, tzinfo=SANTIAGO))
    first_result, _ = _run(tmp_path, Bridge())
    manifest = Path(first_result['manifest'])
    data = json.loads(manifest.read_text(encoding='utf-8'))
    for record in data['registros']:
        if record['estado'] == 'LEIDA':
            record[campo] = valor
    manifest.write_text(json.dumps(data), encoding='utf-8')
    again = Bridge()
    result, _ = _run(tmp_path, again)
    assert result['estado'] == 'COMPLETA' and again.openings


def test_falla_leyendo_la_copia_previa_se_relee(tmp_path, monkeypatch):
    _clock(monkeypatch, datetime(2026, 10, 8, 15, 0, tzinfo=SANTIAGO))
    first_result, _ = _run(tmp_path, Bridge())
    previous = Path(first_result['manifest']).parent
    original = Path.read_bytes

    def locked(self):
        if self.parent == previous and self.suffix == '.html':
            raise PermissionError('bloqueo ficticio')
        return original(self)

    monkeypatch.setattr(Path, 'read_bytes', locked)
    again = Bridge()
    result, _ = _run(tmp_path, again)
    assert result['estado'] == 'COMPLETA' and again.openings
