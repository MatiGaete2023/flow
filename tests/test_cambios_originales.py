"""WP-R3: cambios deliberados a reglas originales (umbrales, redacción, mayoría, sin seguimiento, C-03) y migración."""

import json

import pytest
from historia_fixtures import cum, cumplimiento, esp, fila

from nurus.personal.config import LEGACY_PROX_AUDIENCIA_REV3, TEXT_REVISION, Configuration, defaults


def _regla_espera(tmp_path, tribunal, programa, espera):
    return esp(tmp_path, fila(1, programa, tribunal) + [espera]).rules


def test_umbrales_y_revision_por_defecto():
    umbrales = defaults()['umbrales']
    assert umbrales['espera_dce'] == 0 and umbrales['espera_tome'] == 60
    assert umbrales['espera_tome'] <= umbrales['proyecto_tome']
    assert defaults()['revision_textos'] == TEXT_REVISION == 7


@pytest.mark.parametrize('espera', [0, 1, 29, 200])
def test_dce_correo_desde_el_primer_dia(tmp_path, espera):
    assert _regla_espera(tmp_path, 'LAJA', 'DCE UNO', espera) == ['ESPERA.E05_SOLO_CORREO', 'HISTORIA.E05_DIAS']


def test_dce_sin_t_espera_sigue_en_e06(tmp_path):
    assert esp(tmp_path, fila(1, 'DCE UNO', 'LAJA') + ['']).rules == ['ESPERA.E06_SIN_RESOLUCION']


@pytest.mark.parametrize('espera', [29, 30, 59])
def test_tome_bajo_60_dias_queda_en_e06(tmp_path, espera):
    assert _regla_espera(tmp_path, 'TOME', 'AFT UNO', espera) == ['ESPERA.E06_SIN_RESOLUCION']


@pytest.mark.parametrize('espera', [60, 90])
def test_tome_desde_60_dias_sale_correo_y_proyecto(tmp_path, espera):
    assert _regla_espera(tmp_path, 'TOME', 'AFT UNO', espera) == [
        'ESPERA.E05_PROYECTO_Y_CORREO',
        'HISTORIA.E05_DIAS',
    ]


def test_e05_agrega_los_dias_de_espera_con_la_fecha_de_corte(tmp_path):
    fila_ = esp(tmp_path, fila(1, 'PRM UNO', 'LAJA') + [45])
    # Constancias primero (los días de espera) y la acción del Centro al final (decisión x).
    assert fila_.observation.split(': ', 1)[1] == (
        'Medida revisada, a la espera de ingreso efectivo. Al 14 de octubre de 2026, RUS registra 45 días de espera. '
        'Se remite proyecto de resolución pidiendo cuenta del ingreso efectivo y correo al programa consultando fecha estimada de ingreso.'
    )


def test_texto_prox_audiencia():
    assert defaults()['textos']['COMUN']['PROX_AUDIENCIA']['texto'] == 'Audiencia decretada para el {FECHA_AUDIENCIA}.'


def test_espera_mayor_de_edad_sale_solo_la_mayoria_y_marca_nivel_3(tmp_path):
    r = esp(tmp_path, fila(1, 'PRM UNO', 'LAJA') + [45], extra=['2000-01-01'], encabezados=['FEC. NACIMIENTO'])
    assert r.rules == ['COMUN.MAYORIA_EDAD']
    assert [a[:2] for a in r.avisos] == [(3, 'REVISAR_MAYORIA')]


def test_cumplimiento_mayor_de_edad_marca(tmp_path):
    r = cum(tmp_path, cumplimiento(1, nac='2000-01-01'))
    assert r.rules == ['COMUN.MAYORIA_EDAD'] and [a[:2] for a in r.avisos] == [(3, 'REVISAR_MAYORIA')]


def test_cumplimiento_mayor_de_edad_marca_aunque_la_medida_este_vencida(tmp_path):
    r = cum(tmp_path, cumplimiento(1, dc=-5, dpe=-5, egreso='2026-10-01', nac='2000-01-01'))
    marcas = [a[:2] for a in r.avisos]
    assert r.rules == ['COMUN.MAYORIA_EDAD'] and marcas == [(3, 'REVISAR_MAYORIA')]


def test_sin_seguimiento_vencida_agrega_medida_sin_vigencia(tmp_path):
    vencida = cum(tmp_path, cumplimiento(1, programa='OPD UNO', dc=-5, dpe=-5, egreso='2026-10-01'))
    assert vencida.rules == ['COMUN.NO_SEGUIMIENTO', 'HISTORIA.MEDIDA_SIN_VIGENCIA']
    assert vencida.observation.endswith('Medida sin vigencia en RUS desde el 1 de octubre de 2026.')


def test_sin_seguimiento_vigente_queda_como_antes(tmp_path):
    vigente = cum(tmp_path, cumplimiento(1, programa='OPD UNO', dc=50, dpe=50, egreso='2027-05-01'))
    assert vigente.rules == ['COMUN.NO_SEGUIMIENTO']


def test_c03_agrega_egreso_proyectado(tmp_path):
    r = cum(tmp_path, cumplimiento(1, dc=10, dpe=200, ingreso='2026-10-01', egreso='2027-05-01'))
    assert r.rules == ['CUMPLIMIENTO.C03_INGRESO_RECIENTE', 'HISTORIA.EGRESO_PROYECTADO']
    assert 'RUS registra como fecha de egreso proyectado el 1 de mayo de 2027.' in r.observation


def test_c03_sin_egreso_proyectado_no_agrega_el_fragmento(tmp_path):
    r = cum(tmp_path, cumplimiento(1, dc=10, dpe='', ingreso='2026-10-01', egreso=''))
    assert r.rules == ['CUMPLIMIENTO.C03_INGRESO_RECIENTE']


def _escribir(directorio, mutar):
    base = Configuration(directorio)
    datos = json.loads(json.dumps(base.data))
    mutar(datos)
    base.path.write_text(json.dumps(datos, ensure_ascii=False), encoding='utf-8')
    return Configuration(directorio).data


def test_migracion_cambia_umbrales_y_texto_heredados(tmp_path):
    def antiguo(d):
        d['revision_textos'] = 3
        d['umbrales']['espera_dce'] = 30
        d['umbrales']['espera_tome'] = 30
        d['textos']['COMUN']['PROX_AUDIENCIA']['texto'] = LEGACY_PROX_AUDIENCIA_REV3

    datos = _escribir(tmp_path / 'a', antiguo)
    assert datos['umbrales']['espera_dce'] == 0 and datos['umbrales']['espera_tome'] == 60
    assert datos['textos']['COMUN']['PROX_AUDIENCIA']['texto'] == 'Audiencia decretada para el {FECHA_AUDIENCIA}.'
    assert datos['revision_textos'] == TEXT_REVISION


def test_migracion_respeta_valores_personalizados(tmp_path):
    def propio(d):
        d['revision_textos'] = 3
        d['umbrales']['espera_dce'] = 45
        d['umbrales']['espera_tome'] = 40
        d['textos']['COMUN']['PROX_AUDIENCIA']['texto'] = 'Texto propio {FECHA_AUDIENCIA}.'

    datos = _escribir(tmp_path / 'b', propio)
    assert datos['umbrales']['espera_dce'] == 45 and datos['umbrales']['espera_tome'] == 40
    assert datos['textos']['COMUN']['PROX_AUDIENCIA']['texto'] == 'Texto propio {FECHA_AUDIENCIA}.'


def test_migracion_limita_espera_tome_a_proyecto_tome(tmp_path):
    def acotado(d):
        d['revision_textos'] = 3
        d['umbrales']['proyecto_tome'] = 45
        d['umbrales']['espera_tome'] = 30

    assert _escribir(tmp_path / 'c', acotado)['umbrales']['espera_tome'] == 45


def test_c03_con_c05_activa_y_cruce_emite_c10_hoja2(tmp_path):
    from test_reglas_cumplimiento import CRUCE, _cumplimiento

    cruce = [CRUCE, ['X-1-2026', '11111111-0', 'Persona Ficticia 1', 'LAJA', 'PRM UNO', '2026-12-01']]
    fila_ = _cumplimiento(tmp_path, [(5, 10, '2026-10-09', '2026-10-24')], cruce=cruce).rows[0]
    # El egreso proyectado lo dice la frase de la medida por vencer: no se repite en la de ingreso reciente.
    assert fila_.rules == [
        'CUMPLIMIENTO.C03_INGRESO_RECIENTE',
        'CUMPLIMIENTO.C05_POR_VENCER',
        'CUMPLIMIENTO.C10_HOJA2',
    ]
