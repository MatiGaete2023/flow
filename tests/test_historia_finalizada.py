"""Causa P con Conciliación o Dictación de sentencia sin «Egreso de servicio…» en esa fecha o posterior."""

import pytest
from historia_fixtures import cum, cumplimiento, esp, fila, hist, inf

TEXTO = (
    'La causa se encuentra finalizada, se deja constancia de que se remite proyecto de resolución que aplica '
    'nomenclatura «Egreso de servicio protección especializada y otras redes» a fin de regularizar informáticamente la causa.'
)
EGRESO = 'Egreso de servicio protección especializada y otras redes'
CONCILIACION = ('Conciliacion', '2026-10-01', 30)
SENTENCIA = ('Dictación de sentencia', '2026-10-02', 31)


def _espera(tmp_path, historia, rit='P-1-2026', extra=(), encabezados=()):
    return esp(tmp_path, fila(1, rit=rit) + [45], historia, extra=list(extra), encabezados=list(encabezados))


@pytest.mark.parametrize('tramite', [CONCILIACION, SENTENCIA])
def test_espera_causa_p_con_cierre_sin_egreso(tmp_path, tramite):
    f = _espera(tmp_path, hist(tramite))
    assert f.observation.endswith(': ' + TEXTO) and f.rules == ['HISTORIA.P_FINALIZADA']
    assert f.actions == [] and f.avisos == []


def test_ambos_cierres_dan_una_sola_constancia(tmp_path):
    f = _espera(tmp_path, hist(CONCILIACION, SENTENCIA))
    assert f.observation.count('La causa se encuentra finalizada') == 1


def test_nombres_se_comparan_sin_tildes_ni_mayusculas(tmp_path):
    f = _espera(tmp_path, hist(('DICTACION DE SENTENCIA.', '2026-10-02', 31)))
    assert f.rules == ['HISTORIA.P_FINALIZADA']


@pytest.mark.parametrize('fecha', ['2026-10-02', '2026-10-09'])
def test_egreso_en_la_misma_fecha_o_posterior_no_dispara(tmp_path, fecha):
    f = _espera(tmp_path, hist(SENTENCIA, (EGRESO, fecha, 40)))
    assert 'HISTORIA.P_FINALIZADA' not in f.rules


def test_egreso_anterior_al_cierre_no_cuenta(tmp_path):
    f = _espera(tmp_path, hist((EGRESO, '2026-09-01', 20), SENTENCIA))
    assert f.rules == ['HISTORIA.P_FINALIZADA']


def test_el_cierre_mas_reciente_exige_su_propio_egreso(tmp_path):
    f = _espera(tmp_path, hist(CONCILIACION, (EGRESO, '2026-10-01', 32), ('Dictación de sentencia', '2026-10-05', 33)))
    assert f.rules == ['HISTORIA.P_FINALIZADA']


@pytest.mark.parametrize('rit', ['C-1-2026', 'F-1-2026', ''])
def test_solo_rit_que_parte_con_p(tmp_path, rit):
    f = _espera(tmp_path, hist(SENTENCIA), rit=rit)
    assert 'HISTORIA.P_FINALIZADA' not in f.rules


def test_rit_en_minuscula_cuenta(tmp_path):
    assert _espera(tmp_path, hist(SENTENCIA), rit='p-1-2026').rules == ['HISTORIA.P_FINALIZADA']


@pytest.mark.parametrize('cobertura', ['FALLIDA', 'NO_CONSULTADA', 'CANCELADA'])
def test_sin_cobertura_de_presencia_rigen_las_reglas_originales(tmp_path, cobertura):
    base = _espera(tmp_path, None)
    f = _espera(tmp_path, hist(SENTENCIA, cobertura=cobertura))
    assert (f.observation, f.rules) == (base.observation, base.rules)


def test_cobertura_parcial_alcanza(tmp_path):
    assert _espera(tmp_path, hist(SENTENCIA, cobertura='PARCIAL')).rules == ['HISTORIA.P_FINALIZADA']


def test_sin_historia_rigen_las_reglas_originales(tmp_path):
    assert 'HISTORIA.P_FINALIZADA' not in _espera(tmp_path, None).rules


def test_la_mayoria_de_edad_va_sobre_la_constancia(tmp_path):
    f = _espera(tmp_path, hist(SENTENCIA), extra=['2000-01-01'], encabezados=['FEC. NACIMIENTO'])
    assert f.rules == ['COMUN.MAYORIA_EDAD'] and [a[:2] for a in f.avisos] == [(3, 'REVISAR_MAYORIA')]


def test_cumplimiento(tmp_path):
    datos = fila(1, rit='P-1-2026') + cumplimiento(1)[5:]
    f = cum(tmp_path, datos, hist(CONCILIACION))
    assert f.rules == ['HISTORIA.P_FINALIZADA'] and f.observation.endswith(': ' + TEXTO)
    ok = cum(tmp_path, datos, hist(CONCILIACION, (EGRESO, '2026-10-03', 41)))
    assert 'HISTORIA.P_FINALIZADA' not in ok.rules
    otra = cum(tmp_path, fila(1, rit='C-1-2026') + cumplimiento(1)[5:], hist(CONCILIACION))
    assert 'HISTORIA.P_FINALIZADA' not in otra.rules


def test_informes(tmp_path):
    datos = fila(1, rit='P-1-2026') + ['2026-09-20']
    f = inf(tmp_path, datos, hist(SENTENCIA))
    assert f.rules == ['HISTORIA.P_FINALIZADA'] and f.observation.endswith(': ' + TEXTO)
    ok = inf(tmp_path, datos, hist(SENTENCIA, (EGRESO, '2026-10-02', 40)))
    assert 'HISTORIA.P_FINALIZADA' not in ok.rules


def test_cumplimiento_la_mayoria_de_edad_va_sobre_la_constancia(tmp_path):
    datos = fila(1, rit='P-1-2026') + cumplimiento(1, nac='2000-01-01')[5:]
    f = cum(tmp_path, datos, hist(CONCILIACION))
    assert f.rules == ['COMUN.MAYORIA_EDAD']
