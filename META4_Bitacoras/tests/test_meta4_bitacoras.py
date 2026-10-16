"""Casos ficticios: fechas, identidades, páginas y alerta de etapa."""
from datetime import date
import hashlib
import json
from pathlib import Path
import sys

import pytest
from openpyxl import load_workbook

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
import meta4_bitacoras as app
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


# --------------------------------------------------------------------------
# Regresiones 1.1
# --------------------------------------------------------------------------

def lote_html(source, filas, tab='Cumplimiento'):
    """Lote del descargador con una consulta y una página XLS HTML por elemento de `filas`."""
    headers = ['RIT', 'TRIBUNAL', 'RUT', 'NOMBRE', 'DERIVACIÓN', 'TIPO', 'SEXO', 'FEC. RESOLUCIÓN', 'FEC. INGRESO EFECTIVO']
    pages = []
    for index, fila in enumerate(filas, 1):
        values = {'RIT': f'X-{index}-2026', 'TRIBUNAL': 'Laja', 'RUT': '11111111-1', 'NOMBRE': 'FICTICIO NNA',
                  'DERIVACIÓN': 'FAE FICTICIO', 'TIPO': 'MP', 'SEXO': 'F', 'FEC. RESOLUCIÓN': '01/09/2026',
                  'FEC. INGRESO EFECTIVO': '02/09/2026', **fila}
        data = ('<html><table><tr><td>Juzgado de Familia de Laja</td></tr><tr><td>Informe de Ingresos en Lista de ' + tab +
                '</td></tr><tr><td>Fecha Emisión : 16/10/2026</td></tr><tr>' + ''.join(f'<td>{h}</td>' for h in headers) +
                '</tr><tr>' + ''.join(f'<td>{values[h]}</td>' for h in headers) + '</tr></table></html>').encode()
        filename = f'CUMP FAE LAJA {index}.xls'
        (source / filename).write_bytes(data)
        pages.append(dict(archivo=filename, sha256=hashlib.sha256(data).hexdigest()))
    (source / 'verificacion.json').write_text(json.dumps(dict(estado='VALIDADA', paginas_esperadas=len(pages), paginas_verificadas=len(pages), archivos=pages)))
    (source / 'resumen.json').write_text(json.dumps(dict(estado='VALIDADA', consultas=[dict(verificacion='verificacion.json', seleccion=dict(tab=tab, modality='3'))])))


def test_text_starting_with_equals_is_kept_as_text(tmp_path):
    source = tmp_path / 'fuentes'
    source.mkdir()
    lote_html(source, [{'TIPO': '=MP'}])
    save_diaries(source, [diary(rit='X-1-2026')])
    target = tmp_path / 'resultado'
    app.generar(source, target, CORTE)
    ws = load_workbook(target / 'Registros.xlsx')['CUMPLIMIENTO']
    headers = [c.value for c in ws[1]]
    cell = ws.cell(2, headers.index('TIPO') + 1)
    assert cell.value == '=MP' and cell.data_type == 's'
    ws = load_workbook(target / 'META 4 - actualizado 2026-10.xlsx')['FAE Cumplimiento']
    row = next(c.row for r in ws for c in r if c.value == 'X-1-2026')
    assert ws.cell(row, 3).value == '=MP' and ws.cell(row, 3).data_type == 's'


def test_pages_of_one_query_are_not_duplicates(tmp_path):
    source = tmp_path / 'fuentes'
    source.mkdir()
    lote_html(source, [{}, {}])
    save_diaries(source, [diary(rit='X-1-2026'), diary(rit='X-2-2026', ingreso='202')])
    target = tmp_path / 'resultado'
    app.generar(source, target, CORTE)
    ws = load_workbook(target / 'Registros.xlsx')['MATRIZ_COBERTURA']
    text = next(ws.cell(r, 3).value for r in range(5, ws.max_row + 1) if ws.cell(r, 1).value == 'LAJA')
    assert text == 'EXCEL · 2 registro(s)'
    assert 'duplicadas: 0' in ws['A2'].value


def test_existing_output_is_rejected_before_processing(tmp_path):
    target = tmp_path / 'resultado'
    target.mkdir()
    with pytest.raises(ValueError, match='ya existe'):
        app.generar(tmp_path, target, CORTE)


def test_failed_verification_leaves_no_partial_output(tmp_path, monkeypatch):
    source = tmp_path / 'fuentes'
    source.mkdir()
    prepare(source)
    monkeypatch.setattr(app.meta, 'verify_meta_output', lambda path: ({}, {}, ['simulado']))
    with pytest.raises(ValueError, match='simulado'):
        app.generar(source, tmp_path / 'resultado', CORTE)
    assert sorted(p.name for p in tmp_path.iterdir()) == ['fuentes']


def test_missing_template_is_an_error(tmp_path):
    source = tmp_path / 'fuentes'
    source.mkdir()
    prepare(source)
    with pytest.raises(ValueError, match='plantilla'):
        app.generar(source, tmp_path / 'resultado', CORTE, template=tmp_path / 'no-existe.xlsx')


def test_report_declares_reading_state_and_controls(tmp_path):
    source = tmp_path / 'fuentes'
    source.mkdir()
    prepare(source)
    report = app.generar(source, tmp_path / 'resultado', CORTE)
    assert report['lectura_bitacoras'] == ['SIN ESTADO']
    assert report['controles_bitacora'] == {'VERIFICADA EN EL PERÍODO': 1}


PARAMS = {'tipo_popUp': '11', 'CRR_IdCausa': '301', 'COD_Tribunal': '101', 'TIP_Consulta': '3',
          'ID_Ingreso': '201', 'COD_Etapa': '5', 'FLG_MejorNinez': '0'}


def bitacora_html(fecha='10/10/2026 10:00', etapa='Espera'):
    headers = ['Tipo Observación', 'Etapa', 'Fecha Centro', 'Usuario Centro', 'Observación Centro',
               'Fecha Tribunal', 'Usuario Tribunal', 'Observación Tribunal', 'Acción']
    hidden = ''.join(f'<input type="hidden" name="{n}77" value="{v}">' for n, v in
                     [('HOBS_Centro_', 'Texto ficticio'), ('HOBS_Tribunal_', '---'), ('HCOD_Estado_', '1'),
                      ('HFUN_Tribunal_', ''), ('HFLG_Envio_', 'N')])
    inputs = ''.join(f'<input name="{k}" value="{v}">' for k, v in [
        ('COD_Tribunal', '101'), ('CRR_IdCausa', '301'), ('ID_Ingreso', '201'), ('COD_Etapa', '5'),
        ('TIP_Consulta', '3'), ('tipo_popUp', '11'), ('RIT_Causa', 'X-1-2026'), ('GLS_Nombre', 'FICTICIO NNA'),
        ('GLS_Centro', 'FAE FICTICIO')])
    cells = ['Observación', etapa, fecha, 'Usuario', 'Texto ficticio', '---', '', '---']
    return ('<html><body><form name="InformesPpalForm" action="/SITFAWEB/InformesDAction.do">' + inputs +
            '<table id="TablaInforme"><tr>' + ''.join(f'<td>{h}</td>' for h in headers) + '</tr><tr>' +
            ''.join(f'<td>{c}</td>' for c in cells) + '<td><img id="77" onclick="ver()">' + hidden +
            '</td></tr></table></form></body></html>').encode('utf-8')


def legacy_lote(source, body):
    folder = source / 'Bitacoras 2026-10-16'
    folder.mkdir()
    (folder / 'b1.html').write_bytes(body)
    record = {k: v for k, v in diary().items() if k != 'snapshot'}
    record.update(archivo='b1.html', sha256=hashlib.sha256(body).hexdigest(), params=PARAMS)
    (folder / 'bitacoras.json').write_text(json.dumps(dict(tipo='BITACORAS_LECTURA', version=1, estado='COMPLETA',
                                                            registros=[record])), encoding='utf-8')
    return folder / 'b1.html'


def test_legacy_downloader_html_diaries_are_verified(tmp_path):
    source = tmp_path / 'fuentes'
    source.mkdir()
    make_excel(source / 'CUMP FAE LAJA.xlsx', 'Laja', 'Cumplimiento', [{'rit': 'X-1-2026', 'der': 'FAE FICTICIO'}])
    legacy_lote(source, bitacora_html())
    report = app.generar(source, tmp_path / 'resultado', CORTE)
    assert report['alertas'] == 1 and report['lectura_bitacoras'] == ['COMPLETA']


def test_legacy_html_changed_after_download_is_not_used(tmp_path):
    source = tmp_path / 'fuentes'
    source.mkdir()
    page = legacy_lote(source, bitacora_html())
    page.write_bytes(bitacora_html(etapa='Cumplimiento'))
    records = app.cargar_bitacoras(source, CORTE)
    assert records[0]['control'] == 'BITÁCORA NO VERIFICABLE' and records[0]['entries'] == []


def test_cli_rejects_future_cutoff_and_bad_month(tmp_path):
    import subprocess
    script = Path(app.__file__)
    future = subprocess.run([sys.executable, str(script), '--folder', str(tmp_path), '--output-dir', str(tmp_path / 'r'),
                             '--cutoff', '2999-01-01'], capture_output=True, text=True)
    assert future.returncode == 1 and 'futuro' in future.stderr
    month = subprocess.run([sys.executable, str(script), '--folder', str(tmp_path), '--output-dir', str(tmp_path / 'r'),
                            '--month', '2026-13'], capture_output=True, text=True)
    assert month.returncode == 2 and 'AAAA-MM' in month.stderr


# --------------------------------------------------------------------------
# 2.0: solo lotes del Descargador
# --------------------------------------------------------------------------

def test_diaries_in_separate_folder(tmp_path):
    source, diaries = tmp_path / 'registros', tmp_path / 'bitacoras'
    source.mkdir()
    diaries.mkdir()
    make_excel(source / 'CUMP FAE LAJA.xlsx', 'Laja', 'Cumplimiento', [{'rit': 'X-1-2026', 'der': 'FAE FICTICIO'}])
    legacy_lote(diaries, bitacora_html())
    with pytest.raises(ValueError, match='Faltan bitácoras'):
        app.generar(source, tmp_path / 'r1', CORTE)
    report = app.generar(source, tmp_path / 'r2', CORTE, bitacoras=diaries)
    assert report['alertas'] == 1


def test_filtered_lot_is_rejected(tmp_path):
    lote_html(tmp_path, [{}])
    data = json.loads((tmp_path / 'resumen.json').read_text())
    data['filtro_fecha'] = {'desde': '01/09/2026', 'hasta': '30/09/2026'}
    (tmp_path / 'resumen.json').write_text(json.dumps(data))
    with pytest.raises(ValueError, match='sin filtros'):
        app.fuentes(tmp_path)
    data['filtro_fecha'] = None
    data['consultas'][0]['seleccion']['report'] = '4'
    (tmp_path / 'resumen.json').write_text(json.dumps(data))
    with pytest.raises(ValueError, match='vencimiento'):
        app.fuentes(tmp_path)


def test_read_before_cutoff_requires_review(tmp_path):
    source = tmp_path / 'fuentes'
    source.mkdir()
    prepare(source, [dict(diary(), capturada='2026-10-10T12:00:00')])
    report = app.generar(source, tmp_path / 'resultado', CORTE)
    assert report['controles_bitacora'] == {app.LEIDA_ANTES: 1}
    assert app.fecha_lectura(source) == date(2026, 10, 10)


def test_failed_read_in_one_lot_does_not_hide_verified_read_in_another(tmp_path):
    failed = diary(entries=[entrada('2026-10-01', 'Cumplimiento')])
    failed['estado'] = 'FALLIDA'
    ok = diary(entries=[entrada('2026-10-10', 'Espera')])
    for name, record in (('a', failed), ('b', ok)):
        (tmp_path / name).mkdir()
        save_diaries(tmp_path / name, [record])
    records = app.cargar_bitacoras(tmp_path, CORTE)
    row = app.meta.SourceRow('Cumplimiento', 'FAE', 'LAJA', {
        'TRIBUNAL': 'Laja', 'RIT': 'X-1-2026', 'RUT': '11111111-1', 'DERIVACIÓN': 'FAE FICTICIO',
        'NOMBRE': 'FICTICIO NNA'}, '', '', 1)
    app.enriquecer({'Cumplimiento': [row]}, records, CORTE)
    assert row.values['ALERTA'] == app.ALERTA


def test_tampered_read_still_blocks_even_with_other_lot(tmp_path):
    bad = dict(diary(), snapshot=dict(diary()['snapshot'], ingreso_id='999'))
    for name, record in (('a', bad), ('b', diary())):
        (tmp_path / name).mkdir()
        save_diaries(tmp_path / name, [record])
    row = app.meta.SourceRow('Cumplimiento', 'FAE', 'LAJA', {
        'TRIBUNAL': 'Laja', 'RIT': 'X-1-2026', 'RUT': '11111111-1', 'DERIVACIÓN': 'FAE FICTICIO',
        'NOMBRE': 'FICTICIO NNA'}, '', '', 1)
    assert app.buscar(row, app.cargar_bitacoras(tmp_path, CORTE)) == (None, app.NO_VERIFICABLE)


def test_resumed_lot_previous_reads_are_used(tmp_path):
    source = tmp_path / 'fuentes'
    source.mkdir()
    make_excel(source / 'CUMP FAE LAJA.xlsx', 'Laja', 'Cumplimiento', [{'rit': 'X-1-2026', 'der': 'FAE FICTICIO'}])
    legacy_lote(source, bitacora_html())
    path = next(source.rglob('bitacoras.json'))
    data = json.loads(path.read_text())
    data['anteriores_no_reenumeradas'], data['registros'] = data['registros'], []
    path.write_text(json.dumps(data))
    assert app.generar(source, tmp_path / 'resultado', CORTE)['alertas'] == 1


def test_queries_without_diaries_are_reported(tmp_path):
    source = tmp_path / 'fuentes'
    source.mkdir()
    lote_html(source, [{}])
    data = json.loads((source / 'resumen.json').read_text())
    data['consultas'][0].update(estado='VALIDADA')
    data['consultas'][0]['seleccion'].update(tribunal='101', tribunal_nombre='Juzgado de Familia de Laja', modalidad_nombre='FAE')
    (source / 'resumen.json').write_text(json.dumps(data))
    (source / 'b').mkdir()
    (source / 'b' / 'bitacoras_meta4.json').write_text(json.dumps(dict(
        tipo='META4_BITACORAS_MES', desde='2026-09-16', hasta='2026-10-16', registros=[diary()],
        consultas=[dict(tribunal='101', tab='Espera', modality='3', estado='ENUMERADA')])))
    report = app.generar(source, tmp_path / 'resultado', CORTE)
    assert report['consultas_sin_bitacoras'] == ['Juzgado de Familia de Laja / Cumplimiento / FAE']
