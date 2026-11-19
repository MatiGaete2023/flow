"""Extracto ANÓNIMO de la pestaña Escritos y de la tabla Ingreso Histórico Protección, para decidir cómo asociar un escrito
con su informe (plan 18-nov-2026). Ejecutar localmente sobre el HTML real guardado; entregar solo la salida (nunca el HTML).

    python descargador/extraer_estructura_escritos.py ESCRITOS.html [HISTORIAL.html] > estructura_escritos.json
    python descargador/extraer_estructura_escritos.py --ventana historico/historico-XXXX.html > estructura_historico.json

No emite valores: solo nombres de columnas, cantidades, etiquetas HTML, NOMBRES de atributos, nombre de la función de un
`onclick` y el TIPO de cada argumento (vacío, numérico, hex/base64 largo, texto). Sin RUT, nombres, folios, rutas ni parámetros.
"""

import json
import re
import sys
from pathlib import Path

from lxml import html

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'src'))
from nurus.bitacora_html import decode_snapshot, normalize


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
    funcion = re.match(r'\s*([A-Za-z_][\w.]*)\s*\((.*)\)', onclick, re.DOTALL)
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


def estructura_pestana(data):
    """P3b: forma ANÓNIMA de la pestaña «Escritos» cuando la respuesta de la causa la muestra sin su tabla (variante que carga
    la tabla aparte). Solo etiquetas, nombres de atributos, nombre de la función del `onclick`, tipos de argumentos, nombres
    de funciones de los scripts que mencionan escritos y rutas `.do`/`.jsp` sin valores. Sin RUT, nombres ni folios."""
    documento = html.fromstring(decode_snapshot(data))
    controles = [
        forma_enlace(el)
        for el in documento.xpath('//a | //li | //button | //span | //td | //th | //input | //div')
        if normalize(' '.join(el.text_content().split())) == 'escritos' or normalize(el.get('value', '')) == 'escritos'
    ]
    codigo = ' '.join(documento.xpath('//script/text()'))
    funciones = sorted({f for f in re.findall(r'function\s+([A-Za-z_]\w*)', codigo) if 'escrit' in f.lower()})
    rutas = sorted({r for r in re.findall(r'/[A-Za-z0-9_/]+\.(?:do|jsp)', codigo)})
    parametros = sorted({p for p in re.findall(r'[?&]([A-Za-z_]\w{0,40})=', codigo)})
    return {
        'tipo': 'ESTRUCTURA_PESTANA_ESCRITOS',
        'version': 1,
        'controles': controles[:10],
        'funciones_escritos': funciones[:20],
        'rutas_en_scripts': rutas[:40],
        'nombres_de_parametros': parametros[:60],
        'ajax': {'jquery': codigo.count('$.ajax'), 'xhr': codigo.count('XMLHttpRequest'), 'fetch': codigo.count('fetch(')},
        'contiene_identificadores_personales': False,
    }


_PALABRAS_ROTULO = (
    'cod', 'fecha', 'nombre', 'proyecto', 'tribunal', 'ruc', 'rit', 'causal', 'egreso', 'ingreso', 'entidad', 'quien',
    'programa', 'estado', 'tipo', 'rut', 'sexo', 'historial', 'historic', 'lrpa', 'proteccion', 'menores', 'escrito', 'doc',
    'solicitante', 'folio', 'tramite', 'etapa', 'referencia',
)


def _rotulo_seguro(texto, largo=60):
    """Texto de rótulo SOLO si no puede ser un dato de persona: sin dígitos, corto y sin «@»."""
    texto = ' '.join(str(texto or '').split())
    return texto if texto and len(texto) <= largo and not re.search(r'[\d@]', texto) else None


def estructura_ventana(data):
    """P3c: forma ANÓNIMA de una ventana SITFA (p. ej. «Historial Menores») cuyo lector no encontró la tabla esperada.
    Incluye título, marcos y formularios (solo rutas y nombres de campos), por tabla filas/columnas y los textos de
    encabezado sin dígitos, títulos de sección, funciones y rutas de los scripts. Nunca valores de celdas de datos."""
    from urllib.parse import urlsplit

    documento = html.fromstring(decode_snapshot(data))
    titulo = _rotulo_seguro(' '.join(documento.xpath('//title//text()')))
    marcos = [
        {'etiqueta': el.tag, 'ruta': urlsplit(el.get('src') or '').path}
        for el in documento.xpath('//iframe | //frame')
    ]
    formularios = [
        {
            'accion': urlsplit(f.get('action') or '').path,
            'metodo': (f.get('method') or '').lower(),
            'campos': sorted({c.get('name') for c in f.xpath('.//input | .//select') if c.get('name')})[:60],
        }
        for f in documento.xpath('//form')
    ]
    tablas = []
    for tabla in documento.xpath('//table'):
        filas = tabla.xpath('./thead/tr | ./tr | ./tbody/tr')
        encabezados = []
        for fila in filas[:4]:
            celdas = fila.xpath('./th | ./td')
            textos = [_rotulo_seguro(c.text_content(), 40) for c in celdas]
            # Una fila sin <th> solo cuenta como encabezado si CADA celda trae una palabra de rótulo conocida: así un
            # nombre de persona sin dígitos nunca sale en el extracto.
            rotulos = celdas and all(t and any(p in normalize(t) for p in _PALABRAS_ROTULO) for t in textos)
            if fila.xpath('./th') or rotulos:
                encabezados.append([t if t and any(p in normalize(t) for p in _PALABRAS_ROTULO) else '·' for t in textos])
        tablas.append(
            {
                'filas': len(filas),
                'columnas_max': max((len(f.xpath('./th | ./td')) for f in filas), default=0),
                'anidada': bool(tabla.xpath('ancestor::table')),
                'filas_encabezado': encabezados[:3],
            }
        )
    secciones = sorted(
        {
            t
            for el in documento.xpath('//td | //th | //div | //span | //b | //font | //h1 | //h2 | //h3 | //p | //caption')
            if (t := _rotulo_seguro(el.text_content()))
            and any(p in normalize(t) for p in ('historic', 'historial', 'ingreso', 'proteccion', 'lrpa', 'egreso'))
        }
    )[:30]
    codigo = ' '.join(documento.xpath('//script/text()'))
    return {
        'tipo': 'ESTRUCTURA_VENTANA_SITFA',
        'version': 1,
        'bytes': len(data),
        'titulo': titulo,
        'marcos': marcos[:10],
        'formularios': formularios[:10],
        'tablas': tablas[:40],
        'secciones': secciones,
        'funciones': sorted(set(re.findall(r'function\s+([A-Za-z_]\w*)', codigo)))[:40],
        'rutas_en_scripts': sorted(set(re.findall(r'/[A-Za-z0-9_/]+\.(?:do|jsp)', codigo)))[:40],
        'ajax': {'jquery': codigo.count('$.ajax'), 'xhr': codigo.count('XMLHttpRequest'), 'fetch': codigo.count('fetch(')},
        'contiene_identificadores_personales': False,
    }


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv:
        print(__doc__)
        return 2
    if argv[0] == '--ventana' and len(argv) == 2:
        # Ventana guardada que el lector no reconoció (p. ej. historico/historico-*.html del lote): solo estructura.
        print(json.dumps(estructura_ventana(Path(argv[1]).read_bytes()), ensure_ascii=False, indent=1))
        return 0
    salida = [extraer(argv[0], ['Doc.', 'Tipo Escrito', 'Solicitante', 'Estado'], 'escritos')]
    if len(argv) > 1:
        salida.append(extraer(argv[1], ['Nombre Proyecto', 'Fecha egreso', 'Causal egreso'], 'historico_proteccion'))
    print(json.dumps(salida, ensure_ascii=False, indent=1))
    return 0


if __name__ == '__main__':
    sys.exit(main())
