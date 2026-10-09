"""R-T17 a R-T26: capa contextual de bitácoras (vista previa ficticia de nurus.capture.temporal).

Los casos que dependen de capacidades fuera del alcance vigente (historia SITFA/PDF, CT-03..CT-10) o aún no
implementadas quedan como xfail estricto con [CONFLICTO_ABIERTO] registrado en ESTADO.json.
"""
from datetime import date

import pytest
from nurus.capture import read_capture, write_capture
from nurus.capture.temporal import chronology

from test_capture_contract import fixture

CORTE = date(2026, 10, 8)


def _contexto(tmp_path, cambio):
    manifiesto, tablas = fixture()
    base = tablas['bitacoras'][0]
    cambio(tablas, base)
    captura = read_capture(write_capture(tmp_path / 'captura.zip', manifiesto, tablas))
    return chronology(captura, tablas['registros'][0], CORTE, fixture_preview=True)


def _ids(eventos):
    return [evento['event_id'] for evento in eventos]


def test_rt17_dos_ingresos_misma_causa_sin_contaminacion(tmp_path):
    def cambio(tablas, base):
        tablas['bitacoras'].append({**base, 'event_id': 'otro-ingreso', 'ingreso_id': 'ingreso-ficticio-otro', 'event_at': '2026-09-20'})
    resultado = _contexto(tmp_path, cambio)
    assert _ids(resultado['last_center']) == ['e0']
    assert 'otro-ingreso' not in _ids(resultado['later'] + resultado['uncertain'] + resultado['recent'])


def test_rt18_bitacora_fallida_es_no_evaluable(tmp_path):
    def cambio(tablas, base):
        clave = (base['cause_id'], base['ingreso_id'])
        tablas['bitacoras'] = [e for e in tablas['bitacoras'] if (e['cause_id'], e['ingreso_id']) != clave]
    resultado = _contexto(tmp_path, cambio)
    assert resultado['class'] == 'NO_EVALUABLE' and not resultado['last_center']
    assert 'NO_ACREDITADA' in resultado['reason'] and 'sin observaciones' not in resultado['reason'].lower()


def test_rt19_rit_visual_distinto_con_ids_iguales_se_admite(tmp_path):
    def cambio(tablas, base):
        tablas['bitacoras'].append({**base, 'event_id': 'rit-visual', 'rit_original': 'OTRO-RIT-VISUAL', 'event_at': '2026-09-20'})
    assert 'rit-visual' in _ids(_contexto(tmp_path, cambio)['last_center'])


@pytest.mark.xfail(strict=True, reason='[CONFLICTO_ABIERTO] R-T19: la cronología admite el evento pero no deja advertencia '
                   'trazable de la discrepancia visual de RIT/nombre (03 §12).')
def test_rt19_rit_visual_distinto_deja_advertencia(tmp_path):
    def cambio(tablas, base):
        tablas['bitacoras'].append({**base, 'event_id': 'rit-visual', 'rit_original': 'OTRO-RIT-VISUAL', 'event_at': '2026-09-20'})
    resultado = _contexto(tmp_path, cambio)
    assert any('RIT' in hallazgo['fact'] for hallazgo in resultado['findings'])


def test_rt20_ids_distintos_mismo_nombre_no_se_cruzan(tmp_path):
    def cambio(tablas, base):
        tablas['bitacoras'].append({**base, 'event_id': 'homonimo', 'ingreso_id': 'ingreso-homonimo', 'person_id': 'persona-homonima',
                                    'author': base['author'], 'event_at': '2026-09-25'})
    resultado = _contexto(tmp_path, cambio)
    assert 'homonimo' not in _ids(resultado['last_center'] + resultado['later'] + resultado['uncertain'] + resultado['recent'])


def test_rt21_observaciones_del_centro_empatadas_se_conservan(tmp_path):
    def cambio(tablas, base):
        tablas['bitacoras'].append({**base, 'event_id': 'empate'})
    assert sorted(_ids(_contexto(tmp_path, cambio)['last_center'])) == ['e0', 'empate']


def test_rt22_observacion_anterior_a_la_ventana_no_inventa_ultima(tmp_path):
    def cambio(tablas, base):
        base['event_at'] = '2026-05-01'
    resultado = _contexto(tmp_path, cambio)
    assert not resultado['last_center'] and not resultado['later']
    assert resultado['period_from'] == '2026-06-08'


def test_rt23_evento_mismo_dia_sin_hora_queda_incierto(tmp_path):
    def cambio(tablas, base):
        tablas['bitacoras'].append({**base, 'event_id': 'mismo-dia', 'origin': 'OTRO'})
    resultado = _contexto(tmp_path, cambio)
    assert _ids(resultado['uncertain']) == ['mismo-dia'] and resultado['class'] == 'FOCALIZADA'


FUERA_DE_ALCANCE = ('[CONFLICTO_ABIERTO] {caso}: requiere {fuente}; PLAN_VIGENTE deja la historia SITFA/PDF y '
                    'CT-03..CT-10 fuera de alcance hasta nueva decisión del usuario.')


@pytest.mark.xfail(strict=True, reason=FUERA_DE_ALCANCE.format(caso='R-T24', fuente='historia de trámites (CT-03/CT-05)'))
def test_rt24_informe_vencido_con_tramite_informe_requiere_cotejo(tmp_path):
    from nurus.capture.temporal import CONTEXT_RULES
    resultado = _contexto(tmp_path, lambda tablas, base: None)
    assert any(h['rule_id'] == 'CT-05' and h['status'] == 'APLICA' for h in resultado['findings']), CONTEXT_RULES['CT-05']


@pytest.mark.xfail(strict=True, reason=FUERA_DE_ALCANCE.format(caso='R-T25', fuente='firma con cobertura e identidad (CT-08)'))
def test_rt25_firma_con_cobertura_parcial_no_afirma_ausencia(tmp_path):
    resultado = _contexto(tmp_path, lambda tablas, base: None)
    assert any(h['rule_id'] == 'CT-08' for h in resultado['findings'])


@pytest.mark.xfail(strict=True, reason=FUERA_DE_ALCANCE.format(caso='R-T26', fuente='detección de reiteraciones (CT-07)'))
def test_rt26_textos_parecidos_solo_senal_de_reiteracion(tmp_path):
    def cambio(tablas, base):
        tablas['bitacoras'].append({**base, 'event_id': 'reiterada', 'event_at': '2026-09-20', 'text': base['text'] + ' '})
    resultado = _contexto(tmp_path, cambio)
    assert any(h['rule_id'] == 'CT-07' for h in resultado['findings'])
