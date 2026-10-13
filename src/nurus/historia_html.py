"""Contrato de lectura de la ventana «Información de la Causa» (pestaña Historia), sin red ni escritura.

Estructura observada en H0: `contracts/csmp_capture_v1/HISTORIA_VENTANA.md` §4. Lee solo lo visible de la tabla;
no sigue enlaces de documentos ni ejecuta scripts. Ningún desvío se convierte en «sin trámites».
"""

from datetime import date, datetime
from hashlib import sha256
import re

from lxml import html

from nurus.bitacora_html import decode_snapshot, normalize

HEADERS = ('Folio', 'Doc.', 'Sel', 'Anexo', 'Etapa', 'Trámite', 'Desc. Trámite', 'Referencia', 'Fecha Tram')
CABECERA = (
    'RIT',
    'RUC',
    'Caratulado',
    'F. Ingreso',
    'Proc.',
    'Forma Inicio',
    'Est. Adm.',
    'Etapa',
    'Estado Proc.',
    'Tribunal',
)
# Códigos de error: el mensaje del ValueError es el código.
TABLA_NO_RECONOCIDA = 'HISTORIA_TABLA_NO_RECONOCIDA'
ENCABEZADO_DISTINTO = 'HISTORIA_ENCABEZADO_DISTINTO'
SESION = 'HISTORIA_SESION'
INCOMPLETA = 'HISTORIA_RESPUESTA_INCOMPLETA'
# Advertencias: la lectura sigue siendo válida.
FECHA_ILEGIBLE = 'HISTORIA_FECHA_ILEGIBLE'
FILA_INCOMPLETA = 'HISTORIA_FILA_INCOMPLETA'
ENCABEZADO_REPETIDO = 'HISTORIA_ENCABEZADO_REPETIDO'
POSIBLE_PAGINACION = 'HISTORIA_POSIBLE_PAGINACION'
MENSAJES = {
    TABLA_NO_RECONOCIDA: 'La respuesta no contiene la tabla de historia; puede ser otra página o un cambio de SITFA.',
    ENCABEZADO_DISTINTO: 'Cambió la estructura de la tabla de historia; no se leyó para no perder trámites.',
    SESION: 'La respuesta parece la página de ingreso de SITFA; revisa la sesión.',
    INCOMPLETA: 'La respuesta de historia llegó cortada (sin cierre del documento); se reintenta, no se lee como vacía.',
}
_CANON_HEADERS = tuple(normalize(h) for h in HEADERS)
_CANON_CABECERA = {normalize(e): e for e in CABECERA}


def _texto(nodo):
    partes = nodo.xpath('.//text()[not(ancestor::script or ancestor::style or ancestor::noscript)]')
    return ' '.join(' '.join(partes).split())


def _encabezado(tabla):
    filas = tabla.xpath('./thead/tr | ./tr | ./tbody/tr')
    if not filas:
        return None, []
    primera = filas[0]
    return tuple(normalize(_texto(c)) for c in primera.xpath('./th | ./td')), filas[1:]


def _fecha(texto):
    try:
        return datetime.strptime(texto, '%d/%m/%Y').date().isoformat()
    except ValueError:
        return ''


def _cabecera(documento):
    datos = {}
    for etiqueta in documento.xpath('//label'):
        clave = normalize(_texto(etiqueta).rstrip(': '))
        if clave in _CANON_CABECERA and _CANON_CABECERA[clave] not in datos:
            datos[_CANON_CABECERA[clave]] = ' '.join((etiqueta.tail or '').split())
    reservada = any(
        'reservada' in normalize(t) for t in documento.xpath('//*[contains(@class,"panel-heading")]//text()')
    )
    return datos, reservada


def _hay_paginacion(documento):
    """Indicios visibles de paginación fuera de scripts: clases típicas, campos de página o enlaces Siguiente."""
    clases = 'pagination|paginacion|paginador|pager'
    if documento.xpath(
        '//*[not(ancestor::script)][@class]['
        + ' or '.join(f'contains(concat(" ",@class," ")," {c} ")' for c in clases.split('|'))
        + ']'
    ):
        return True
    if documento.xpath(
        '//input[@name and (contains(@name,"pagina") or contains(@name,"nroPag") or contains(@name,"numPag"))]'
    ):
        return True
    return bool(documento.xpath('//a[translate(normalize-space(.),"SIGUENT","siguent")="siguiente"]'))


def read_historia(data, *, source='', captured_at=''):
    """Devuelve encabezado, filas visibles y advertencias. Lanza ValueError(código) si no es una historia legible."""
    crudo = data.encode('utf-8') if isinstance(data, str) else data
    texto = decode_snapshot(data)
    if not texto.strip():
        raise ValueError(TABLA_NO_RECONOCIDA)
    # Igual que el lector de bitácoras: una respuesta cortada podría parecer vacía o con menos filas.
    if not re.search(r'</html\s*>', texto, re.I):
        raise ValueError(INCOMPLETA)
    documento = html.fromstring(texto)
    if documento.xpath('//input[translate(@type,"PASSWORD","password")="password"]'):
        raise ValueError(SESION)
    candidatas = []
    parecidas = 0
    for tabla in documento.xpath('//table'):
        encabezado, filas = _encabezado(tabla)
        if encabezado == _CANON_HEADERS:
            candidatas.append(filas)
        elif encabezado and encabezado[:1] == _CANON_HEADERS[:1]:
            parecidas += 1
    if len(candidatas) != 1:
        raise ValueError(ENCABEZADO_DISTINTO if parecidas and not candidatas else TABLA_NO_RECONOCIDA)
    filas = candidatas[0]
    resultado = []
    advertencias = []
    for posicion, fila in enumerate(filas, 1):
        celdas = fila.xpath('./td | ./th')
        if tuple(normalize(_texto(c)) for c in celdas) == _CANON_HEADERS:
            advertencias.append({'codigo': ENCABEZADO_REPETIDO, 'fila': posicion, 'folio': ''})
            continue
        if len(celdas) == 4 and posicion == len(filas):
            # SITFA entrega la última fila solo con Folio, Doc., Sel y Anexo (H0): se informa y no se exporta.
            advertencias.append({'codigo': FILA_INCOMPLETA, 'fila': posicion, 'folio': _texto(celdas[0])})
            continue
        if len(celdas) != len(HEADERS):
            raise ValueError(ENCABEZADO_DISTINTO)
        valores = [_texto(c) for c in celdas]
        fecha_iso = _fecha(valores[8])
        if not fecha_iso:
            advertencias.append({'codigo': FECHA_ILEGIBLE, 'fila': posicion, 'folio': valores[0]})
        resultado.append(
            {
                'fila': posicion,
                'folio': valores[0],
                'etapa': valores[4],
                'tramite': valores[5],
                'descripcion': valores[6],
                'referencia': valores[7],
                'fecha_texto': valores[8],
                'fecha_iso': fecha_iso,
                'anexo': bool(celdas[3].xpath('.//img')),
            }
        )
    if _hay_paginacion(documento):
        # La única muestra (H0) no estaba paginada; si SITFA paginara, la primera página no es la historia completa.
        advertencias.append({'codigo': POSIBLE_PAGINACION, 'fila': 0, 'folio': ''})
    encabezado, reservada = _cabecera(documento)
    return {
        'encabezado': encabezado,
        'reservada': reservada,
        'filas': resultado,
        'vacio_explicito': not filas,
        'advertencias': advertencias,
        'sha256': sha256(crudo).hexdigest(),
        'source': source,
        'captured_at': captured_at,
    }


def separar_ventana(filas, hasta):
    """(dentro, anteriores, posteriores, sin_fecha) según la ventana de 4 meses que termina en `hasta`.

    Usa la misma función que la cronología (WP-50). Las filas sin fecha legible se devuelven aparte para que
    el producto las muestre con su advertencia; nunca se descartan en silencio.
    """
    from nurus.capture.temporal import earliest

    if isinstance(hasta, datetime):
        hasta = hasta.date()
    if not isinstance(hasta, date):
        raise TypeError('hasta debe ser una fecha')
    desde = earliest(hasta)
    dentro, anteriores, posteriores, sin_fecha = [], [], [], []
    for fila in filas:
        if not fila['fecha_iso']:
            sin_fecha.append(fila)
            continue
        dia = date.fromisoformat(fila['fecha_iso'])
        (anteriores if dia < desde else posteriores if dia > hasta else dentro).append(fila)
    return dentro, anteriores, posteriores, sin_fecha


def mensaje(codigo):
    return MENSAJES.get(codigo, codigo)


__all__ = ('HEADERS', 'CABECERA', 'read_historia', 'separar_ventana', 'mensaje')
