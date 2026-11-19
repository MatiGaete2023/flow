"""P2 (plan flow-python): revisión de bitácoras de punta a punta con lotes ficticios.

Descargador (alcance de la ventana) → paquete (`filters_json.revision_bitacoras`) → Asistente (fila.revision_bitacora) →
clasificación por período → Excel y JSON."""

import hashlib
import json
from datetime import date

from export_csmp_capture import export_capture
from openpyxl import load_workbook
from test_reutilizacion_local import bound_lot, diaries

from nurus.capture import read_capture
from nurus.personal.capture_input import import_capture
from nurus.personal.config import defaults
from nurus.personal.revision_bitacoras import CON, NO_VERIFICABLE, SIN, clasificar, exportar, resumen, revisar


def _trabajo(tmp_path, manifest):
    lot = bound_lot(tmp_path / 'lote')
    path = export_capture([lot], tmp_path / 'paquete.zip', diaries=[manifest], match_diaries=True)
    work = import_capture(path, defaults(), as_of=date(2026, 10, 13))
    return work, {work.capture_sources[row.id]['cause_id']: row for row in work.rows}, path


def _paginar(manifest, indice):
    """La ventana del ingreso `indice` trae un paginador: su lectura no acredita ausencia."""
    data = json.loads(manifest.read_text(encoding='utf-8'))
    registro = data['registros'][indice]
    html = manifest.parent / registro['archivo']
    raw = html.read_bytes().replace(b'</form>', b'<a href="#" onclick="irPagina(2)">2</a></form>', 1)
    html.write_bytes(raw)
    registro['sha256'] = hashlib.sha256(raw).hexdigest()
    manifest.write_text(json.dumps(data), encoding='utf-8')


def test_p2_paquete_lleva_el_alcance_y_el_asistente_clasifica(tmp_path):
    manifest = diaries(tmp_path / 'Bitacoras 2026-10-13', 2)
    _paginar(manifest, 1)
    work, filas, paquete = _trabajo(tmp_path, manifest)
    filtros = [json.loads(q['filters_json']) for q in read_capture(paquete).rows('consultas') if q['screen'] == 'bitacoras']
    alcances = sorted(f['revision_bitacoras']['coverage'] for f in filtros if 'revision_bitacoras' in f)
    assert alcances == ['COMPLETA', 'PARCIAL']
    leida = filas['100'].revision_bitacora
    assert leida['alcance'] == 'COMPLETA' and leida['completa'] and leida['capturada'] == '2026-10-08'
    assert leida['fechas_centro'] == ['2026-08-12']
    assert clasificar(leida, date(2026, 8, 1), date(2026, 8, 31))[0] == CON
    assert clasificar(leida, date(2026, 9, 8), date(2026, 10, 8))[0] == SIN
    clase, motivo = clasificar(leida, date(2026, 9, 14), date(2026, 10, 13))
    assert clase == NO_VERIFICABLE and 'antes del fin del período' in motivo
    clase, motivo = clasificar(filas['101'].revision_bitacora, date(2026, 9, 8), date(2026, 10, 8))
    assert clase == NO_VERIFICABLE and 'recorte' in motivo
    # Persistencia en la sesión.
    work.save(tmp_path / 'sesion')
    from nurus.personal.work import Work

    cargado = Work.load(tmp_path / 'sesion')
    assert [r.revision_bitacora for r in cargado.rows] == [r.revision_bitacora for r in work.rows]


def test_p2_exporta_excel_y_json_en_carpeta_nueva(tmp_path):
    work, _, _ = _trabajo(tmp_path, diaries(tmp_path / 'Bitacoras 2026-10-13', 1))
    desde, hasta = date(2026, 9, 8), date(2026, 10, 8)
    items = revisar(work, desde, hasta)
    conteo = resumen(items)
    assert conteo[SIN] == 1 and conteo[NO_VERIFICABLE] == 1  # el segundo ingreso no tiene bitácora leída
    excel, informe = exportar(work, items, desde, hasta, tmp_path / 'revision')
    libro = load_workbook(excel, read_only=True)
    try:
        filas = list(libro.active.iter_rows(values_only=True))
    finally:
        libro.close()
    assert filas[0][0] == 'Clasificación' and sorted(f[0] for f in filas[1:]) == ['No verificable', 'Sin observación']
    datos = json.loads(informe.read_text(encoding='utf-8'))
    assert datos['periodo'] == {'desde': '2026-09-08', 'hasta': '2026-10-08'} and datos['resumen']['Sin observación'] == 1
    try:
        exportar(work, items, desde, hasta, tmp_path / 'revision')
    except ValueError as exc:
        assert 'ya existe' in str(exc)
    else:
        raise AssertionError('debía rechazar una carpeta existente')


def test_p3c_historico_por_rut_llega_al_registro_sin_id_de_persona(tmp_path):
    from pathlib import Path

    import historico_manual as hm
    from lectura_bitacoras import persona_por_rut

    manifest = diaries(tmp_path / 'Bitacoras 2026-10-13', 1)
    fixture = (Path(__file__).parents[1] / 'tests/fixtures_integracion/historial_menores_ficticio.html').read_bytes()
    rut = hm._rut_limpio('00111111-1').lstrip('0')  # RUT ficticio de las filas de bound_lot, sin ceros a la izquierda
    hm.capturar(
        manifest.parent,
        fixture,
        {'tribunal': '113', 'person_id': persona_por_rut(rut), 'rut': rut, 'nombre': 'Persona uno'},
        capturada='2026-10-08T12:00:00+00:00',
        url=f'PopUpHistorial.jsp?tipo_popUp=77&RUT_Litigante={rut[:-1]}&RUT_DV={rut[-1]}',
    )
    lot = bound_lot(tmp_path / 'lote')
    path = export_capture([lot], tmp_path / 'p.zip', diaries=[manifest], match_diaries=True, historico=manifest.parent)
    captura = read_capture(path)
    personas = {r['person_id'] for r in captura.rows('registros')}
    assert persona_por_rut(rut) in personas
    assert {h['person_id'] for h in captura.rows('historico_proteccion')} == {persona_por_rut(rut)}
    work = import_capture(path, defaults(), as_of=date(2026, 10, 13))
    assert any('PROGRAMA FICTICIO UNO' in (r.observation or '') for r in work.rows)
