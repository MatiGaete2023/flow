"""Extracto ANÓNIMO de la pestaña Escritos y de la tabla Ingreso Histórico Protección, para decidir cómo asociar un escrito
con su informe (plan 18-nov-2026). Ejecutar localmente sobre el HTML real guardado; entregar solo la salida (nunca el HTML).

    python descargador/extraer_estructura_escritos.py ESCRITOS.html [HISTORIAL.html] > estructura_escritos.json
    python descargador/extraer_estructura_escritos.py --ventana historico/historico-XXXX.html > estructura_historico.json

El extracto contiene cantidades, tipos y literales de catálogos cerrados. Los valores desconocidos se resumen sin copiar su texto.
"""

import json
import re
import sys
from pathlib import Path
from urllib.parse import urlsplit

from lxml import html

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'src'))
from nurus.bitacora_html import decode_snapshot, normalize

_ROTULOS = (
    'Historial Menores', 'Despliegue de Pop Up', 'Ingreso Historico Protección', 'Ingreso Historico(LRPA)',
    'Ingreso Histórico', 'Protección', 'Escritos', 'Doc.', 'Tipo Escrito', 'Solicitante', 'Estado',
    'Nombre Proyecto', 'Programa', 'Fecha Ingreso', 'Fecha Egreso', 'Causal Egreso', 'Causal Ingreso',
    'Tribunal', 'RUC', 'RIT', 'RUT', 'RUN', 'Sexo', 'Nombre del niño', 'Nombre del menor',
    'Folio', 'Etapa', 'Trámite', 'Referencia', 'Código', 'Entidad', 'Proyecto', 'Quién Egresa',
)
_ROTULO_SEGURO = {normalize(r): r for r in _ROTULOS}
_FUNCIONES = {'ShowHistoriaRut', 'ShowHistorialFichas', 'verEscritos', 'verDoc'}
_RUTAS = {
    '/SITFAWEB/IrPopUpPublicoAccion.do', '/SITFAWEB/EscritosAccion.do',
    '/SITFAWEB/jsp/Tramitacion/PopUp/PopUpHistorial.jsp',
}
_ATRIBUTOS = {'id', 'class', 'name', 'value', 'href', 'onclick', 'src', 'type', 'title', 'style', 'target', 'alt'}
_PARAMETROS = {'id', 'clave', 'crr_causa', 'tipo_popUp', 'RUT_Litigante', 'RUT_DV'}


def _rotulo_seguro(texto):
    return _ROTULO_SEGURO.get(normalize(texto).strip(': '))


def _ruta_segura(valor):
    try:
        ruta = urlsplit(valor or '').path
    except ValueError:
        return 'otra'
    return ruta if not ruta or ruta in _RUTAS else 'otra'


def _catalogo(valores, permitidos):
    valores = set(valores)
    return sorted(valores & permitidos) + (['otro'] if valores - permitidos else [])


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
    salida = {'etiqueta': nodo.tag, 'atributos': _catalogo(nodo.attrib, _ATRIBUTOS)}
    onclick = nodo.get('onclick') or ''
    funcion = re.match(r'\s*([A-Za-z_][\w.]*)\s*\((.*)\)', onclick, re.DOTALL)
    if funcion:
        argumentos = [a.strip().strip('\'"') for a in funcion.group(2).split(',')] if funcion.group(2).strip() else []
        salida['onclick'] = {'funcion': funcion.group(1) if funcion.group(1) in _FUNCIONES else 'otra', 'argumentos': [tipo_valor(a) for a in argumentos]}
    href = nodo.get('href')
    if href is not None:
        esquema = href.split(':', 1)[0].lower() if re.match(r'^[A-Za-z]+:', href) else ('ancla' if href.startswith('#') else 'relativo')
        if esquema not in ('https', 'http', 'javascript', 'data', 'ancla', 'relativo'):
            esquema = 'otro'
        parametros = re.findall(r'[?&]([^=&#]+)=([^&#]*)', href)
        salida['href'] = {'esquema': esquema, 'parametros': [{'nombre': n if n in _PARAMETROS else 'otro', 'tipo': tipo_valor(v)} for n, v in parametros]}
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
    for _, originales, filas in tablas(documento, requeridos):
        item = {'columnas': [_rotulo_seguro(o) or '·' for o in originales], 'filas_de_datos': len(filas), 'filas': []}
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
    """Resume la forma de la pestaña Escritos, con tipos y literales de catálogos cerrados."""
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
        'version': 2,
        'controles': controles[:10],
        'funciones_escritos': _catalogo(funciones, _FUNCIONES),
        'rutas_en_scripts': _catalogo(rutas, _RUTAS),
        'nombres_de_parametros': _catalogo(parametros, _PARAMETROS),
        'ajax': {'jquery': codigo.count('$.ajax'), 'xhr': codigo.count('XMLHttpRequest'), 'fetch': codigo.count('fetch(')},
        'contiene_identificadores_personales': False,
    }


def estructura_ventana(data):
    """P3c: forma ANÓNIMA de una ventana SITFA (p. ej. «Historial Menores») cuyo lector no encontró la tabla esperada.
    Incluye dimensiones de tablas y rasgos estructurales de un catálogo cerrado."""
    documento = html.fromstring(decode_snapshot(data))
    titulo = _rotulo_seguro(' '.join(documento.xpath('//title//text()')))
    marcos = [
        {'etiqueta': el.tag, 'ruta': _ruta_segura(el.get('src'))}
        for el in documento.xpath('//iframe | //frame')
    ]
    formularios = [
        {
            'accion': _ruta_segura(f.get('action')),
            'metodo': (f.get('method') or '').lower() if (f.get('method') or '').lower() in ('get', 'post', '') else 'otro',
            'campos': _catalogo((c.get('name') for c in f.xpath('.//input | .//select') if c.get('name')), _PARAMETROS),
        }
        for f in documento.xpath('//form')
    ]
    tablas = []
    for tabla in documento.xpath('//table'):
        filas = tabla.xpath('./thead/tr | ./tr | ./tbody/tr')
        encabezados = []
        for fila in filas[:4]:
            celdas = fila.xpath('./th | ./td')
            textos = [_rotulo_seguro(c.text_content()) for c in celdas]
            if fila.xpath('./th') or (celdas and all(textos)):
                encabezados.append([t or '·' for t in textos])
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
        }
    )[:30]
    codigo = ' '.join(documento.xpath('//script/text()'))
    return {
        'tipo': 'ESTRUCTURA_VENTANA_SITFA',
        'version': 2,
        'bytes': len(data),
        'titulo': titulo,
        'marcos': marcos[:10],
        'formularios': formularios[:10],
        'tablas': tablas[:40],
        'secciones': secciones,
        'funciones': _catalogo(re.findall(r'function\s+([A-Za-z_]\w*)', codigo), _FUNCIONES),
        'rutas_en_scripts': _catalogo(re.findall(r'/[A-Za-z0-9_/]+\.(?:do|jsp)', codigo), _RUTAS),
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
