"""WP-50: cronología de bitácoras en modo «solo mostrar» (D5, horizonte de cuatro meses)."""

from datetime import date
import json
from pathlib import Path

import pytest

from nurus.capture import write_capture
from nurus.capture.temporal import DISPLAY_RULES, display_chronology
from nurus.personal.capture_findings import last_center_label, novelties_text
from nurus.personal.capture_input import import_capture
from nurus.personal.config import defaults
from test_capture_contract import fixture

CORTE = date(2026, 10, 8)
ROOT = Path(__file__).resolve().parents[1]


def importar(tmp_path, cambiar=None):
    m, t = fixture()
    if cambiar:
        cambiar(t)
    work = import_capture(write_capture(tmp_path / 'captura.zip', m, t), defaults(), as_of=CORTE)
    return work, {row.id: row for row in work.rows}


def test_empate_sin_hora_queda_incierto_y_posterior_se_lista(tmp_path):
    def cambiar(t):
        base = t['bitacoras'][0]
        t['bitacoras'] += [
            {**base, 'event_id': 'mismo-dia', 'origin': 'OTRO', 'text': 'Respuesta ficticia sin hora.'},
            {**base, 'event_id': 'despues', 'origin': 'OTRO', 'event_at': '2026-09-20', 'text': 'Oficio ficticio.'},
        ]

    work, rows = importar(tmp_path, cambiar)
    display = work.capture_context['r0']['display']
    assert display['status'] == 'ACREDITADA'
    assert [e['event_id'] for e in display['last_center']] == ['e0']
    assert [e['event_id'] for e in display['later']] == ['despues']
    assert [e['event_id'] for e in display['uncertain']] == ['mismo-dia']
    assert last_center_label(work, rows['r0']) == '10/09/2026 · 2 posteriores'
    texto = novelties_text(work, rows['r0'])
    assert 'orden incierto' in texto and 'Respuesta ficticia sin hora.' in texto and 'Oficio ficticio.' in texto
    # Otro ingreso de la misma causa no se mezcla.
    assert [e['event_id'] for e in work.capture_context['r1']['display']['last_center']] == ['e1']
    assert last_center_label(work, rows['r1']) == '10/09/2026'


def test_origen_no_acreditado_es_origen_desconocido(tmp_path):
    def cambiar(t):
        t['bitacoras'][0]['origin_verified'] = 'false'

    work, rows = importar(tmp_path, cambiar)
    display = work.capture_context['r0']['display']
    assert display['status'] == 'ORIGEN_DESCONOCIDO' and not display['last_center']
    assert [e['event_id'] for e in display['unattributed']] == ['e0']
    assert last_center_label(work, rows['r0']) == 'ORIGEN_DESCONOCIDO'
    assert 'ORIGEN_DESCONOCIDO' in novelties_text(work, rows['r0'])


def test_fuera_de_horizonte_no_se_busca_ni_se_afirma_ausencia(tmp_path):
    def cambiar(t):
        t['bitacoras'][0]['event_at'] = '2026-06-07'

    work, rows = importar(tmp_path, cambiar)
    display = work.capture_context['r0']['display']
    assert display['period_from'] == '2026-06-08' and display['period_to'] == '2026-10-08'
    assert display['status'] == 'SIN_ENTRADAS_CAPTURADAS' and display['outside_horizon'] == 1
    assert last_center_label(work, rows['r0']) == 'Sin entradas capturadas'
    texto = novelties_text(work, rows['r0'])
    assert 'no acredita ausencia' in texto and '1 entrada(s) anteriores' in texto


def test_hora_cierta_ordena_dentro_del_mismo_dia():
    centro = {
        'event_id': 'c',
        'event_at': '2026-09-10T10:00:00-03:00',
        'time_precision': 'SEGUNDO',
        'time_zone': 'America/Santiago',
        'origin': 'CENTRO',
        'origin_verified': 'true',
        'coverage': 'COMPLETA',
    }
    respuesta = {**centro, 'event_id': 'r', 'origin': 'OTRO', 'event_at': '2026-09-10T11:00:00-03:00'}
    previa = {**centro, 'event_id': 'p', 'origin': 'OTRO', 'event_at': '2026-09-10T09:00:00-03:00'}
    display = display_chronology([respuesta, centro, previa], CORTE)
    assert [e['event_id'] for e in display['later']] == ['r'] and not display['uncertain']


def test_vista_no_cambia_observaciones_hallazgos_ni_productos(tmp_path, monkeypatch):
    con_vista, _ = importar(tmp_path)
    from nurus.capture import temporal

    monkeypatch.setattr(temporal, 'DISPLAY_RULES', ())
    (tmp_path / 'sin').mkdir()
    sin_vista, _ = importar(tmp_path / 'sin')
    assert all('display' in c for c in con_vista.capture_context.values())
    assert not any('display' in c for c in sin_vista.capture_context.values())
    for a, b in zip(con_vista.rows, sin_vista.rows):
        assert (a.observation, a.actions, a.rules, a.review, a.warnings) == (
            b.observation,
            b.actions,
            b.rules,
            b.review,
            b.warnings,
        )
    for key, contexto in con_vista.capture_context.items():
        otro = sin_vista.capture_context[key]
        assert {k: v for k, v in contexto.items() if k != 'display'} == otro
        assert contexto['class'] == 'NO_EVALUABLE'
        assert all(f['status'] != 'APLICA' for f in contexto['findings'] if f['rule_id'].startswith('CT-'))


def test_reimportar_sesion_previa_sin_vista_la_recalcula(tmp_path):
    work, _ = importar(tmp_path)
    work.display_version = ''
    for contexto in work.capture_context.values():
        contexto.pop('display')
    m, t = fixture()
    nuevo = import_capture(write_capture(tmp_path / 'otra.zip', m, t), defaults(), as_of=CORTE, previous=work)
    assert all('display' in c for c in nuevo.capture_context.values())


def test_catalogo_coincide_con_reglas_solo_mostrar():
    catalogo = json.loads((ROOT / 'contracts/csmp_context_v1/catalog.json').read_text(encoding='utf-8'))
    activas = tuple(r['rule_id'] for r in catalogo['rules'] if r['activation_state'] == 'ACTIVA_SOLO_MOSTRAR')
    assert activas == DISPLAY_RULES == ('CT-01', 'CT-02')
    assert not any(r['enabled'] for r in catalogo['rules'])


def test_entradas_sin_fecha_y_posteriores_al_corte_se_cuentan_aparte():
    # C2 (revisión integral): ni las entradas sin fecha ni las posteriores al corte son «anteriores al período».
    centro = {
        'event_id': 'c',
        'event_at': '2026-09-10',
        'origin': 'CENTRO',
        'origin_verified': 'true',
        'coverage': 'COMPLETA',
    }
    sin_fecha = {**centro, 'event_id': 's', 'event_at': ''}
    futura = {**centro, 'event_id': 'f', 'event_at': '2026-10-20', 'origin': 'OTRO'}
    display = display_chronology([centro, sin_fecha, futura], CORTE)
    assert (display['outside_horizon'], display['undated'], display['after_cutoff']) == (0, 1, 1)


def test_bitacoras_no_solicitadas_se_distinguen_de_vacias():
    # F4: sin bitácoras solicitadas no se dice «Sin entradas capturadas».
    assert display_chronology([], CORTE, requested=False)['status'] == 'NO_SOLICITADA'
    assert display_chronology([], CORTE)['status'] == 'SIN_ENTRADAS_CAPTURADAS'


def test_paquete_sin_bitacoras_muestra_no_solicitadas(tmp_path):
    def cambiar(t):
        t['bitacoras'] = []

    m, t = fixture()
    cambiar(t)
    m['table_states']['bitacoras'] = 'NO_SOLICITADA'
    work = import_capture(write_capture(tmp_path / 'sin.zip', m, t), defaults(), as_of=CORTE)
    fila = work.rows[0]
    assert last_center_label(work, fila) == 'Bitácoras no solicitadas'
    assert 'no solicitó bitácoras' in novelties_text(work, fila)


def test_version_nueva_de_la_vista_reutiliza_la_cache_sin_reevaluar(tmp_path, monkeypatch):
    # C5 (revisión E3 de WP-86): mismos bytes con display_version antigua → se recalcula solo la vista.
    from nurus.personal import work as work_module

    work, _ = importar(tmp_path)
    work.display_version = 'solo-mostrar-1'
    for contexto in work.capture_context.values():
        contexto.pop('display')
    monkeypatch.setattr(work_module.Work, 'analyze', lambda *a, **k: pytest.fail('no debe reevaluar'))
    m, t = fixture()
    nuevo = import_capture(write_capture(tmp_path / 'otra.zip', m, t), defaults(), as_of=CORTE, previous=work)
    assert all(c['display']['version'] == nuevo.display_version for c in nuevo.capture_context.values())


def test_rollback_quita_la_vista_guardada(tmp_path, monkeypatch):
    from nurus.capture import temporal

    work, _ = importar(tmp_path)
    work.display_version = 'solo-mostrar-1'
    monkeypatch.setattr(temporal, 'DISPLAY_RULES', ())
    m, t = fixture()
    nuevo = import_capture(write_capture(tmp_path / 'otra.zip', m, t), defaults(), as_of=CORTE, previous=work)
    assert not any('display' in c for c in nuevo.capture_context.values())
