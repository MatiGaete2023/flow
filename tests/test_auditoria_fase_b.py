"""Regresiones de los defectos reproducidos en la auditoría final; solo fuentes ficticias."""

from datetime import date

import pytest
from historia_fixtures import captura_con_historia, fila, hist, inf

from nurus.capture import read_capture, write_capture
from nurus.personal.capture_input import import_capture
from nurus.personal.config import defaults
from nurus.personal.record_edits import apply
from nurus.personal.work import Work


def _marcas(fila_):
    return [aviso[1] for aviso in fila_.avisos]


@pytest.mark.parametrize('corte,aplica', [('2026-10-09', False), ('2026-10-10', True), ('2026-10-14', True)])
def test_historia_solo_usa_hechos_hasta_el_corte_y_preserva_la_fuente(tmp_path, corte, aplica):
    metadata, tables = captura_con_historia([('Ingreso efectivo', '2026-10-10', 40)])
    path = write_capture(tmp_path / 'captura.zip', metadata, tables)
    original = path.read_bytes()
    work = import_capture(path, defaults(), as_of=date.fromisoformat(corte))
    # «Ingreso efectivo» con RUS en Espera: reglas normales y solo la marca, y solo con hechos hasta el corte.
    assert ('R3_INGRESO_EFECTIVO' in _marcas(work.rows[0])) is aplica
    assert work.rows[0].actions == ['programa_espera', 'PC_IE']
    assert read_capture(path).rows('historia_tramites')[0]['event_at'] == '2026-10-10'
    assert path.read_bytes() == original


def test_reimportar_con_otro_corte_recalcula_historia_sin_perder_edicion(tmp_path):
    metadata, tables = captura_con_historia([('Ingreso efectivo', '2026-10-10', 40)])
    path = write_capture(tmp_path / 'captura.zip', metadata, tables)
    previous = import_capture(path, defaults(), as_of=date(2026, 10, 14))
    previous.rows[0].review['OBSERVACION'] = 'Edición humana ficticia'
    updated = import_capture(path, defaults(), as_of=date(2026, 10, 9), previous=previous)
    assert 'R3_INGRESO_EFECTIVO' not in _marcas(updated.rows[0])
    assert updated.rows[0].review['OBSERVACION'] == 'Edición humana ficticia'
    assert 'R3_INGRESO_EFECTIVO' in _marcas(previous.rows[0])


@pytest.mark.parametrize('cause_id', ['causa-ficticia-0', ''])
def test_registro_incierto_no_recibe_historia_por_sus_ids_declarados(tmp_path, cause_id):
    metadata, tables = captura_con_historia([('Ingreso efectivo', '2026-10-10', 40)])
    tables['registros'][0].update(identity_status='INCIERTA', cause_id=cause_id)
    path = write_capture(tmp_path / 'captura.zip', metadata, tables)
    work = import_capture(path, defaults(), as_of=date(2026, 10, 14))
    assert 'R3_INGRESO_EFECTIVO' not in _marcas(work.rows[0])
    assert work.rows[0].actions == ['programa_espera', 'PC_IE']
    # Otro ingreso de la misma causa mantiene su vínculo acreditado; no se suprime la fuente globalmente.
    assert 'R3_INGRESO_EFECTIVO' in _marcas(work.rows[1])
    assert read_capture(path).rows('historia_tramites')


def test_historia_antes_de_la_ventana_no_acredita_ausencia():
    source = hist(('Ingreso efectivo', '2026-10-10', 40))
    view = source.al_corte(date(2026, 6, 1))
    assert not view.con_datos
    assert view.tramites == () and source.tramites


def test_historia_recorta_ventana_de_ausencia_sin_elevar_cobertura_parcial():
    source = hist(('Prorroga', '2026-10-10', 40), cobertura='PARCIAL')
    view = source.al_corte(date(2026, 10, 9))
    assert view.hasta == date(2026, 10, 9) and view.desde == source.desde
    assert view.cobertura == 'PARCIAL' and not view.tramites


@pytest.mark.parametrize('descripcion', ['Ingreso informe de cumplimiento X', 'Inf. Cump Programa Ambulatorio'])
def test_informe_no_retiro_acciones_por_tramite_futuro(tmp_path, descripcion):
    row = inf(tmp_path, fila(1) + ['2026-09-20'], hist((descripcion, '2026-10-15', 40)))
    assert row.actions == ['programa_vencido', 'PC_INFO']
    assert 'I01_CONTRADICHA' not in [warning[1] for warning in row.avisos]


def test_escrito_de_periodo_anterior_no_retiro_acciones_del_actual(tmp_path):
    row = inf(tmp_path, fila(1) + ['2026-09-20'], hist(('Inf. Cump Programa Ambulatorio', '2026-07-01', 40)))
    assert row.actions == ['programa_vencido', 'PC_INFO']
    assert 'I01_CONTRADICHA' not in [warning[1] for warning in row.avisos]


@pytest.mark.parametrize('descripcion', ['Ingreso informe de cumplimiento X', 'Inf. Cump Programa Ambulatorio'])
def test_entrega_temprana_de_periodo_incierto_advierte_sin_cambiar_textos_ni_acciones(tmp_path, descripcion):
    row = inf(tmp_path, fila(1) + ['2026-09-20'], hist((descripcion, '2026-09-15', 40)))
    assert row.actions == ['programa_vencido', 'PC_INFO']
    assert row.rules == ['INFORMES.I01_VENCIDO_GENERAL']
    # La advertencia es una marca de nivel 2 en la bandeja (decisión x), como el resto de las señales de revisión.
    assert [aviso[:2] for aviso in row.avisos] == [(2, 'ENTREGA_ANTERIOR')]
    assert 'período' in row.avisos[0][2]


@pytest.mark.parametrize('fecha', ['2026-09-20', '2026-09-25'])
def test_escrito_desde_vencimiento_conserva_contraste_aprobado(tmp_path, fecha):
    row = inf(tmp_path, fila(1) + ['2026-09-20'], hist(('Inf. Cump Programa Ambulatorio', fecha, 40)))
    assert row.actions == ['NOMENCL'] and 'I01_CONTRADICHA' in [warning[1] for warning in row.avisos]


def test_edicion_y_recuperacion_no_pierden_contexto_ni_dependen_del_zip_original(tmp_path):
    metadata, tables = captura_con_historia([('Ingreso efectivo', '2026-10-10', 40)])
    path = write_capture(tmp_path / 'captura.zip', metadata, tables)
    work = import_capture(path, defaults(), as_of=date(2026, 10, 14))
    original = list(work.rows[0].rules)
    apply(work, [work.rows[0].id], overrides={'espera': '46'})
    assert work.rows[0].rules == original and work.rows[0].actions == ['programa_espera', 'PC_IE']
    assert 'R3_INGRESO_EFECTIVO' in _marcas(work.rows[0])
    work.save(tmp_path / 'sesion')
    path.unlink()  # La sesión conserva sus fuentes; no debe volver a abrir el archivo original.
    restored = Work.load(tmp_path / 'sesion', defaults())
    apply(restored, [restored.rows[0].id], overrides={'espera': '47'})
    assert restored.rows[0].rules == original and restored.rows[0].actions == ['programa_espera', 'PC_IE']
    assert 'R3_INGRESO_EFECTIVO' in _marcas(restored.rows[0])


def test_editar_rut_actualiza_la_duplicidad_del_par_y_se_deshace_junto(tmp_path):
    from historia_fixtures import ESPERA, correr
    from nurus.personal.record_edits import undo

    filas = [fila(1, rit='C-1-2026', rut='11111111-1') + [45], fila(2, rit='C-1-2026', rut='22222222-2') + [45]]
    work = correr(tmp_path, 'ESPERA', ESPERA, filas, paquete=False)
    assert [r.avisos for r in work.rows] == [[], []]
    before = apply(work, [work.rows[1].id], overrides={'rut': '11111111-1'})
    assert [[a[1] for a in r.avisos] for r in work.rows] == [['R6_DUPLICIDAD'], ['R6_DUPLICIDAD']]
    undo(work, before)
    assert [r.avisos for r in work.rows] == [[], []]


def test_editar_vencimiento_actualiza_el_acumulado_del_par(tmp_path):
    from historia_fixtures import INFORMES, correr

    filas = [fila(1, rit='C-1-2026') + ['2026-08-20'], fila(2, rit='C-1-2026') + ['2026-11-20']]
    filas[1][2] = filas[0][2]
    work = correr(tmp_path, 'INFORMES', INFORMES, filas, paquete=False)
    assert 'HISTORIA.J2_ACUMULADOS' not in work.rows[0].rules
    apply(work, [work.rows[1].id], overrides={'vencimiento': '2026-09-20'})
    assert all('HISTORIA.J2_ACUMULADOS' in r.rules for r in work.rows)
    assert 'el más antiguo venció el 20 de agosto de 2026' in work.rows[0].observation
