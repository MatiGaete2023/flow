"""Calendarios parciales y advertencias deben conservar su alcance real."""

import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'src'))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tests'))
from nurus.capture import read_capture
from nurus.capture.diaries import import_lote
from export_csmp_capture import export_capture
from resultados import load_lot
from test_resultados import fixture
from bitacora_fixtures import manifest as diary_fixture


def test_partial_calendar_and_unattempted_calendar_never_complete(tmp_path):
    folder = fixture(tmp_path / 'lot', mode='Informes', complete=False)
    path = folder / 'verificacion 001.json'
    manifest = json.loads(path.read_text())
    manifest['seleccion']['screen'] = 'calendario_informes'
    manifest.update(estado='INCOMPLETA', paginas_esperadas=2)
    path.write_text(json.dumps(manifest), encoding='utf-8')
    path = folder / 'resumen.json'
    summary = json.loads(path.read_text())
    summary.update(consultas=[], consultas_completadas=0, plan=[manifest['seleccion']])
    path.write_text(json.dumps(summary), encoding='utf-8')
    capture = read_capture(export_capture([load_lot(folder, partial=True)], tmp_path / 'partial.zip'))
    assert capture.manifest['table_states']['calendarios'] == 'PARCIAL'
    assert {r['coverage'] for r in capture.rows('calendarios')} == {'PARCIAL'}
    # F5 (revisión integral): la fila de calendario lleva su vencimiento ISO (fixture: 10/10/2026).
    assert {r['event_at'] for r in capture.rows('calendarios')} == {'2026-10-10'}
    (folder / 'verificacion 001.json').unlink()
    capture = read_capture(export_capture([load_lot(folder, partial=True)], tmp_path / 'unattempted.zip'))
    assert capture.manifest['table_states']['calendarios'] == 'NO_CONSULTADA'
    assert not capture.rows('calendarios')


def test_diary_visible_mismatch_reaches_package_and_query_scope(tmp_path):
    diary = tmp_path / 'diary'
    diary.mkdir()
    path = diary_fixture(diary)
    metadata = json.loads(path.read_text())
    metadata['registros'] = metadata['registros'][:1]
    metadata['registros'][0]['nombre'] = 'Otro nombre ficticio'
    path.write_text(json.dumps(metadata), encoding='utf-8')
    warnings = import_lote(path)['queries'][0]['warnings']
    assert warnings
    capture = read_capture(
        export_capture([load_lot(fixture(tmp_path / 'primary'))], tmp_path / 'capture.zip', diaries=path)
    )
    actual = [i['message'] for i in capture.rows('incidencias') if i['code'] == 'BITACORA_ADVERTENCIA']
    assert actual == warnings
    query = next(q for q in capture.rows('consultas') if q['screen'] == 'bitacoras')
    assert json.loads(query['filters_json'])['remote_identity']['ingreso_id'] == '33'
