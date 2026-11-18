"""Lectores de la bandeja de Escritos de la causa y de la tabla Ingreso Histórico Protección de la persona.

Plan 18-nov-2026. Sin red, sin escritura y sin abrir documentos: solo lo visible de las tablas. Los valores salen de la
MISMA fila, ubicando cada columna por su encabezado (el orden de columnas no importa). Estados que distinguen:

- tabla con cabecera válida y cero filas → ``vacio_explicito`` (bandeja vacía al instante capturado);
- tabla con filas → ``filas``;
- pestaña/vista presente pero sin cuerpo legible → ValueError(ESCRITOS_NO_VERIFICADA / HISTORICO_NO_VERIFICADA);
- respuesta cortada o de sesión → ValueError con su código.
Ningún desvío se convierte en «sin escritos» ni en «sin otros programas».
"""

from hashlib import sha256
import re

from lxml import html

from nurus.bitacora_html import decode_snapshot, normalize

ESCRITOS_HEADERS = ('Doc.', 'Tipo Escrito', 'Solicitante', 'Estado')
HISTORICO_HEADERS = ('Programa', 'Fecha egreso', 'Causal egreso')  # «Programa» = «Nombre Proyecto» en SITFA
ESCRITOS_NO_VERIFICADA = 'ESCRITOS_NO_VERIFICADA'
ESCRITOS_AUSENTE = 'ESCRITOS_AUSENTE'
HISTORICO_NO_VERIFICADA = 'HISTORICO_NO_VERIFICADA'
INCOMPLETA = 'ESCRITOS_RESPUESTA_INCOMPLETA'
SESION = 'ESCRITOS_SESION'
FILA_CORTA = 'ESCRITOS_FILA_INCOMPLETA'
MENSAJES = {
    ESCRITOS_NO_VERIFICADA: 'La pestaña Escritos no trae una tabla legible; la fuente queda sin verificar, no vacía.',
    ESCRITOS_AUSENTE: 'La respuesta no incluye la pestaña Escritos (otra versión de SITFA o no cargada): fuente no solicitada.',
    HISTORICO_NO_VERIFICADA: 'La vista de Ingreso Histórico Protección no trae la tabla esperada; queda sin verificar.',
    INCOMPLETA: 'La respuesta llegó cortada (sin cierre del documento); se reintenta, no se lee como vacía.',
    SESION: 'La respuesta parece la página de ingreso de SITFA; revisa la sesión.',
}


def _texto(nodo):
    partes = nodo.xpath('.//text()[not(ancestor::script or ancestor::style or ancestor::noscript)]')
    return ' '.join(' '.join(partes).split())


def _documento(data):
    texto = decode_snapshot(data)
    if not texto.strip():
        raise ValueError(ESCRITOS_AUSENTE)
    if not re.search(r'</html\s*>', texto, re.I):
        raise ValueError(INCOMPLETA)
    documento = html.fromstring(texto)
    if documento.xpath('//input[translate(@type,"PASSWORD","password")="password"]'):
        raise ValueError(SESION)
    return documento, (data.encode('utf-8') if isinstance(data, str) else data)


def _tablas_con(documento, requeridos):
    """[(posiciones por encabezado normalizado, encabezado original, filas de datos)] de las tablas que traen todos."""
    pedidos = [normalize(h) for h in requeridos]
    salida = []
    for tabla in documento.xpath('//table'):
        filas = tabla.xpath('./thead/tr | ./tr | ./tbody/tr')
        if not filas:
            continue
        cabecera = [normalize(_texto(c)) for c in filas[0].xpath('./th | ./td')]
        if all(p in cabecera for p in pedidos):
            posiciones = {nombre: cabecera.index(nombre) for nombre in pedidos}
            salida.append((posiciones, cabecera, filas[1:]))
    return salida


def _leer_filas(posiciones, cabecera, filas, requeridos, codigo_fila):
    resultado, advertencias = [], []
    for numero, fila in enumerate(filas, 1):
        celdas = fila.xpath('./td | ./th')
        valores = [_texto(c) for c in celdas]
        if [normalize(v) for v in valores] == cabecera:
            continue  # encabezado repetido por SITFA
        if len(celdas) != len(cabecera):
            advertencias.append({'codigo': codigo_fila, 'fila': numero})
            continue
        item = {requerido: valores[posiciones[normalize(requerido)]] for requerido in requeridos}
        item['fila'] = numero
        item['literal'] = dict(zip(cabecera, valores))
        resultado.append(item)
    return resultado, advertencias


def read_escritos(data, *, source='', captured_at=''):
    """Bandeja de Escritos. Devuelve {'filas', 'vacio_explicito', 'advertencias', 'sha256', ...} con Doc., Tipo Escrito,
    Solicitante y Estado tal como se ven. Una celda «Doc.» solo con icono queda vacía: no se infiere la referencia."""
    documento, crudo = _documento(data)
    tablas = _tablas_con(documento, ESCRITOS_HEADERS)
    if len(tablas) != 1:
        # Sin ninguna mención de la columna «Tipo Escrito» la respuesta no trae la pestaña (otra versión de SITFA): la
        # fuente no se pidió. Con mención pero sin tabla legible, o con varias tablas, queda sin verificar.
        presente = 'tipo escrito' in normalize(_texto(documento))
        raise ValueError(ESCRITOS_NO_VERIFICADA if presente or tablas else ESCRITOS_AUSENTE)
    posiciones, cabecera, filas = tablas[0]
    resultado, advertencias = _leer_filas(posiciones, cabecera, filas, ESCRITOS_HEADERS, FILA_CORTA)
    return {
        'filas': resultado,
        'vacio_explicito': not filas,
        'advertencias': advertencias,
        'sha256': sha256(crudo).hexdigest(),
        'source': source,
        'captured_at': captured_at,
    }


# SITFA (Historial Menores) llama «Nombre Proyecto» a la columna del programa; «Programa» se acepta por compatibilidad.
HISTORICO_PROGRAMA = ('Nombre Proyecto', 'Programa')
_TITULO_PROTECCION = 'ingreso historico proteccion'


def _filas_directas(tabla):
    return tabla.xpath('./thead/tr | ./tr | ./tbody/tr')


def _tablas_historico(documento):
    """[(fila de encabezado, columnas por nombre, cabecera original, filas de datos)] de las tablas con las columnas del
    histórico de protección. La vista trae también «Ingreso Historico(LRPA)» con casi las mismas columnas: se descarta por
    su título (el más cercano antes del encabezado). El encabezado puede no ser la primera fila (banda de título)."""
    elementos = list(documento.iter())  # mantiene vivos los proxies: id() de lxml no es estable si se sueltan
    orden = {el: i for i, el in enumerate(elementos)}
    titulos = []
    for el in elementos:
        if not isinstance(el.tag, str) or el.tag in ('script', 'style', 'table', 'tr', 'tbody', 'thead'):
            continue
        texto = normalize(_texto(el))
        if texto == _TITULO_PROTECCION:
            titulos.append((orden[el], True))
        elif texto.startswith('ingreso historico') and 'lrpa' in texto and len(texto) < 40:
            titulos.append((orden[el], False))
    egreso = [normalize(h) for h in HISTORICO_HEADERS[1:]]
    programas = [normalize(h) for h in HISTORICO_PROGRAMA]
    salida = []
    for tabla in documento.xpath('//table'):
        filas = _filas_directas(tabla)
        for i, fila in enumerate(filas):
            originales = [_texto(c) for c in fila.xpath('./th | ./td')]
            cabecera = [normalize(h) for h in originales]
            if all(e in cabecera for e in egreso) and any(p in cabecera for p in programas):
                previos = [es_proteccion for idx, es_proteccion in titulos if idx < orden[fila]]
                if titulos and not (previos and previos[-1]):
                    break  # tabla de otro historial (LRPA)
                salida.append((fila, cabecera, originales, filas[i + 1 :]))
                break
    return salida


def read_historico(data, *, source='', captured_at='', persona_esperada=()):
    """Tabla visible de Ingreso Histórico Protección (programa = «Nombre Proyecto»). `persona_esperada`: textos (RUT,
    nombre) de la persona de la fila seleccionada; la vista es de esa persona solo si alguno aparece en el documento. Sin
    ellos, o sin coincidencia, la pertenencia queda INCIERTA (se puede continuar con las demás fuentes, pero no se afirma
    otro programa). Las filas devuelven siempre las claves «Programa», «Fecha egreso» y «Causal egreso»."""
    documento, crudo = _documento(data)
    tablas = _tablas_historico(documento)
    if len(tablas) != 1:
        raise ValueError(HISTORICO_NO_VERIFICADA)
    _, cabecera, originales, filas = tablas[0]
    columna_programa = next(cabecera.index(normalize(h)) for h in HISTORICO_PROGRAMA if normalize(h) in cabecera)
    posiciones = {
        'Programa': columna_programa,
        'Fecha egreso': cabecera.index(normalize('Fecha egreso')),
        'Causal egreso': cabecera.index(normalize('Causal egreso')),
    }
    resultado, advertencias = [], []
    for numero, fila in enumerate(filas, 1):
        celdas = fila.xpath('./td | ./th')
        valores = [_texto(c) for c in celdas]
        if [normalize(v) for v in valores] == cabecera:
            continue
        if not any(valores):
            continue  # fila de relleno al pie de la tabla
        if len(celdas) != len(cabecera):
            advertencias.append({'codigo': FILA_CORTA, 'fila': numero})
            continue
        item = {clave: valores[i] for clave, i in posiciones.items()}
        item['fila'] = numero
        item['literal'] = dict(zip(originales, valores))
        resultado.append(item)
    visible = ' ' + normalize(_texto(documento).replace('.', '')) + ' '
    pruebas = []
    for p in persona_esperada:
        limpio = normalize(str(p).replace('.', ''))
        # Un RUT identifica; un nombre solo si tiene al menos dos palabras (un nombre suelto aparece en cualquier texto).
        if limpio and (any(c.isdigit() for c in limpio) or len(limpio.split()) >= 2):
            pruebas.append(' ' + limpio + ' ')
    verificada = any(p in visible for p in pruebas)
    return {
        'filas': resultado,
        'vacio_explicito': not resultado and not advertencias,
        'ownership': 'VERIFICADA' if verificada else 'INCIERTA',
        'advertencias': advertencias,
        'sha256': sha256(crudo).hexdigest(),
        'source': source,
        'captured_at': captured_at,
    }


def mensaje(codigo):
    return MENSAJES.get(codigo, codigo)


__all__ = ('ESCRITOS_AUSENTE', 'ESCRITOS_NO_VERIFICADA', 'ESCRITOS_HEADERS', 'HISTORICO_HEADERS', 'read_escritos', 'read_historico', 'mensaje')
