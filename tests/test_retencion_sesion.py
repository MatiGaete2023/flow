"""Retención local con datos ficticios y confirmación expresa."""

import json
import os
import time
import pytest
from hashlib import sha256
from types import SimpleNamespace

from openpyxl import Workbook
from nurus.personal.config import defaults, RETENCION_DIAS, SESIONES_ANTERIORES
from nurus.personal.session import prune_previous, previous_candidates, saved_data_summary
from nurus.personal.work import Work


def _libro(ruta, numero):
    libro = Workbook()
    hoja = libro.active
    hoja.title = 'ESPERA'
    hoja.append(['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'NOMBRE CENTRO', 'T ESPERA'])
    hoja.append([f'X-{numero}-2026', 'LAJA', 'Persona Ficticia', '11111111-1', 'PRM uno', 40 + numero])
    libro.save(ruta)
    return ruta


def _anterior(root, name, days):
    folder = root / name
    folder.mkdir(parents=True)
    path = folder / 'trabajo.json'
    path.write_text('{}', encoding='utf-8')
    age = time.time() - days * 86400
    os.utime(path, (age, age))
    (folder / 'copia.bin').write_bytes(b'ficticio')
    return folder


def test_guardar_sesion_deja_solo_las_dos_copias_recuperables(tmp_path):
    sesion = tmp_path / 'sesion'
    for numero in range(3):
        trabajo = Work(defaults())
        trabajo.analyze(_libro(tmp_path / f't{numero}.xlsx', numero), 'ESPERA')
        trabajo.save(sesion)
    copias = {p.name for p in sesion.glob('*.bin')}
    assert len(copias) == 2 and trabajo.source_hash + '.bin' in copias
    assert Work.load(sesion).source_hash == trabajo.source_hash
    previous = json.loads((sesion / 'trabajo.json.bak').read_text())
    assert previous['source_hash'] + '.bin' in copias


@pytest.mark.parametrize('backup', [None, [], 'texto', {'source_hash': 123}, '{'])
def test_copia_invalida_no_impide_guardar_ni_borra_bins(tmp_path, backup):
    sesion = tmp_path / 'sesion'
    sesion.mkdir()
    (sesion / 'trabajo.json.bak').write_text(backup if backup == '{' else json.dumps(backup), encoding='utf-8')
    preservada = sesion / 'recuperable.bin'
    preservada.write_bytes(b'ficticio')
    trabajo = Work(defaults())
    trabajo.analyze(_libro(tmp_path / 'fuente.xlsx', 1), 'ESPERA')
    trabajo.save(sesion)
    assert Work.load(sesion).source_hash == trabajo.source_hash
    assert preservada.exists()


def test_json_principal_no_objeto_no_sobrescribe_respaldo(tmp_path):
    sesion = tmp_path / 'sesion'
    sesion.mkdir()
    (sesion / 'trabajo.json').write_text('null', encoding='utf-8')
    backup = sesion / 'trabajo.json.bak'
    backup.write_text('{"source_hash": "anterior"}', encoding='utf-8')
    (sesion / 'anterior.bin').write_bytes(b'ficticio')
    trabajo = Work(defaults())
    trabajo.analyze(_libro(tmp_path / 'fuente.xlsx', 1), 'ESPERA')
    trabajo.save(sesion)
    assert json.loads(backup.read_text()) == {'source_hash': 'anterior'}
    assert (sesion / 'anterior.bin').exists()


def test_limite_decidido_conserva_diez_y_sesenta_dias_tras_confirmar(tmp_path):
    assert (RETENCION_DIAS, SESIONES_ANTERIORES) == (60, 10)
    anteriores = tmp_path / 'sesion' / 'anteriores'
    for numero in range(12):
        _anterior(anteriores, f's{numero:02}', numero)
    _anterior(anteriores, 'antigua', 70)
    _anterior(tmp_path / 'sesion', 'vigente', 90)
    assert prune_previous(anteriores) == []
    assert len(list(anteriores.iterdir())) == 13
    assert sorted(prune_previous(anteriores, confirmed=True)) == ['antigua', 's10', 's11']
    assert len(list(anteriores.iterdir())) == 10
    assert (tmp_path / 'sesion' / 'vigente' / 'trabajo.json').exists()


def test_poda_ignora_carpetas_desconocidas_y_enlaces(tmp_path):
    anteriores = tmp_path / 'anteriores'
    anteriores.mkdir()
    (anteriores / 'ajeno').mkdir()
    (anteriores / 'ajeno' / 'dato.txt').write_text('ficticio')
    outside = _anterior(tmp_path, 'externo', 80)
    (anteriores / 'enlace').symlink_to(outside, target_is_directory=True)
    assert previous_candidates(anteriores) == []
    assert prune_previous(anteriores, confirmed=True) == []
    assert outside.exists() and (anteriores / 'ajeno' / 'dato.txt').exists()


def test_interfaz_cancelada_no_borra_y_confirmada_preserva_vigente(tmp_path, monkeypatch):
    from nurus.personal.app_base import App
    from nurus.personal import app_base

    archive = tmp_path / 'sesion' / 'anteriores'
    old = _anterior(archive, 'antigua', 70)
    current = tmp_path / 'sesion' / 'trabajo.json'
    current.write_text('{}')
    messages = []
    app = SimpleNamespace(cfg=SimpleNamespace(directory=tmp_path), status=SimpleNamespace(set=messages.append))
    monkeypatch.setattr(app_base.messagebox, 'askyesno', lambda *a, **kw: False)
    assert App._prune_previous_sessions(app) == [] and old.exists() and current.exists()
    assert app._retention_pending and 'pendiente' in messages[-1]
    monkeypatch.setattr(app_base.messagebox, 'askyesno', lambda *a, **kw: True)
    assert App._prune_previous_sessions(app) == ['antigua']
    assert not old.exists() and current.exists()


def test_borrado_por_fecha_no_aplica_limite_de_cantidad(tmp_path):
    archive = tmp_path / 'anteriores'
    for number in range(12):
        _anterior(archive, str(number), 10)
    old = _anterior(archive, 'antigua', 70)
    assert prune_previous(archive, keep=None, confirmed=True) == ['antigua']
    assert not old.exists() and len(list(archive.iterdir())) == 12


def test_inventario_cuenta_adjuntos_y_recibos_sin_borrar_externos(tmp_path):
    attachment = tmp_path / 'externo.xlsx'
    attachment.write_bytes(b'ficticio')
    session = tmp_path / 'sesion'
    session.mkdir()
    (session / 'actual.bin').write_bytes(b'copia')
    data = {
        'receipts': {'r1': {'estado': 'borrador'}},
        'session': {'drafts': [{'attachments': [str(attachment), str(attachment)]}]},
    }
    (session / 'trabajo.json').write_text(json.dumps(data), encoding='utf-8')
    report = saved_data_summary(tmp_path)
    assert report['sesiones']['count'] == 1 and report['copias'] == {'count': 1, 'bytes': 5}
    assert report['adjuntos'] == {'count': 1, 'bytes': 8} and report['recibos']['count'] == 1
    assert report['recibos']['bytes'] > 0 and attachment.exists()


def test_fuentes_originales_legacy_y_backup_se_conservan(tmp_path):
    session = tmp_path / 'sesion'
    for number in range(2):
        work = Work(defaults())
        work.analyze(_libro(tmp_path / f'legacy{number}.xlsx', number), 'ESPERA')
        work.original_content = f'original ficticio {number}'.encode()
        work.original_hash = sha256(work.original_content).hexdigest()
        work.save(session)
    assert Work.load(session).original_content == work.original_content
    previous = json.loads((session / 'trabajo.json.bak').read_text())
    for field in ('source_hash', 'original_hash'):
        assert (session / (previous[field] + '.bin')).exists()
    # La excepción legacy conserva cuatro referencias; el flujo actual normal usa dos.
    assert len(list(session.glob('*.bin'))) == 4


@pytest.mark.parametrize(
    'drafts',
    [
        None,
        {},
        [None],
        [{'attachments': None}],
        [{'attachments': 'archivo.xlsx'}],
        [{'attachments': ['\x00archivo', '', None]}],
    ],
)
def test_inventario_tolera_datos_parciales_sin_borrar_la_sesion(tmp_path, drafts):
    session = tmp_path / 'sesion'
    session.mkdir()
    data = json.dumps({'session': {'drafts': drafts}})
    path = session / 'trabajo.json'
    path.write_text(data)
    report = saved_data_summary(tmp_path)
    assert report['sesiones']['count'] == 1
    assert report['adjuntos'] == {'count': 0, 'bytes': 0}
    assert path.read_text() == data


def test_inventario_ignora_json_vigente_que_es_un_enlace(tmp_path):
    external = tmp_path / 'externo.json'
    external.write_text('{}')
    session = tmp_path / 'sesion'
    session.mkdir()
    (session / 'trabajo.json').symlink_to(external)
    report = saved_data_summary(tmp_path)
    assert report['sesiones']['count'] == 0
    assert external.read_text() == '{}'
