"""WP-R2: el scope HISTORIA del catálogo de textos coincide con el contrato del plan."""

import json
import string

from nurus.personal.config import BASE, Configuration, defaults, validate

ESPERADO = {
    'BASE_TRAMITE': {'DESC', 'FECHA', 'FOLIO'},
    'VARIOS': {'DESC', 'FECHA', 'FOLIO', 'N'},
    'MEDIDA_REVISADA': set(),
    'SOLICITUD_DCE': {'FECHA', 'FOLIO'},
    'CONTINUIDAD_ATENCION': {'FECHA', 'FOLIO'},
    'INFORME_INGRESADO': {'FECHA', 'FOLIO'},
    'E04_CON_FOLIO': {'FECHA_RESOLUCION', 'FOLIO', 'PROGRAMA'},
    'R3_MULTI': set(),
    'REITERA': {'FECHA_ANTERIOR', 'MATERIA'},
    'MATERIA_CURADOR': set(),
    'MATERIA_SIN_FICHA': set(),
    'MATERIA_SIN_FAE': set(),
    'MATERIA_VENCIDA': set(),
    'E05_DIAS': {'CORTE', 'N'},
    'E10_LISTA': {'N'},
    'MEDIDA_SIN_VIGENCIA': {'FECHA_EGRESO_PROYECTADO'},
    'C2_PRORROGA_HASTA': {'FECHA', 'FECHA_EGRESO_PROYECTADO', 'FOLIO'},
    'INGRESO_INFORMADO': {'FECHA_INGRESO'},
    'SERVICIO_INGRESO': {'FECHA', 'FOLIO', 'PROGRAMA'},
    'CURADOR_SIN_EFECTO': {'FECHA', 'FOLIO'},
    'CURADOR_DESIGNADO': {'FECHA', 'FOLIO'},
    'EGRESO_POSTERIOR_MAYORIA': set(),
    'EGRESO_POSTERIOR_MAYORIA_FECHA': {'FECHA_MAYORIA'},
    'EGRESO_PROYECTADO': {'FECHA_EGRESO_PROYECTADO'},
    'P_FINALIZADA': set(),
    'C10_ESCRITO': {'FECHA_ESCRITO', 'FOLIO_ESCRITO'},
    'J2_ACUMULADOS': {'FECHA', 'N'},
    'I01_SIN_ENVIO_GENERAL': {'FECHA_VENCIMIENTO'},
    'I01_SIN_ENVIO_DCE': {'FECHA_VENCIMIENTO'},
    'R11_ORDEN': {'FECHA', 'FOLIO'},
    'R11_PIDE_CUENTA': {'FECHA', 'FOLIO'},
    'R11_SIN_EFECTO': {'FECHA', 'FOLIO'},
    'R11_TERMINO': {'FECHA', 'FOLIO'},
    'T5_INSTITUCIONES': {'N', 'PNOMBRE'},
}


def _campos(texto):
    return {campo for _, campo, _, _ in string.Formatter().parse(texto) if campo}


def test_claves_y_variables_coinciden_exactamente():
    textos = defaults()['textos']['HISTORIA']
    assert set(textos) == set(ESPERADO)
    for clave, entrada in textos.items():
        assert _campos(entrada['texto']) == ESPERADO[clave], clave
        assert entrada['confirmado'] is True and entrada['nota'], clave


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
