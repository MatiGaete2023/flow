"""R-T09 a R-T15: reglas tradicionales de Cumplimiento con fecha de corte fija."""
import pytest

from reglas_rt_util import analizar, persona

BASE = ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'NOMBRE CENTRO', 'DIAS DE CUMPLIMIENTO', 'DIAS PARA EGRESAR',
        'FEC. INGRESO EFECTIVO', 'FEC. EGRESO PROYECTADO']
LEJANO = '2027-05-01'


def _cumplimiento(tmp_path, casos, encabezados=BASE, cruce=None, programa='PRM UNO'):
    filas = [persona(n, programa=programa) + list(extra) for n, extra in enumerate(casos, 1)]
    return analizar(tmp_path, 'CUMPLIMIENTO', encabezados, filas, cruce=cruce)


def test_rt09_ingreso_reciente_solo_en_intervalo_y_con_fecha(tmp_path):
    filas = _cumplimiento(tmp_path, [(0, 100, '2026-10-09', LEJANO), (30, 100, '2026-09-09', LEJANO),
                                     (31, 100, '2026-09-08', LEJANO), (10, 100, '', LEJANO)]).rows
    assert [f.rules for f in filas[:3]] == [['CUMPLIMIENTO.C03_INGRESO_RECIENTE']] * 2 + [['CUMPLIMIENTO.C09_SIN_OBSERVACIONES']]
    assert filas[3].rules == ['CUMPLIMIENTO.C09_SIN_OBSERVACIONES']
    assert any('sin fecha de ingreso' in aviso for aviso in filas[3].warnings)


def test_rt10_dias_para_egresar_cortes(tmp_path):
    filas = _cumplimiento(tmp_path, [(100, -1, '2026-01-01', '2026-10-08'), (100, 0, '2026-01-01', '2026-10-09'),
                                     (100, 45, '2026-01-01', '2026-11-23'), (100, 46, '2026-01-01', '2026-11-24')]).rows
    assert [f.rules for f in filas] == [['CUMPLIMIENTO.C04_VENCIDA'], ['CUMPLIMIENTO.C05_VENCE_HOY'],
                                        ['CUMPLIMIENTO.C05_POR_VENCER'], ['CUMPLIMIENTO.C09_SIN_OBSERVACIONES']]
    assert '8 de octubre de 2026' in filas[0].observation and 'hoy' in filas[1].observation


def test_rt11_dias_negativos_con_egreso_futuro_no_afirma_vencimiento(tmp_path):
    fila = _cumplimiento(tmp_path, [(100, -5, '2026-01-01', '2026-12-30')]).rows[0]
    assert 'CUMPLIMIENTO.C04_VENCIDA' not in fila.rules
    assert 'vencida' not in fila.observation.lower()
    assert any('contradicción' in aviso for aviso in fila.warnings)


def test_rt12_ficha_residencial_cortes(tmp_path):
    encabezados = BASE + ['FEC. ACT. F. INDIVIDUAL']
    fichas = ['2026-09-09', '2026-09-08', '2026-04-12', '2026-04-11', None]
    filas = _cumplimiento(tmp_path, [(100, 100, '2026-01-01', LEJANO, ficha) for ficha in fichas],
                          encabezados, programa='RES UNO').rows
    assert [f.rules for f in filas] == [['CUMPLIMIENTO.C07_FICHA_RECIENTE'], ['CUMPLIMIENTO.C09_SIN_OBSERVACIONES'],
                                        ['CUMPLIMIENTO.C09_SIN_OBSERVACIONES'], ['CUMPLIMIENTO.C07_FICHA_ANTIGUA'],
                                        ['CUMPLIMIENTO.C07_SIN_FICHA']]


def test_rt13_fae_sin_ficha_con_columna(tmp_path):
    encabezados = BASE + ['FEC. ACT. F. FAE']
    filas = _cumplimiento(tmp_path, [(200, 100, '2026-01-01', LEJANO, None), (100, 100, '2026-07-01', LEJANO, None)],
                          encabezados, programa='FAE UNO').rows
    assert 'CUMPLIMIENTO.C08_FICHA_FAE' in filas[0].rules
    assert 'CUMPLIMIENTO.C08_FICHA_FAE' not in filas[1].rules


@pytest.mark.xfail(strict=True, reason='[CONFLICTO_ABIERTO] 03 §6 C-08: sin columna de ficha FAE el código afirma '
                   '«no registra ficha»; el documento pide NO_EVALUABLE. No se cambia la regla sin decisión del usuario.')
def test_rt13_fae_sin_columna_no_afirma_ausencia(tmp_path):
    fila = _cumplimiento(tmp_path, [(200, 100, '2026-01-01', LEJANO)], programa='FAE UNO').rows[0]
    assert 'CUMPLIMIENTO.C08_FICHA_FAE' not in fila.rules


CRUCE = ['RIT', 'RUT', 'NOMBRE', 'TRIBUNAL', 'NOMBRE CENTRO', 'FECHA VENCIMIENTO']


def test_rt14_hoja2_duplicada_no_cruza_otro_programa(tmp_path):
    cruce = [CRUCE,
             ['X-1-2026', '11111111-1', 'Persona Ficticia 1', 'LAJA', 'PRM UNO', '2026-12-01'],
             ['X-1-2026', '11111111-1', 'Persona Ficticia 1', 'LAJA', 'PRM UNO', '2027-01-15'],
             ['X-1-2026', '11111111-1', 'Persona Ficticia 1', 'LAJA', 'PRM OTRO', '2027-02-15'],
             ['X-2-2026', '11111111-1', 'Persona Ficticia 2', 'LAJA', 'PRM OTRO', '2027-03-15']]
    trabajo = _cumplimiento(tmp_path, [(100, 100, '2026-01-01', LEJANO), (100, 100, '2026-01-01', LEJANO)], cruce=cruce)
    primera, segunda = trabajo.rows
    assert primera.rules == ['CUMPLIMIENTO.C10_HOJA2'] and '15 de enero de 2027' in primera.observation
    assert 'CUMPLIMIENTO.C10_HOJA2' not in segunda.rules
    assert any('Cruce duplicado' in aviso for aviso in trabajo.warnings)


def test_rt15_c04_c05_suprimen_c10(tmp_path):
    cruce = [CRUCE] + [[f'X-{n}-2026', '11111111-1', f'Persona Ficticia {n}', 'LAJA', 'PRM UNO', '2026-12-01'] for n in (1, 2)]
    filas = _cumplimiento(tmp_path, [(100, -1, '2026-01-01', '2026-10-08'), (100, 10, '2026-01-01', '2026-10-19')],
                          cruce=cruce).rows
    assert filas[0].rules == ['CUMPLIMIENTO.C04_VENCIDA']
    assert filas[1].rules == ['CUMPLIMIENTO.C05_POR_VENCER']
