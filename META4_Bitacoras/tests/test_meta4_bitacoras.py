"""Casos ficticios: fechas, identidades, páginas y alerta de etapa."""
import base64
from datetime import date
import hashlib
import json
from pathlib import Path
import sys
from types import SimpleNamespace
import threading

import pytest
from openpyxl import load_workbook

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
import meta4_bitacoras as app
from descarga_meta4 import DiarioMensual
from test_generate_meta4 import make_excel

CORTE = date(2026, 10, 16)


def entrada(day, stage='Espera'):
    return dict(fecha=day, etapa=stage, texto='Observación ficticia', fecha_respuesta='2026-10-16T15:00:00')


def diary(rit='X-1-2026', ingreso='201', entries=None):
    return dict(tribunal_nombre='Juzgado de Familia de Laja', tribunal_codigo='101',
                causa_id='301', ingreso_id=ingreso, rit=rit, rut='11111111-1',
                nombre='FICTICIO NNA', programa='FAE FICTICIO', estado='LEIDA',
                capturada='2026-10-16T12:00:00', advertencias=[],
                snapshot=dict(tribunal_codigo='101', causa_id='301', ingreso_id=ingreso,
                              rit=rit, persona='FICTICIO NNA', centro='FAE FICTICIO',
                              entries=entries if entries is not None else [entrada('2026-10-10T10:00:00')]))


def save_diaries(root, records):
    (root / 'bitacoras_meta4.json').write_text(json.dumps(dict(tipo='META4_BITACORAS_MES', version=1,
        desde='2026-09-16', hasta='2026-10-16', registros=records)), encoding='utf-8')


def test_month_uses_calendar_and_clamps_day():
    assert app.periodo(date(2026, 3, 31))[0] == date(2026, 2, 28)
    assert app.periodo(date(2026, 1, 10))[0] == date(2025, 12, 10)
    assert app.periodo(date(2024, 3, 31))[0] == date(2024, 2, 29)


def test_last_ignores_reports_old_and_future_uses_center_date():
    events = [entrada('2026-09-15T23:59:59'), entrada('2026-09-16T00:00:00', 'Cumplimiento'),
              entrada('2026-10-09T10:00:00'), entrada('2026-10-15T10:00:00', 'Informes'),
              entrada('2026-10-17T10:00:00', 'Cumplimiento')]
    assert app.ultima(events, CORTE)[:2] == (date(2026, 10, 9), 'Espera')


def test_ties_require_review_instead_of_inventing_order():
    day, stage, control = app.ultima([entrada('2026-10-10'), entrada('2026-10-10', 'Cumplimiento')], CORTE)
    assert day == date(2026, 10, 10) and not stage and control.startswith('REVISAR')


def test_no_monthly_observation_is_blank():
    assert app.ultima([entrada('2026-08-01'), entrada('2026-10-10', 'Informes')], CORTE)[0] is None


def prepare(root, records=None):
    make_excel(root / 'CUMP FAE LAJA.xlsx', 'Laja', 'Cumplimiento', [{'rit': 'X-1-2026', 'der': 'FAE FICTICIO'}])
    save_diaries(root, records or [diary()])


def test_generation_alert_in_meta_registros_and_consolidado(tmp_path):
    source = tmp_path / 'fuentes'
    source.mkdir()
    prepare(source, [diary(entries=[entrada('2026-10-09T10:00:00'), entrada('2026-10-15', 'Informes')])])
    target = tmp_path / 'resultado'
    result = app.generar(source, target, CORTE)
    assert result['registros'] == 1 and result['alertas'] == 1
    for filename, sheet in [('Registros.xlsx', 'CUMPLIMIENTO'), ('Registros consolidados.xlsx', 'Cumplimiento')]:
        wb = load_workbook(target / filename)
        ws = wb[sheet]
        headers = [c.value for c in ws[1]]
        assert ws.cell(2, headers.index('FECHA OBS.') + 1).value.date() == date(2026, 10, 9)
        assert ws.cell(2, headers.index('ALERTA') + 1).value == app.ALERTA
        assert ws.cell(2, 1).fill.fgColor.rgb.endswith('FFC7CE')
    wb = load_workbook(target / 'META 4 - actualizado 2026-10.xlsx')
    ws = wb['FAE Cumplimiento']
    cells = [c for row in ws for c in row if c.value == 'X-1-2026']
    assert len(cells) == 1
    cell = cells[0]
    assert ws.cell(cell.row, 8).value.date() == date(2026, 10, 9)
    assert ws.cell(cell.row, 8).comment.text == app.ALERTA
    assert cell.fill.fgColor.rgb.endswith('FFC7CE')


def test_multiple_ingresos_never_match_by_rit_only(tmp_path):
    source = tmp_path / 'fuentes'
    source.mkdir()
    prepare(source, [diary(), diary(ingreso='202')])
    target = tmp_path / 'resultado'
    report = app.generar(source, target, CORTE)
    assert report['alertas'] == 0 and report['revision_bitacora'] == 1
    ws = load_workbook(target / 'Registros consolidados.xlsx')['Cumplimiento']
    headers = [c.value for c in ws[1]]
    assert ws.cell(2, headers.index('FECHA OBS.') + 1).value is None
    assert ws.cell(2, headers.index('CONTROL BITÁCORA') + 1).value == 'IDENTIDAD AMBIGUA'


def test_empty_and_failed_diaries_visible(tmp_path):
    source = tmp_path / 'fuentes'
    source.mkdir()
    failed = diary()
    failed['estado'] = 'FALLIDA'
    prepare(source, [failed])
    result = app.generar(source, tmp_path / 'resultado', CORTE)
    assert result['revision_bitacora'] == 1


def test_persisted_period_cannot_be_extended(tmp_path):
    save_diaries(tmp_path, [diary()])
    with pytest.raises(ValueError, match='no cubren'):
        app.cargar_bitacoras(tmp_path, date(2026, 10, 17))


def test_native_multipage_and_html_xls(tmp_path):
    source = tmp_path / 'fuentes'
    source.mkdir()
    pages = []
    for index in (1, 2):
        headers = ['RIT', 'TRIBUNAL', 'RUT', 'NOMBRE', 'DERIVACIÓN', 'TIPO', 'SEXO', 'FEC. RESOLUCIÓN', 'FEC. INGRESO EFECTIVO']
        values = [f'X-{index}-2026', 'Laja', '11111111-1', 'FICTICIO NNA', 'FAE FICTICIO', 'MP', 'F', '01/09/2026', '02/09/2026']
        data = ('<html><table><tr><td>Juzgado de Familia de Laja</td></tr><tr><td>Informe de Ingresos en Lista de Cumplimiento</td></tr><tr><td>Fecha Emisión : 16/10/2026</td></tr><tr>' + ''.join(f'<td>{v}</td>' for v in headers) + '</tr><tr>' + ''.join(f'<td>{v}</td>' for v in values) + '</tr></table></html>').encode()
        filename = f'CUMP FAE LAJA {index}.xls'
        (source / filename).write_bytes(data)
        pages.append(dict(archivo=filename, sha256=hashlib.sha256(data).hexdigest()))
    (source / 'verificacion.json').write_text(json.dumps(dict(estado='VALIDADA', paginas_esperadas=2, paginas_verificadas=2, archivos=pages)))
    (source / 'resumen.json').write_text(json.dumps(dict(estado='VALIDADA', consultas=[dict(verificacion='verificacion.json', seleccion=dict(tab='Cumplimiento', modality='3'))])))
    save_diaries(source, [diary(rit='X-1-2026'), diary(rit='X-2-2026', ingreso='202')])
    result = app.generar(source, tmp_path / 'resultado', CORTE)
    assert result['registros'] == 2 and result['alertas'] == 2


def test_changed_source_rejected(tmp_path):
    (tmp_path / 'p.xls').write_bytes(b'changed')
    (tmp_path / 'v.json').write_text(json.dumps(dict(estado='VALIDADA', paginas_esperadas=1, paginas_verificadas=1, archivos=[dict(archivo='p.xls', sha256='bad')])) )
    (tmp_path / 'resumen.json').write_text(json.dumps(dict(estado='VALIDADA', consultas=[dict(verificacion='v.json', seleccion=dict(tab='Espera', modality='3'))])))
    with pytest.raises(ValueError, match='cambió'):
        app.fuentes(tmp_path)


def test_live_diary_saves_only_month_no_history(tmp_path):
    raw = b'verified transport response'
    params = {'ID_Ingreso': '201'}
    calls = []
    class Bridge:
        def call(self, command, payload):
            calls.append(command)
            return dict(status=200, type='text/html', body=base64.b64encode(raw).decode(),
                        params=params, digest=hashlib.sha256(raw).hexdigest(), startedAt='2026-10-16T10:00:00Z')
    snap = diary()['snapshot']
    snap['entries'] = [entrada('2026-09-01'), entrada('2026-10-10'), entrada('2026-10-17')]
    session = SimpleNamespace(openings=0, folder=tmp_path, read=lambda *a: snap, anotar=lambda *a: None)
    record = {'params': params}
    runner = DiarioMensual(Bridge(), {}, lambda *a: None, threading.Event(), corte=CORTE)
    runner._abrir_bitacora(session, record)
    assert calls == ['diary_open']
    assert len(record['snapshot']['entries']) == 1
    assert not list(tmp_path.glob('*.html'))
    assert record['estado'] == 'LEIDA'


def test_remote_identity_narrows_matching_ingreso():
    row = app.meta.SourceRow('Cumplimiento', 'FAE', 'LAJA', {
        'TRIBUNAL': 'Laja', 'RIT': 'X-1-2026', 'RUT': '11111111-1', 'DERIVACIÓN': 'FAE FICTICIO',
        'NOMBRE': 'FICTICIO NNA', '_REMOTE': ('101', '301', '202')}, '', '', 1)
    records = [dict(diary(ingreso=str(i)), control='OK', entries=[entrada('2026-10-10', s)])
               for i, s in [(201, 'Cumplimiento'), (202, 'Espera')]]
    app.enriquecer({'Cumplimiento': [row]}, records, CORTE)
    assert row.values['ALERTA'] == app.ALERTA


def test_same_rit_different_person_or_program_not_associated():
    row = app.meta.SourceRow('Cumplimiento', 'FAE', 'LAJA', {
        'TRIBUNAL': 'Laja', 'RIT': 'X-1-2026', 'RUT': '22222222-2', 'DERIVACIÓN': 'FAE FICTICIO',
        'NOMBRE': 'OTRA PERSONA'}, '', '', 1)
    d = dict(diary(), control='OK', entries=[entrada('2026-10-10')])
    assert app.buscar(row, [d]) == (None, 'SIN BITÁCORA ASOCIADA')


def test_last_cumplimiento_does_not_raise_alert():
    row = app.meta.SourceRow('Cumplimiento', 'FAE', 'LAJA', {
        'TRIBUNAL': 'Laja', 'RIT': 'X-1-2026', 'RUT': '11111111-1', 'DERIVACIÓN': 'FAE FICTICIO',
        'NOMBRE': 'FICTICIO NNA'}, '', '', 1)
    d = dict(diary(), control='OK', entries=[entrada('2026-10-10'), entrada('2026-10-11', 'Cumplimiento')])
    app.enriquecer({'Cumplimiento': [row]}, [d], CORTE)
    assert row.values['FECHA OBS.'] == date(2026, 10, 11)
    assert not row.values['ALERTA']


def test_month_export_contains_only_center_dates_in_window(tmp_path):
    d = dict(diary(), control='OK', entries=[entrada('2026-10-10T12:34:56')])
    app.exportar_bitacoras(tmp_path / 'bitacoras.xlsx', [d])
    ws = load_workbook(tmp_path / 'bitacoras.xlsx')['Observaciones del mes']
    assert ws.max_row == 2
    assert ws.cell(2, 8).value.strftime('%H:%M:%S') == '12:34:56'
