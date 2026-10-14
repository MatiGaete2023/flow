"""WP-R1: historia y bitácora llegan al motor sin cambiar ninguna observación."""

import json
from datetime import date

from historia_fixtures import CUMPLIMIENTO, ESPERA, INFORMES, Todas, captura_falsa, correr, cumplimiento, fila, hist
from test_mayoria_edad import analizar, persona

from nurus.personal.capture_input import _historias_por_causa
from nurus.personal.motor.historia import construir, ultimo


def test_construir_descarta_filas_sin_fecha_o_sin_descripcion():
    h = construir(
        [
            {'event_at': '', 'description': 'A', 'folio': '1'},
            {'event_at': '2026-10-01', 'description': ' ', 'folio': '2'},
        ],
        'COMPLETA',
    )
    assert h.tramites == ()


def test_buscar_ignora_tildes_mayusculas_y_puntos_finales():
    h = hist(('Prorroga', '2026-10-01', 1), ('PRORROGA.', '2026-10-02', 2), ('prórroga', '2026-10-03', 3))
    assert len(h.buscar('Prórroga')) == 3
    assert (
        len(
            hist(('Resp. Diagnóstico clínico especializado', '2026-10-01', 4)).buscar(
                'Resp. Diagnóstico clínico especializado.'
            )
        )
        == 1
    )


def test_folio_numerico_y_no_numerico():
    h = hist(('A', '2026-10-01', '41'), ('B', '2026-10-01', 'S/N'))
    por_desc = {t.desc: t for t in h.tramites}
    assert (por_desc['A'].folio, por_desc['B'].folio, por_desc['B'].folio_txt) == (41, -1, 'S/N')


def test_ultimo_del_mismo_dia_es_el_de_mayor_folio():
    h = hist(('A', '2026-10-01', 7), ('A', '2026-10-01', 9))
    assert ultimo(list(h.tramites)).folio == 9 and ultimo([]) is None


def test_cobertura_presencia_y_ausencia():
    assert hist(cobertura='PARCIAL').con_datos and not hist(cobertura='PARCIAL').permite_ausencia
    assert hist(cobertura='VACIA_COMPROBADA').permite_ausencia and not hist(cobertura='VACIA_COMPROBADA').con_datos
    assert not hist(cobertura='FALLIDA').con_datos and not hist(cobertura='NO_CONSULTADA').permite_ausencia


def _consulta(qid, cobertura, causa='100'):
    filtros = {
        'historia_causa': {'source_system': 'RUS', 'court_code': 'T1', 'cause_id': causa},
        'ventana': {'criterio': 'ventana_4_meses', 'desde': '2026-06-14', 'hasta': '2026-10-14'},
    }
    return {'query_id': qid, 'screen': 'historia_tramites', 'filters_json': json.dumps(filtros), 'coverage': cobertura}


def test_historias_por_causa_usa_clave_cobertura_y_gana_la_mejor():
    fila = {'query_id': 'b', 'event_at': '2026-10-01', 'description': 'Prorroga', 'folio': '5'}
    captura = captura_falsa(
        [
            _consulta('a', 'PARCIAL'),
            _consulta('b', 'COMPLETA'),
            {'query_id': 'x', 'screen': 'seguimiento', 'filters_json': '{}', 'coverage': 'COMPLETA'},
        ],
        [fila],
    )
    resultado = _historias_por_causa(captura)
    assert list(resultado) == [('RUS', 'T1', '100')]
    historia = resultado[('RUS', 'T1', '100')]
    assert historia.cobertura == 'COMPLETA' and len(historia.tramites) == 1
    assert (historia.desde, historia.hasta) == (date(2026, 6, 14), date(2026, 10, 14))


def test_historias_por_causa_sin_consultas_es_vacio():
    assert _historias_por_causa(captura_falsa([], [])) == {}


def test_analyze_con_historia_vacia_o_sin_ella_no_cambia_nada(tmp_path):
    from openpyxl import Workbook

    from nurus.personal.config import defaults
    from nurus.personal.work import Work

    libro = Workbook()
    hoja = libro.active
    hoja.title = 'ESPERA'
    hoja.append(['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'NOMBRE CENTRO', 'T ESPERA'])
    hoja.append(persona(1) + [45])
    ruta = tmp_path / 'e.xlsx'
    libro.save(ruta)
    sin = Work(defaults()).analyze(ruta, 'ESPERA', as_of=date(2026, 10, 14))
    con = Work(defaults()).analyze(
        ruta, 'ESPERA', as_of=date(2026, 10, 14), historia_by_record=Todas(None), bitacora_by_record=Todas(())
    )
    assert (sin.rows[0].observation, sin.rows[0].rules) == (con.rows[0].observation, con.rows[0].rules)
    assert sin.rows[0].avisos == con.rows[0].avisos == []
    _ = analizar


def test_sin_cobertura_de_presencia_el_resultado_es_el_original_en_las_tres_etapas(tmp_path):
    for modo, enc, datos in (
        ('ESPERA', ESPERA, fila(1) + [45]),
        ('CUMPLIMIENTO', CUMPLIMIENTO, cumplimiento(1)),
        ('INFORMES', INFORMES, fila(1) + ['2026-09-20']),
    ):
        a = correr(tmp_path, modo, enc, [datos], paquete=False).rows[0]
        b = correr(
            tmp_path, modo, enc, [datos], {0: hist(('Ingreso efectivo', '2026-10-02', 41), cobertura='NO_SOLICITADA')}
        ).rows[0]
        assert (a.observation, a.rules, a.actions) == (b.observation, b.rules, b.actions)
        assert a.avisos == b.avisos == []
