"""Q-03: mismo ingreso verificado en dos pestañas de estado del mismo lote."""

from datetime import date
from types import SimpleNamespace

from nurus.capture import write_capture
from nurus.capture.fixtures import fixture
from nurus.personal.capture_input import import_capture, mark_inconsistent_states
from nurus.personal.config import defaults


def _record(record_type, ingreso='i1', status='VERIFICADA'):
    return {
        'source_system': 'RUS',
        'court_code': 'T1',
        'cause_id': 'c1',
        'ingreso_id': ingreso,
        'record_type': record_type,
        'identity_status': status,
    }


def _run(provenance):
    rows = [SimpleNamespace(id=key, warnings=[]) for key in provenance]
    mark_inconsistent_states(rows, provenance)
    return {row.id: row.warnings for row in rows}


def test_espera_y_cumplimiento_del_mismo_ingreso_se_avisan_en_ambas_filas():
    warnings = _run({'a': _record('ESPERA'), 'b': _record('CUMPLIMIENTO')})
    expected = ['Q-03: el mismo ingreso figura en Espera y Cumplimiento en este lote; revisar su estado en RUS.']
    assert warnings == {'a': expected, 'b': expected}


def test_informes_convive_con_cumplimiento_sin_aviso():
    assert _run({'a': _record('CUMPLIMIENTO'), 'b': _record('INFORMES')}) == {'a': [], 'b': []}


def test_identidad_no_verificada_o_ingresos_distintos_no_avisan():
    assert _run({'a': _record('ESPERA'), 'b': _record('EGRESADOS', status='INCIERTA')}) == {'a': [], 'b': []}
    assert _run({'a': _record('ESPERA'), 'b': _record('CUMPLIMIENTO', ingreso='i2')}) == {'a': [], 'b': []}


def test_importacion_de_paquete_marca_q03(tmp_path):
    metadata, tables = fixture(count=2)
    second = tables['registros'][1]
    second['ingreso_id'] = tables['registros'][0]['ingreso_id']
    second['record_type'] = 'CUMPLIMIENTO'
    tables['bitacoras'][1]['ingreso_id'] = second['ingreso_id']
    path = tmp_path / 'paquete.zip'
    write_capture(path, metadata, tables)
    work = import_capture(path, defaults(), as_of=date(2026, 10, 19))
    flagged = [row for row in work.rows if any(w.startswith('Q-03:') for w in row.warnings)]
    assert len(flagged) == 2
