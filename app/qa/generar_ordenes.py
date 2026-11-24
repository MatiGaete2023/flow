"""E-A5 · Oráculo de los avisos de órdenes de búsqueda OB-01..OB-05 de Flow Python (`ordenes_busqueda.avisos_orden`).

Uso (desde la raíz): python app/qa/generar_ordenes.py [salida.json]   (por omisión app/tests/datos/ordenes.json)
"""

import json
import sys
from datetime import date
from pathlib import Path

sys.stdout.reconfigure(encoding='utf-8')
RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'src')]

from nurus.personal.ordenes_busqueda import avisos_orden  # noqa: E402

CORTE = date(2026, 11, 20)
BASE = {'ESTADO O B': 'Activa', 'RESULTADO': '', 'FECHA TRAMITE': '10/11/2026', 'FEC NACIMIENTO': '10/05/2012',
        'ESTADO ENVIO': 'Realizada', 'TIPO LITIGANTE': 'Niño', 'CURADOR': '(13624088-9)Persona Ficticia', 'PROX AUDS': ''}


def caso(nombre, **cambios):
    valores = dict(BASE)
    for clave, valor in cambios.items():
        clave = clave.replace('__', ' ')
        if valor is None:
            valores.pop(clave, None)
        else:
            valores[clave] = valor
    return {'nombre': nombre, 'valores': valores, 'avisos': avisos_orden(valores, CORTE)}


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'ordenes.json'
    casos = [
        caso('sin_avisos'),
        caso('sin_resultado_hace_mas_de_60_dias', FECHA__TRAMITE='01/09/2026'),
        caso('sin_resultado_hace_60_dias_exactos', FECHA__TRAMITE='21/09/2026'),
        caso('sin_resultado_hace_61_dias', FECHA__TRAMITE='20/09/2026'),
        caso('con_resultado', RESULTADO='Ubicado', FECHA__TRAMITE='01/01/2026'),
        caso('resultado_con_guiones', RESULTADO='---', FECHA__TRAMITE='01/01/2026'),
        caso('mayor_de_edad', FEC__NACIMIENTO='10/05/2005'),
        caso('cumple_18_hoy', FEC__NACIMIENTO='20/11/2008'),
        caso('nacida_el_29_de_febrero', FEC__NACIMIENTO='29/02/2008'),
        caso('envio_pendiente', ESTADO__ENVIO='Pendiente'),
        caso('envio_vacio', ESTADO__ENVIO=''),
        caso('sin_curador_nino', CURADOR='', TIPO__LITIGANTE='Niño'),
        caso('sin_curador_adolescente', CURADOR='---', TIPO__LITIGANTE='Adolescente'),
        caso('sin_curador_adulto', CURADOR='', TIPO__LITIGANTE='Adulto'),
        caso('audiencia_futura', PROX__AUDS='25/11/2026'),
        caso('audiencias_varias', PROX__AUDS='10/11/2026, 05/12/2026, 25/11/2026'),
        caso('audiencia_pasada', PROX__AUDS='10/11/2026'),
        caso('orden_no_activa', ESTADO__O__B='Cerrada', RESULTADO='', FECHA__TRAMITE='01/01/2026', FEC__NACIMIENTO='10/05/2005'),
        caso('sin_columna_de_estado', ESTADO__O__B=None),
        caso('columnas_ausentes', RESULTADO=None, FECHA__TRAMITE=None, FEC__NACIMIENTO=None, ESTADO__ENVIO=None, PROX__AUDS=None),
        caso('titulos_con_tildes_y_puntos', **{}),
    ]
    # Títulos que llegan con otra escritura (tildes, puntos y espacios) siguen siendo las mismas columnas.
    alternos = {'ESTADO O B': 'Estado O.B.', 'RESULTADO': 'Resultado', 'FECHA TRAMITE': 'Fecha Trámite', 'FEC NACIMIENTO': 'Fec. Nacimiento',
                'ESTADO ENVIO': 'Estado Envío', 'TIPO LITIGANTE': 'Tipo Litigante', 'CURADOR': 'Curador', 'PROX AUDS': 'Prox. Auds.'}
    valores = {alternos[k]: v for k, v in BASE.items()}
    casos[-1] = {'nombre': 'titulos_con_tildes_y_puntos', 'valores': valores, 'avisos': avisos_orden(valores, CORTE)}
    destino.write_bytes((json.dumps({'corte': CORTE.isoformat(), 'casos': casos}, ensure_ascii=False, indent=1) + '\n').encode('utf-8'))
    for c in casos:
        print(c['nombre'], [a[:5] for a in c['avisos']])


if __name__ == '__main__':
    main()
