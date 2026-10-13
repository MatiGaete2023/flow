"""Estructura anónima de la ventana «Información de la Causa» (pestaña Historia) y de cómo se abre.

Uso (en el PC autorizado, desde la raíz del repositorio):
    python descargador/extraer_estructura_historia.py -o estructura_historia.json ventana.html [listado.html listado_files captura.har]

Acepta copias guardadas con Chrome (Ctrl+S) de la ventana de historia y del listado de Seguimiento, archivos .js
guardados junto a ellas y un HAR exportado desde DevTools. Solo `estructura_historia.json` puede compartirse; los
originales no salen del equipo. Antes de compartirlo, ábrelo y comprueba que no aparezcan nombres, RIT, RUC, folios,
fechas, glosas ni tokens.

Entrega etiquetas conocidas, rutas `.do/.jsp`, nombres de campos y funciones, conteos y la *forma* de cada celda
(fecha, entero, texto de cierto largo), nunca valores. Si detecta que un texto de entrada aparece en la salida,
no escribe nada y termina con código 3.
"""

import argparse
from collections import Counter
import json
from pathlib import Path
import re
import sys
import unicodedata
from urllib.parse import parse_qsl, urlsplit

VERSION = 1
MAX_BYTES = 40 * 1024 * 1024
MAX_HAR = 400 * 1024 * 1024  # un HAR «con contenido» de DevTools incluye imágenes y scripts
MAX_TABLAS = 200
MAX_COLUMNAS = 30
MAX_ESQUELETO = 4000
IDENT = re.compile(r'^[A-Za-z_$][\w$]{0,60}$')
RUTA = re.compile(r'^/?(?:[A-Za-z0-9_\-]+/)*[A-Za-z0-9_\-]+\.(?:do|jsp)$')
INTERES = re.compile(r'(?i)histor|detalle|popup|causa|show|abrir|ventana|tramite|consulta')
LLAMADA = re.compile(r'([A-Za-z_$][\w$]*)\s*\(([^()]*)\)')

ETIQUETAS = (
    'Información de la Causa', 'Causa Reservada', 'RIT', 'RUC', 'Caratulado', 'F. Ingreso', 'Fecha Ingreso',
    'Proc.', 'Forma Inicio', 'Est. Adm.', 'Etapa', 'Estado Proc.', 'Tribunal', 'Texto Requerimiento',
    'Litigantes', 'Materias', 'Plazos', 'Remisión', 'Historia', 'Notificación', 'Diligencias', 'Escritos',
    'Folio', 'Doc.', 'Sel', 'Anexo', 'Trámite', 'Desc. Trámite', 'Referencia', 'Fecha Tram', 'Fecha Trámite',
    'Fecha', 'Descripción', 'Estado', 'Tipo', 'Página', 'Siguiente', 'Anterior', 'Primera', 'Última', 'Cerrar',
    'Imprimir', 'Volver', 'Nombre', 'Nombre Menor', 'RUT', 'RUT (->RCEI)', 'Derivación', 'Programa',
    'Nombre Centro', 'Obs.', 'Observaciones', 'Causa', 'Historia Causa', 'Ver', 'Buscar',
)
VALORES_CONOCIDOS = (
    'Espera', 'Cumplimiento', 'Informes', 'Egresados', 'Resolución', 'Escrito', 'Actuación', 'Audiencia',
    'Notificación', 'Diligencia', 'Certificación', 'Oficio', 'Acta', 'Sin referencia', 'Sin archivar',
    'Concluido', 'Tramitación', 'Requerimiento', 'Demanda',
)


def normalizar(texto):
    texto = unicodedata.normalize('NFKD', str(texto or '')).encode('ascii', 'ignore').decode()
    return re.sub(r'\s+', ' ', texto).strip().strip(':.').strip().lower()


CANON_ETIQUETAS = {normalizar(e): e for e in ETIQUETAS}
CANON_VALORES = {normalizar(v): v for v in VALORES_CONOCIDOS}
PERMITIDOS = set(CANON_ETIQUETAS) | set(CANON_VALORES)


def etiqueta(texto):
    clave = normalizar(texto)
    if clave in CANON_ETIQUETAS:
        return CANON_ETIQUETAS[clave]
    if ':' in str(texto):
        previa = normalizar(str(texto).split(':', 1)[0])
        if previa in CANON_ETIQUETAS:
            return CANON_ETIQUETAS[previa] + ': VALOR'
    return 'ETIQUETA_NO_RECONOCIDA'


def forma(texto):
    texto = ' '.join(str(texto or '').split())
    clave = normalizar(texto)
    if not texto:
        return 'VACIO'
    if re.fullmatch(r'\d{2}/\d{2}/\d{4}', texto):
        return 'FECHA_DD/MM/AAAA'
    if re.fullmatch(r'\d{2}/\d{2}/\d{4} \d{1,2}:\d{2}(:\d{2})?', texto):
        return 'FECHA_HORA'
    if re.fullmatch(r'\d+', texto):
        return 'ENTERO_1-3' if len(texto) <= 3 else 'ENTERO_4-6' if len(texto) <= 6 else 'ENTERO_7+'
    if re.fullmatch(r'[A-Z]{1,3}-\d+-\d{4}', texto):
        return 'FORMA_RIT'
    if re.fullmatch(r'\d{2}-\s?\d-\d+-\d', texto):
        return 'FORMA_RUC'
    if clave in CANON_VALORES:
        return 'VALOR_CONOCIDO:' + CANON_VALORES[clave]
    if clave in CANON_ETIQUETAS:
        return 'ETIQUETA:' + CANON_ETIQUETAS[clave]
    largo = len(texto)
    tramo = '1-20' if largo <= 20 else '21-60' if largo <= 60 else '61-200' if largo <= 200 else '201+'
    return 'TEXTO_' + tramo + ('_CON_DIGITOS' if re.search(r'\d', texto) else '')


def forma_parametro(valor):
    largo = len(valor)
    tramo = '0' if not largo else '1-3' if largo <= 3 else '4-12' if largo <= 12 else '13-64' if largo <= 64 else '65+'
    if not valor:
        clase = 'VACIO'
    elif valor.isdigit():
        clase = 'DIGITOS'
    elif re.fullmatch(r'[A-Za-z0-9]+', valor):
        clase = 'ALFANUM'
    elif re.fullmatch(r'[A-Za-z0-9+/=_\-%]+', valor):
        clase = 'BASE64_O_CODIFICADO'
    else:
        clase = 'OTRO'
    return clase + '/' + tramo


def nombre_campo(nombre):
    return nombre if isinstance(nombre, str) and IDENT.fullmatch(nombre) else 'CAMPO_NO_RECONOCIDO'


def resumir_url(valor, sensibles):
    valor = str(valor or '').strip()
    if valor.lower().startswith('javascript:'):
        return {'tipo': 'JAVASCRIPT', 'llamadas': llamadas(valor[11:], sensibles)}
    try:
        url = urlsplit(valor)
    except ValueError:
        return {'tipo': 'URL_NO_RECONOCIDA'}
    if url.scheme and url.scheme not in ('http', 'https'):
        return {'tipo': 'ESQUEMA_' + nombre_campo(url.scheme)}
    host = url.hostname or ''
    origen = 'RELATIVA' if not host else 'SITFA' if host == 'familia.pjud.cl' else 'OTRO_HOST'
    ruta = url.path
    if RUTA.fullmatch(ruta):
        ruta_segura = ruta
    elif '.' in ruta.rsplit('/', 1)[-1]:
        extension = ruta.rsplit('.', 1)[-1].lower()
        ruta_segura = 'ARCHIVO.' + (extension if re.fullmatch(r'[a-z0-9]{1,5}', extension) else 'otro')
    else:
        ruta_segura = 'RUTA_NO_RECONOCIDA' if ruta not in ('', '/') else ruta
    parametros = []
    for nombre, dato in parse_qsl(url.query, keep_blank_values=True):
        sensibles.add(dato)
        parametros.append({'nombre': nombre_campo(nombre), 'forma': forma_parametro(dato)})
    return {'tipo': 'URL', 'origen': origen, 'ruta': ruta_segura, 'parametros': parametros}


def llamadas(codigo, sensibles):
    salida = Counter()
    for nombre, argumentos in LLAMADA.findall(str(codigo or '')):
        partes = [p.strip() for p in argumentos.split(',')] if argumentos.strip() else []
        tipos = []
        for parte in partes:
            if re.fullmatch(r'\d+', parte):
                tipos.append('num')
            elif parte[:1] in ('"', "'"):
                tipos.append('str')
                sensibles.add(parte.strip('\'"'))
            else:
                tipos.append('expr')
        salida[f'{nombre_campo(nombre)}/{len(partes)}:' + ','.join(tipos)] += 1
    return dict(sorted(salida.items()))


def texto_visible(nodo):
    partes = nodo.xpath('.//text()[not(ancestor::script or ancestor::style or ancestor::noscript)]')
    return ' '.join(' '.join(partes).split())


def filas_directas(tabla):
    return tabla.xpath('./tr | ./tbody/tr | ./thead/tr | ./tfoot/tr')


def resumen_celda(celda, sensibles):
    rasgos = Counter()
    if celda.xpath('.//img'):
        rasgos['imagen'] += 1
    if celda.xpath('.//a'):
        rasgos['enlace'] += 1
    if celda.xpath('.//input'):
        rasgos['control'] += 1
    acciones = Counter()
    for nodo in celda.xpath('.//*[@onclick or @href]'):
        for atributo in ('onclick', 'href'):
            valor = nodo.get(atributo) or ''
            if not valor:
                continue
            if atributo == 'onclick' or valor.lower().startswith('javascript:'):
                acciones.update(llamadas(valor.split(':', 1)[-1] if atributo == 'href' else valor, sensibles))
            else:
                url = resumir_url(valor, sensibles)
                acciones['href ' + url.get('ruta', url['tipo'])] += 1
    return rasgos, acciones


def resumen_tabla(tabla, sensibles):
    filas = filas_directas(tabla)
    encabezado = None
    perfiles = []
    irregulares = 0
    datos = 0
    for fila in filas:
        celdas = fila.xpath('./th | ./td')
        textos = [texto_visible(c) for c in celdas]
        sensibles.update(textos)
        etiquetas = [etiqueta(t) for t in textos]
        reconocidas = sum(e != 'ETIQUETA_NO_RECONOCIDA' for e in etiquetas)
        if encabezado is None and celdas and (
            fila.xpath('./th')
            or all(e != 'ETIQUETA_NO_RECONOCIDA' or not t for e, t in zip(etiquetas, textos))
            or (len(celdas) >= 3 and reconocidas >= 0.6 * len(celdas))
        ):
            encabezado = etiquetas[:MAX_COLUMNAS]
            perfiles = [{'formas': Counter(), 'rasgos': Counter(), 'acciones': Counter()} for _ in encabezado]
            continue
        if encabezado is None:
            continue
        datos += 1
        if len(celdas) != len(encabezado):
            irregulares += 1
        for indice, celda in enumerate(celdas[: len(perfiles)]):
            perfil = perfiles[indice]
            perfil['formas'][forma(textos[indice])] += 1
            rasgos, acciones = resumen_celda(celda, sensibles)
            perfil['rasgos'].update(rasgos)
            perfil['acciones'].update(acciones)
    return {
        'id': nombre_campo(tabla.get('id')) if tabla.get('id') else '',
        'clase': ' '.join(nombre_campo(c) for c in (tabla.get('class') or '').split()),
        'anidada': bool(tabla.xpath('ancestor::table')),
        'filas': len(filas),
        'encabezado': encabezado or [],
        'filas_de_datos': datos,
        'filas_con_celdas_distintas_al_encabezado': irregulares,
        'columnas': [
            {
                'etiqueta': encabezado[i],
                'formas': dict(sorted(p['formas'].items())),
                'rasgos': dict(sorted(p['rasgos'].items())),
                'acciones': dict(sorted(p['acciones'].items())),
            }
            for i, p in enumerate(perfiles)
        ]
        if encabezado
        else [],
    }


def esqueleto(fuente, inicio):
    """Cuerpo de una función con literales y números largos enmascarados; rutas .do/.jsp y nombres de campo se conservan."""
    abre = fuente.find('{', inicio)
    if abre < 0:
        return ''
    nivel, i, comilla = 0, abre, None
    while i < len(fuente):
        c = fuente[i]
        if comilla:
            if c == '\\':
                i += 1
            elif c == comilla:
                comilla = None
        elif c in ('"', "'", '`'):
            comilla = c
        elif c == '{':
            nivel += 1
        elif c == '}':
            nivel -= 1
            if nivel == 0:
                break
        i += 1
    cuerpo = fuente[inicio : i + 1]
    cuerpo = re.sub(r'/\*.*?\*/', ' ', cuerpo, flags=re.S)
    cuerpo = re.sub(r'(?m)//[^\n\'"]*$', ' ', cuerpo)

    def literal(match):
        valor = match.group(0)[1:-1]
        ruta = valor.split('?', 1)[0]
        if RUTA.fullmatch(ruta.split('://', 1)[-1].split('/', 1)[-1] if '://' in ruta else ruta):
            consulta = valor.split('?', 1)[1] if '?' in valor else ''
            campos = [p.split('=', 1)[0] for p in consulta.split('&') if p]
            return '"' + ruta + ('?' + '&'.join(nombre_campo(c) + '=VALOR' for c in campos) if consulta else '') + '"'
        if re.fullmatch(r'&?[A-Za-z_][\w]{0,40}=', valor):
            return '"' + valor + '"'
        if re.fullmatch(r'[A-Za-z_][\w]{0,40}', valor) and normalizar(valor) not in PERMITIDOS:
            return '"IDENT"'
        return '"TEXTO"'

    cuerpo = re.sub(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|`(?:\\.|[^`\\])*`', literal, cuerpo)
    cuerpo = re.sub(r'\b\d{3,}\b', 'NUM', cuerpo)
    cuerpo = re.sub(r'[ \t]+', ' ', cuerpo)
    return cuerpo[:MAX_ESQUELETO]


def resumen_script(fuente, sensibles):
    for match in re.finditer(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'', fuente):
        sensibles.add(match.group(0)[1:-1])
    definidas = Counter(nombre_campo(n) for n in re.findall(r'function\s+([A-Za-z_$][\w$]*)\s*\(', fuente))
    esqueletos = {}
    for match in re.finditer(r'function\s+([A-Za-z_$][\w$]*)\s*\(', fuente):
        nombre = match.group(1)
        if INTERES.search(nombre) and nombre not in esqueletos and len(esqueletos) < 40:
            esqueletos[nombre_campo(nombre)] = esqueleto(fuente, match.start())
    return {
        'funciones_definidas': dict(sorted(definidas.items())),
        'window_open': len(re.findall(r'window\.open\s*\(', fuente)),
        'location_asignaciones': len(re.findall(r'location(\.href)?\s*=', fuente)),
        'submit': len(re.findall(r'\.submit\s*\(', fuente)),
        'xhr_o_fetch': len(re.findall(r'XMLHttpRequest|\bfetch\s*\(|\$\.ajax|\$\.post|\$\.get', fuente)),
        'esqueletos': esqueletos,
    }


def decodificar(datos):
    if datos.startswith(b'\xef\xbb\xbf'):
        return datos.decode('utf-8-sig')
    try:
        return datos.decode('utf-8')
    except UnicodeDecodeError:
        return datos.decode('cp1252', errors='replace')


def resumen_html(texto, sensibles):
    from lxml import html

    documento = html.fromstring(texto) if texto.strip() else None
    if documento is None:
        return {'vacio': True}
    sensibles.update(documento.xpath('//text()'))
    for nodo in documento.xpath('//*[@value or @title or @alt]'):
        sensibles.update(nodo.get(a) for a in ('value', 'title', 'alt') if nodo.get(a))
    tablas = [resumen_tabla(t, sensibles) for t in documento.xpath('//table')[:MAX_TABLAS]]
    formularios = []
    for formulario in documento.xpath('//form'):
        campos = []
        for control in formulario.xpath('.//input | .//select | .//textarea'):
            valor = control.get('value') or ''
            sensibles.add(valor)
            campos.append(
                {
                    'nombre': nombre_campo(control.get('name') or control.get('id') or ''),
                    'tipo': nombre_campo(control.get('type') or control.tag),
                    'forma_valor': forma_parametro(valor),
                }
            )
        formularios.append(
            {
                'nombre': nombre_campo(formulario.get('name') or ''),
                'metodo': (formulario.get('method') or 'get').lower()[:6],
                'accion': resumir_url(formulario.get('action') or '', sensibles),
                'campos': campos,
            }
        )
    acciones = Counter()
    for nodo in documento.xpath('//*[@onclick]'):
        acciones.update(llamadas(nodo.get('onclick'), sensibles))
    for nodo in documento.xpath('//a[starts-with(translate(@href,"JAVSCRIPT","javscript"),"javascript:")]'):
        acciones.update(llamadas(nodo.get('href').split(':', 1)[1], sensibles))
    marcos = [resumir_url(n.get('src') or '', sensibles) for n in documento.xpath('//iframe | //frame')]
    scripts_externos = [resumir_url(n.get('src'), sensibles) for n in documento.xpath('//script[@src]')]
    codigo = '\n'.join(n.text or '' for n in documento.xpath('//script[not(@src)]'))
    textos = [normalizar(t) for t in documento.xpath('//text()')]
    paginacion = Counter(
        CANON_ETIQUETAS[t] for t in textos if t in ('siguiente', 'anterior', 'primera', 'ultima', 'pagina')
    )
    pestanas = Counter(
        CANON_ETIQUETAS[t] for t in textos if t in ('historia', 'notificacion', 'diligencias', 'escritos')
    )
    declarados = list(documento.xpath('//meta[@charset]/@charset')) + [
        c.lower().split('charset=')[-1] for c in documento.xpath('//meta[@content]/@content') if 'charset=' in c.lower()
    ]
    charset = next((c.strip().lower() for c in declarados if re.fullmatch(r'[a-z0-9_\-]{1,20}', c.strip().lower())), '')
    return {
        'titulo': etiqueta(texto_visible(documento.xpath('//title')[0])) if documento.xpath('//title') else '',
        'charset_declarado': charset,
        'etiquetas_reconocidas': dict(
            sorted(Counter(etiqueta(t) for t in documento.xpath('//text()') if etiqueta(t) != 'ETIQUETA_NO_RECONOCIDA').items())
        ),
        'pestanas_visibles': dict(sorted(pestanas.items())),
        'indicios_paginacion': dict(sorted(paginacion.items())),
        'tablas': tablas,
        'formularios': formularios,
        'acciones_onclick': dict(sorted(acciones.items())),
        'marcos': marcos,
        'scripts_externos': scripts_externos,
        'script_en_linea': resumen_script(codigo, sensibles),
    }


def resumen_har(datos, sensibles):
    peticiones = []
    for entrada in datos.get('log', {}).get('entries', []):
        solicitud, respuesta = entrada.get('request', {}), entrada.get('response', {})
        url = resumir_url(solicitud.get('url', ''), sensibles)
        post = solicitud.get('postData') or {}
        campos_post = []
        if post.get('params'):
            pares = [(p.get('name', ''), p.get('value', '')) for p in post['params']]
        elif post.get('text') and 'x-www-form-urlencoded' in (post.get('mimeType') or ''):
            pares = parse_qsl(post['text'], keep_blank_values=True)
        else:
            pares = []
            if post.get('text'):
                sensibles.add(post['text'])
        for nombre, valor in pares:
            sensibles.add(valor)
            campos_post.append({'nombre': nombre_campo(nombre), 'forma': forma_parametro(str(valor))})
        ubicacion = next(
            (h.get('value', '') for h in respuesta.get('headers', []) if str(h.get('name', '')).lower() == 'location'),
            '',
        )
        contenido = respuesta.get('content') or {}
        tipo = str(contenido.get('mimeType') or '').split(';')[0].strip().lower()
        item = {
            'metodo': solicitud.get('method') if solicitud.get('method') in ('GET', 'POST', 'HEAD') else 'OTRO',
            'url': url,
            'campos_post': campos_post,
            'estado_http': respuesta.get('status') if isinstance(respuesta.get('status'), int) else None,
            'redireccion': resumir_url(ubicacion, sensibles) if ubicacion else None,
            'tipo_contenido': tipo if re.fullmatch(r'[a-z]+/[a-z0-9.+\-]+', tipo) else 'NO_RECONOCIDO',
            'bytes': contenido.get('size') if isinstance(contenido.get('size'), int) else None,
        }
        cuerpo = contenido.get('text') or ''
        if isinstance(cuerpo, str) and cuerpo and contenido.get('encoding') != 'base64':
            if tipo == 'text/html':
                item['html'] = resumen_html(cuerpo, sensibles)
            elif tipo in ('application/javascript', 'text/javascript'):
                item['javascript'] = resumen_script(cuerpo, sensibles)
            else:
                sensibles.add(cuerpo)
        peticiones.append(item)
    return {'peticiones': peticiones}


def analizar(ruta, sensibles):
    ruta = Path(ruta)
    if ruta.stat().st_size > (MAX_HAR if ruta.suffix.lower() == '.har' else MAX_BYTES):
        raise ValueError('archivo demasiado grande')
    datos = ruta.read_bytes()
    sufijo = ruta.suffix.lower()
    if sufijo == '.har':
        return {'tipo': 'HAR', **resumen_har(json.loads(decodificar(datos)), sensibles)}
    if sufijo == '.js':
        return {'tipo': 'JS', **resumen_script(decodificar(datos), sensibles)}
    if sufijo in ('.html', '.htm'):
        return {'tipo': 'HTML', **resumen_html(decodificar(datos), sensibles)}
    raise ValueError('tipo de archivo no admitido (usa .html, .htm, .js o .har)')


def fugas(salida, sensibles):
    """Textos de entrada (4+ caracteres, no etiquetas ni valores conocidos) que reaparecen en la salida."""
    encontradas = []
    for valor in sensibles:
        valor = ' '.join(str(valor or '').split())
        if len(valor) < 4 or normalizar(valor) in PERMITIDOS:
            continue
        # Conteos y tamaños son números de la salida; rutas y nombres de campo se conservan a propósito.
        if re.fullmatch(r'\d{1,6}', valor) or RUTA.fullmatch(valor.split('?', 1)[0]) or re.fullmatch(r'&?[A-Za-z_]\w{0,40}=', valor):
            continue
        if valor in salida:
            encontradas.append(len(valor))
    return encontradas


def expandir(archivos):
    """Admite carpetas (p. ej. `pagina_files`) y comodines, porque la consola de Windows no los expande."""
    from glob import glob

    salida = []
    for archivo in archivos:
        ruta = Path(archivo)
        if ruta.is_dir():
            salida += sorted(p for p in ruta.iterdir() if p.is_file() and p.suffix.lower() in ('.html', '.htm', '.js', '.har'))
        elif any(c in archivo for c in '*?['):
            salida += sorted(Path(p) for p in glob(archivo) if Path(p).is_file())
        else:
            salida.append(ruta)
    return salida


def main(argv):
    parser = argparse.ArgumentParser(description='Estructura anónima de la historia de causa SITFA.')
    parser.add_argument('-o', '--salida', required=True)
    parser.add_argument('archivos', nargs='+')
    args = parser.parse_args(argv)
    destino = Path(args.salida)
    if destino.exists():
        print('El archivo de salida ya existe; elige otro nombre.')
        return 1
    sensibles = set()
    copias = []
    for numero, archivo in enumerate(expandir(args.archivos), 1):
        try:
            copias.append({'archivo': f'entrada_{numero}{archivo.suffix.lower()}', **analizar(archivo, sensibles)})
        except (OSError, ValueError) as exc:
            copias.append({'archivo': f'entrada_{numero}', 'error': type(exc).__name__ + ': ' + forma(str(exc))})
    resultado = {
        'tipo': 'ESTRUCTURA_HISTORIA_SIN_VALORES',
        'version': VERSION,
        'copias': copias,
        'advertencia': 'Sin valores de celdas, campos ocultos, cookies, encabezados ni tokens. Revisa antes de compartir.',
    }
    texto = json.dumps(resultado, ensure_ascii=False, indent=2)
    detectadas = fugas(texto, sensibles)
    if detectadas:
        print(f'Se detectaron {len(detectadas)} posibles valores de entrada en la salida; no se escribió nada.')
        return 3
    try:
        with destino.open('x', encoding='utf-8') as archivo:
            archivo.write(texto)
    except FileExistsError:
        print('El archivo de salida ya existe; elige otro nombre.')
        return 1
    print(f'Estructura guardada en {destino}. Revísala y comparte solo ese archivo; los originales quedan en este equipo.')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
