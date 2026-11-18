"""Extracto ANÓNIMO de la pestaña Escritos y de la tabla Ingreso Histórico Protección, para decidir cómo asociar un escrito
con su informe (plan 18-nov-2026). Ejecutar localmente sobre el HTML real guardado; entregar solo la salida (nunca el HTML).

    python descargador/extraer_estructura_escritos.py ESCRITOS.html [HISTORIAL.html] > estructura_escritos.json

No emite valores: solo nombres de columnas, cantidades, etiquetas HTML, NOMBRES de atributos, nombre de la función de un
`onclick` y el TIPO de cada argumento (vacío, numérico, hex/base64 largo, texto). Sin RUT, nombres, folios, rutas ni parámetros.
"""

import json
import re
import sys
from pathlib import Path

from lxml import html

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'src'))
from nurus.bitacora_html import decode_snapshot, normalize  # noqa: E402


def tipo_valor(valor):
    valor = str(valor or '')
    if not valor:
        return 'vacio'
    if valor.isdigit():
        return f'numerico({len(valor)})'
    if re.fullmatch(r'[A-Za-z0-9+/=%]{16,}', valor):
        return f'opaco({len(valor)})'
    return 'texto'


def forma_enlace(nodo):
    salida = {'etiqueta': nodo.tag, 'atributos': sorted(nodo.attrib)}
    onclick = nodo.get('onclick') or ''
    funcion = re.match(r'\s*([A-Za-z_][\w.]*)\s*\((.*)\)', onclick, re.S)
    if funcion:
        argumentos = [a.strip().strip('\'"') for a in funcion.group(2).split(',')] if funcion.group(2).strip() else []
        salida['onclick'] = {'funcion': funcion.group(1), 'argumentos': [tipo_valor(a) for a in argumentos]}
    href = nodo.get('href')
    if href is not None:
        esquema = href.split(':', 1)[0].lower() if re.match(r'^[A-Za-z]+:', href) else ('ancla' if href.startswith('#') else 'relativo')
        parametros = re.findall(r'[?&]([^=&#]+)=([^&#]*)', href)
        salida['href'] = {'esquema': esquema, 'parametros': [{'nombre': n, 'tipo': tipo_valor(v)} for n, v in parametros]}
    return salida


def tablas(documento, requeridos):
    pedidos = [normalize(r) for r in requeridos]
    for tabla in documento.xpath('//table'):
        filas = tabla.xpath('./thead/tr | ./tr | ./tbody/tr')
        for i, fila in enumerate(filas):
            celdas = fila.xpath('./th | ./td')
            cabecera = [normalize(' '.join(c.text_content().split())) for c in celdas]
            if all(p in cabecera for p in pedidos):
                yield cabecera, [c.text_content() for c in celdas], filas[i + 1 :]
                break


def extraer(ruta, requeridos, etiqueta):
    documento = html.fromstring(decode_snapshot(Path(ruta).read_bytes()))
    resultado = {'fuente': etiqueta, 'tablas': []}
    for cabecera, originales, filas in tablas(documento, requeridos):
        item = {'columnas': [' '.join(o.split()) for o in originales], 'filas_de_datos': len(filas), 'filas': []}
        for fila in filas[:5]:
            celdas = fila.xpath('./td | ./th')
            item['filas'].append(
                {
                    'celdas': len(celdas),
                    'vacias': sum(not ' '.join(c.text_content().split()) for c in celdas),
                    'enlaces_por_celda': [
                        [forma_enlace(n) for n in c.xpath('.//a | .//img | .//*[@onclick]')] for c in celdas
                    ],
                    'minusculas': [
                        (t := ' '.join(c.text_content().split())) != '' and t == t.lower() for c in celdas
                    ],
                }
            )
        resultado['tablas'].append(item)
    return resultado


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv:
        print(__doc__)
        return 2
    salida = [extraer(argv[0], ['Doc.', 'Tipo Escrito', 'Solicitante', 'Estado'], 'escritos')]
    if len(argv) > 1:
        salida.append(extraer(argv[1], ['Nombre Proyecto', 'Fecha egreso', 'Causal egreso'], 'historico_proteccion'))
    print(json.dumps(salida, ensure_ascii=False, indent=1))
    return 0


if __name__ == '__main__':
    sys.exit(main())
