"""NOMENCLATURA de proyectos NOMENCL según Python (`resolutions.project_nomenclature`), con filas ficticias.

Sin red ni archivos del usuario. Escribe rust/crates/flow-productos/tests/datos/nomenclatura_python.json, que coteja
la prueba `nomenclatura_como_python` de flow-productos.
"""
import json
import sys
from pathlib import Path
from types import SimpleNamespace

RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'src')]
from nurus.personal.resolutions import observed_nomenclature, project_nomenclature  # noqa: E402

C10 = 'HISTORIA.C10_ESCRITO'
FIN = 'HISTORIA.P_FINALIZADA'


def fila(observacion='', res=None, reglas=(), acciones=()):
    return {'observacion': observacion, 'res': res, 'reglas': list(reglas), 'acciones': list(acciones)}


CASOS = [
    # tests/test_nomenclatura_proyecto_20261116.py
    ('res_egreso', [fila('Causa finalizada, aplicar egreso.', 'NOMENCL')]),
    ('res_informe_avance', [fila('Informe de avance a folio 12.', 'NOMENCL')]),
    ('comillas_angulares', [fila('Se remite proyecto que aplica nomenclatura «Ingreso informe de diagnóstico» para regularizar.', 'NOMENCL')]),
    ('comillas_res_x', [fila('Se remite proyecto de nomenclatura «Egreso de servicio protección especializada y otras redes».', 'X')]),
    ('regla_c10', [fila('Texto.', 'NOMENCL', [C10])]),
    ('regla_finalizada', [fila('Texto.', 'NOMENCL', [FIN])]),
    ('observacion_prevalece', [fila('Se recepciona informe de cumplimiento.', 'NOMENCL', [FIN])]),
    ('ambigua', [fila('Informe de avance; evaluar egreso.', 'NOMENCL')]),
    ('ajena', [fila('Texto.', 'NOMENCL')]),
    ('ambigua_no_cae_a_regla', [fila('Informe de avance; evaluar egreso.', 'NOMENCL', [C10])]),
    ('egreso_solo', [fila('Egreso.', 'NOMENCL')]),
    ('informe_avance_mulchen', [fila('Informe de avance.', 'NOMENCL')]),
    # Variantes ficticias propias.
    ('comillas_rectas', [fila('Aplicar NOMENCLATURA "  Ingreso   informe de diagnóstico  " por favor.', 'NOMENCL')]),
    ('comillas_curvas', [fila('nomenclatura“Egreso por cumplimiento de objetivos”', 'NOMENCL')]),
    ('comillas_mezcladas', [fila('Nomenclatura «Egreso ficticio" y egreso.', 'NOMENCL')]),
    ('comillas_vacias', [fila('Nomenclatura «» pendiente; aplicar egreso.', 'NOMENCL')]),
    ('comillas_solo_espacios', [fila('Nomenclatura «   » y aplicar egreso.', 'NOMENCL', [C10])]),
    ('comillas_con_salto', [fila('Nomenclatura «Ingreso\ninforme» egreso.', 'NOMENCL')]),
    ('ingreso_informe_con_tilde', [fila('INGRESO INFORME ficticio.', 'NOMENCL')]),
    ('egreso_con_mayusculas', [fila('EGRESÓ del programa ficticio.', 'NOMENCL')]),
    ('solo_informe_no_basta', [fila('Pedir informe ficticio.', 'NOMENCL')]),
    ('vacia_con_regla', [fila('', None, [C10], ['NOMENCL'])]),
    ('ambas_reglas', [fila('Texto.', None, [C10, FIN], ['NOMENCL'])]),
    ('sin_nada', [fila('', None)]),
    ('guion_entre_palabras', [fila('Ingreso-informe ficticio.', 'NOMENCL')]),
    ('puntos_entre_palabras', [fila('Informe.de.avance ficticio.', 'NOMENCL')]),
    ('tabulador_y_nbsp', [fila('Informe\tde\u00a0cumplimiento ficticio.', 'NOMENCL')]),
    ('egreso_dentro_de_palabra', [fila('Reegresado ficticio.', 'NOMENCL')]),
    # Orden: primero la fila con RES NOMENCL, luego la de la acción NOMENCL, luego el resto.
    ('orden_res_primero', [fila('Aplicar egreso.'), fila('Informe de avance.', 'NOMENCL')]),
    ('orden_accion_antes_que_resto', [fila('Aplicar egreso.'), fila('', None, [C10], ['NOMENCL'])]),
    ('orden_res_texto_marcado', [fila('Aplicar egreso.', 'x'), fila('Informe de avance.', 'nomenclatura')]),
    ('orden_res_pc_ie_no_cuenta', [fila('Aplicar egreso.', 'PC_IE'), fila('Informe de avance.', None, (), ['NOMENCL'])]),
    ('ambigua_salta_a_la_siguiente', [fila('Informe de avance; evaluar egreso.', 'NOMENCL', [C10]), fila('Aplicar egreso.')]),
    ('resto_en_orden', [fila('Texto.'), fila('Texto.', None, [FIN]), fila('Informe de avance.')]),
]


def main():
    casos = []
    for nombre, filas in CASOS:
        rows = []
        for f in filas:
            review = {} if f['res'] is None else {'RES': f['res']}
            rows.append(SimpleNamespace(review=review, observation=f['observacion'], rules=f['reglas'], actions=f['acciones']))
        observadas = [observed_nomenclature(r) for r in rows]
        casos.append({'nombre': nombre, 'filas': filas, 'observadas': observadas, 'esperado': project_nomenclature(rows)})
    salida = RAIZ / 'rust/crates/flow-productos/tests/datos/nomenclatura_python.json'
    salida.write_text(json.dumps(casos, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(f'{len(casos)} casos de nomenclatura cotejados con Python.')


if __name__ == '__main__':
    main()
