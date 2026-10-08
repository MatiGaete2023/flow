from resultados import load_lot, export_ics
from test_resultados import fixture
import json


def unfold(path):
    return path.read_text(encoding='utf-8').replace('\n ', '')


def test_projected_exit_is_not_presented_as_report_deadline(tmp_path):
    lot = load_lot(fixture(tmp_path/'cumplimiento', mode='Cumplimiento'))
    content = unfold(export_ics(lot))
    assert 'DTSTART;VALUE=DATE:20261230' in content
    assert 'SUMMARY:Egreso proyectado SITFA' in content
    assert 'SUMMARY:Vencimiento de informe' not in content
    assert 'FEC.EGRESO PROYECTADO' in content
    assert 'no acredita un egreso efectivo' in content


def test_report_deadline_keeps_its_explicit_meaning(tmp_path):
    lot = load_lot(fixture(tmp_path/'informes', mode='Informes'))
    content = unfold(export_ics(lot))
    assert 'SUMMARY:Vencimiento de informe SITFA' in content
    assert 'FECHA VENCIMIENTO' in content
    assert 'DTSTART;VALUE=DATE:20261010' in content
    assert '00111111' not in content


def test_partial_calendar_declares_incomplete_source(tmp_path):
    lot = load_lot(fixture(tmp_path/'parcial', mode='Informes', complete=False), partial=True)
    content = unfold(export_ics(lot))
    assert 'Cobertura del lote: INCOMPLETO' in content


def test_measure_calendar_deadline_is_not_a_report_deadline(tmp_path):
    folder=fixture(tmp_path/'medidas',mode='Informes')
    for name in ('resumen.json','verificacion 001.json'):
        path=folder/name;data=json.loads(path.read_text())
        if name=='resumen.json':
            data['plan'][0]['screen']='calendario_medidas'
            data['consultas'][0]['seleccion']['screen']='calendario_medidas'
        else:data['seleccion']['screen']='calendario_medidas'
        path.write_text(json.dumps(data),encoding='utf-8')
    content=unfold(export_ics(load_lot(folder)))
    assert 'SUMMARY:Vencimiento de medida SITFA' in content
    assert 'SUMMARY:Vencimiento de informe' not in content
    assert 'no acredita un egreso efectivo' in content
