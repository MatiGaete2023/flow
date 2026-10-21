"""R-T01 y R-T04 a R-T08: reglas tradicionales de Espera con fecha de corte fija."""

import pytest

from test_mayoria_edad import analizar, persona

BASE = ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'NOMBRE CENTRO', 'T ESPERA']
CORREO, PROYECTO = 'programa_espera', 'PC_IE'


def _espera(tmp_path, casos, encabezados=BASE):
    filas = [persona(n, tribunal, programa) + list(extra) for n, (tribunal, programa, *extra) in enumerate(casos, 1)]
    return analizar(tmp_path, 'ESPERA', encabezados, filas).rows


def test_rt01_derivacion_sin_seguimiento_solo_si_coincide_al_inicio(tmp_path):
    filas = _espera(tmp_path, [('LAJA', 'OPD COMUNAL', 90), ('LAJA', 'PRM OPDIMA', 90)])
    assert filas[0].rules == ['COMUN.NO_SEGUIMIENTO'] and filas[0].actions == []
    assert 'COMUN.NO_SEGUIMIENTO' not in filas[1].rules
    assert filas[1].rules == ['ESPERA.E05_PROYECTO_Y_CORREO', 'HISTORIA.E05_DIAS']


def test_rt04_dce_solo_correo_desde_el_primer_dia_y_sin_word(tmp_path):
    filas = _espera(tmp_path, [('LAJA', 'DCE UNO', 29), ('LAJA', 'DCE UNO', 30), ('MULCHEN', 'DCE UNO', 31)])
    for fila in filas:
        assert fila.rules == ['ESPERA.E05_SOLO_CORREO', 'HISTORIA.E05_DIAS']
        assert fila.actions == [CORREO]


@pytest.mark.parametrize('tribunal', ['LAJA', 'MULCHEN'])
def test_rt05_laja_mulchen_tramo_exacto(tmp_path, tribunal):
    filas = _espera(tmp_path, [(tribunal, 'PRM UNO', 29), (tribunal, 'PRM UNO', 30)])
    assert filas[0].rules == ['ESPERA.E06_SIN_RESOLUCION'] and filas[0].actions == []
    assert filas[1].rules == ['ESPERA.E05_PROYECTO_Y_CORREO', 'HISTORIA.E05_DIAS']
    assert filas[1].actions == [CORREO, PROYECTO]


def test_rt06_tome_cortes_29_30_59_60(tmp_path):
    filas = _espera(tmp_path, [('TOME', 'PRM UNO', dias) for dias in (29, 30, 59, 60)])
    assert [f.rules for f in filas] == [
        ['ESPERA.E06_SIN_RESOLUCION'],
        ['ESPERA.E06_SIN_RESOLUCION'],
        ['ESPERA.E06_SIN_RESOLUCION'],
        ['ESPERA.E05_PROYECTO_Y_CORREO', 'HISTORIA.E05_DIAS'],
    ]
    assert [f.actions for f in filas] == [[], [], [], [CORREO, PROYECTO]]


@pytest.mark.parametrize('valor', ['', 'sin dato', -3])
def test_rt07_espera_invalida_genera_incidencia_y_fallback(tmp_path, valor):
    fila = _espera(tmp_path, [('LAJA', 'PRM UNO', valor)])[0]
    assert fila.rules == ['ESPERA.E06_SIN_RESOLUCION'] and fila.actions == []
    assert any('tiempo de espera ausente o inválido' in aviso for aviso in fila.warnings)


def test_rt08_resolucion_reciente_solo_antes_de_30_dias(tmp_path):
    encabezados = BASE + ['FEC. RESOLUCION']
    filas = _espera(tmp_path, [('LAJA', 'PRM UNO', 5, '2026-09-10'), ('LAJA', 'PRM UNO', 5, '2026-09-09')], encabezados)
    assert filas[0].rules == ['ESPERA.E04_RESOLUCION_RECIENTE']
    assert '10 de septiembre de 2026' in filas[0].observation
    assert filas[1].rules == ['ESPERA.E06_SIN_RESOLUCION']
