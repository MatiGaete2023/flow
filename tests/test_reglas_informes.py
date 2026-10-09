"""R-T16, R-T27 y R-T28: Informes y casos transversales con fecha de corte fija."""
from reglas_rt_util import analizar, persona

BASE = ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'NOMBRE CENTRO', 'FECHA VENCIMIENTO']


def test_rt16_informe_vence_ayer_hoy_30_31(tmp_path):
    casos = ['2026-10-08', '2026-10-09', '2026-11-08', '2026-11-09']
    filas = analizar(tmp_path, 'INFORMES', BASE, [persona(n) + [fecha] for n, fecha in enumerate(casos, 1)]).rows
    assert [f.rules for f in filas] == [['INFORMES.I01_VENCIDO_GENERAL'], ['INFORMES.I02_POR_VENCER_GENERAL'],
                                        ['INFORMES.I02_POR_VENCER_GENERAL'], []]
    assert filas[0].actions == ['programa_vencido', 'PC_INFO']
    assert filas[1].actions == ['programa_por_vencer']
    assert filas[3].observation == '' and filas[3].actions == []


def test_rt16_dce_distingue_informe_diagnostico(tmp_path):
    fila = analizar(tmp_path, 'INFORMES', BASE, [persona(1, programa='DCE UNO') + ['2026-10-08']]).rows[0]
    assert fila.rules == ['INFORMES.I01_VENCIDO_DCE']


def test_rt16_sin_fecha_no_emite_frase_e_informa_incidencia(tmp_path):
    fila = analizar(tmp_path, 'INFORMES', BASE, [persona(1) + ['']]).rows[0]
    assert fila.rules == [] and fila.observation == ''
    assert any('sin fecha de vencimiento' in aviso for aviso in fila.warnings)


def test_rt27_regla_valida_con_dato_faltante_de_otra(tmp_path):
    encabezados = ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'NOMBRE CENTRO', 'DIAS DE CUMPLIMIENTO', 'DIAS PARA EGRESAR',
                   'FEC. INGRESO EFECTIVO', 'FEC. EGRESO PROYECTADO']
    fila = analizar(tmp_path, 'CUMPLIMIENTO', encabezados,
                    [persona(1) + ['sin dato', 10, '2026-01-01', '2026-10-19']]).rows[0]
    assert fila.rules == ['CUMPLIMIENTO.C05_POR_VENCER']
    assert '19 de octubre de 2026' in fila.observation
    assert any('días de cumplimiento ausentes o inválidos' in aviso for aviso in fila.warnings)


def test_rt28_tribunal_no_soportado_sin_textos_inventados(tmp_path):
    encabezados = ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'NOMBRE CENTRO', 'T ESPERA']
    fila = analizar(tmp_path, 'ESPERA', encabezados, [persona(1, tribunal='TRIBUNAL FICTICIO') + [90]]).rows[0]
    assert fila.rules == [] and fila.observation == '' and fila.actions == []
    assert any('Tribunal fuera de Laja, Mulchén y Tomé' in aviso for aviso in fila.warnings)
    assert fila.values['TRIBUNAL'] == 'TRIBUNAL FICTICIO'
