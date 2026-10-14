"""WP-R2: el scope HISTORIA del catálogo de textos coincide con el contrato del plan."""

import json
import string

from nurus.personal.config import BASE, Configuration, defaults, validate

ESPERADO = {
    'BASE_TRAMITE': {'DESC', 'FECHA', 'FOLIO'},
    'VARIOS': {'DESC', 'FECHA', 'FOLIO', 'N'},
    'MEDIDA_REVISADA': set(),
    'E04_CON_FOLIO': {'FECHA_RESOLUCION', 'FOLIO', 'PROGRAMA'},
    'R3_CIERRE': {'CORTE'},
    'R3_MULTI': set(),
    'REITERA': {'FECHA_ANTERIOR', 'MATERIA'},
    'MATERIA_CURADOR': set(),
    'MATERIA_SIN_FICHA': set(),
    'MATERIA_SIN_FAE': set(),
    'MATERIA_VENCIDA': set(),
    'E05_DIAS': {'CORTE', 'N'},
    'E10_LISTA': {'N'},
    'MEDIDA_SIN_VIGENCIA': {'FECHA_EGRESO_PROYECTADO'},
    'C1_RUS': {'FECHA_INGRESO'},
    'C2_PRORROGA': {'CORTE', 'FECHA', 'FECHA_EGRESO_PROYECTADO', 'FOLIO'},
    'C3_CON_FOLIO': {'FECHA_INGRESO', 'FOLIO', 'PROGRAMA'},
    'EGRESO_PROYECTADO': {'FECHA_EGRESO_PROYECTADO'},
    'C4_FICHA_RUS': {'FECHA_FICHA'},
    'C4_FICHA_SIN_RUS': set(),
    'C4_FOLIO': {'FOLIO'},
    'C8_NO_USO': {'DESDE', 'FECHA', 'HASTA'},
    'C9_DCE': {'FECHA', 'FOLIO'},
    'P_FINALIZADA': set(),
    'C10_ESCRITO': {'FECHA_ESCRITO', 'FOLIO_ESCRITO'},
    'I1_VENCIDO': {'FECHA_VENCIMIENTO'},
    'I1_POR_VENCER': {'FECHA_VENCIMIENTO'},
    'J2_ACUMULADOS': {'FECHA', 'N'},
    'I01_SIN_ENVIO_GENERAL': {'FECHA_VENCIMIENTO'},
    'I01_SIN_ENVIO_DCE': {'FECHA_VENCIMIENTO'},
    'R11_TERMINO': {'DESC', 'FECHA', 'FECHA_ORDEN', 'FOLIO', 'FOLIO_ORDEN'},
    'R11_SIN_EFECTO': {'DESC', 'FECHA', 'FECHA_ORDEN', 'FOLIO', 'FOLIO_ORDEN'},
    'T2_MN': {'DESC_ANTERIOR', 'FECHA', 'FECHA_ANTERIOR', 'FOLIO', 'FOLIO_ANTERIOR'},
    'T5_INSTITUCIONES': {'N', 'PNOMBRE'},
}


def _campos(texto):
    return {campo for _, campo, _, _ in string.Formatter().parse(texto) if campo}


def test_claves_y_variables_coinciden_exactamente():
    textos = defaults()['textos']['HISTORIA']
    assert set(textos) == set(ESPERADO)
    for clave, entrada in textos.items():
        assert _campos(entrada['texto']) == ESPERADO[clave], clave
        assert entrada['confirmado'] is False and entrada['nota'], clave


def test_la_configuracion_valida_y_una_guardada_sin_el_scope_queda_completa(tmp_path):
    validate(defaults())
    conf = Configuration(tmp_path / 'config')
    datos = json.loads(json.dumps(conf.data))
    datos['textos'].pop('HISTORIA')
    conf.path.write_text(json.dumps(datos, ensure_ascii=False), encoding='utf-8')
    assert set(Configuration(conf.directory).data['textos']['HISTORIA']) == set(ESPERADO)


def test_el_catalogo_rus_espeja_los_textos_de_personal():
    personal = json.loads((BASE / 'textos_base.json').read_text(encoding='utf-8'))
    rus = json.loads((BASE.parent / 'rus' / 'textos_observaciones.json').read_text(encoding='utf-8'))
    assert {k: {c: e['texto'] for c, e in v.items()} for k, v in rus.items()} == {
        k: {c: e['texto'] for c, e in v.items()} for k, v in personal.items() if not k.startswith('_')
    }
