"""Contrato de lectura del «Calendario de informes proyectados» RUS (IrPopUpInformesAccion.do, tipo_popUp=8).

Sin red ni escritura. Estructura observada el 19-oct-2026 (Explorador V1.3 y capturas del usuario): formulario
InformesPpalForm con identidad oculta COD_Tribunal, CRR_IdCausa e ID_Ingreso, y una tabla
«Fecha Proyección · Fecha Informe · Estado». Estados conocidos: PENDIENTE y ENVIADO. Cualquier desvío es error, nunca
«sin proyecciones».
"""

from datetime import datetime
from hashlib import sha256
import re

from lxml import html

from nurus.bitacora_html import decode_snapshot, normalize

ENCABEZADO = ('fecha proyeccion', 'fecha informe', 'estado')
ESTADOS = ('PENDIENTE', 'ENVIADO')
# Argumentos de ShowInformesProgramados(idCausa, idParte, codTribunal, codCentro, rit, fecIngEfectivo, glsCentro,
# nomMenor, intervencion, remision, id_ingreso, FLG_Antiguo).
CAUSA, TRIBUNAL, INGRESO = 0, 2, 10


def _input(form, name):
    found = form.xpath('.//input[@name=$name]', name=name)
    if len(found) != 1:
        raise ValueError('Campo ausente o repetido en el calendario: ' + name)
    return found[0].get('value', '')


def _fecha(texto, *, opcional=False):
    texto = texto.strip()
    if opcional and texto in ('', '-', '--', '---'):
        return ''
    try:
        return datetime.strptime(texto, '%d/%m/%Y').date().isoformat()
    except ValueError:
        raise ValueError('El calendario contiene una fecha ilegible.') from None


def _texto(nodo):
    return ' '.join(' '.join(nodo.xpath('.//text()[not(ancestor::script or ancestor::style)]')).split())


def _plazo(documento, etiqueta):
    for nodo in documento.xpath('//td | //th | //span | //label | //b | //strong'):
        texto = _texto(nodo)
        if normalize(texto).startswith(etiqueta):
            numero = re.search(r'(\d+)\s*mes', normalize(texto))
            if numero:
                return int(numero.group(1))
    return None


def read_calendar(data, args, *, source='', captured_at=''):
    """Proyecciones del calendario de un ingreso, verificando que la respuesta sea del ingreso pedido."""
    raw = data.encode('utf-8') if isinstance(data, str) else data
    text = decode_snapshot(data)
    if not re.search(r'</html\s*>', text, re.I):
        raise ValueError('La respuesta HTML del calendario está incompleta.')
    documento = html.fromstring(text)
    if documento.xpath('//input[@type="password"]'):
        raise ValueError('La respuesta parece la página de ingreso de SITFA; revisa la sesión.')
    forms = documento.xpath('//form[@name="InformesPpalForm"]')
    if len(forms) != 1:
        raise ValueError('No se encontró el formulario del calendario; comprueba la sesión.')
    form = forms[0]
    identidad = {
        'tribunal_codigo': _input(form, 'COD_Tribunal'),
        'causa_id': _input(form, 'CRR_IdCausa'),
        'ingreso_id': _input(form, 'ID_Ingreso'),
    }
    if (identidad['tribunal_codigo'], identidad['causa_id'], identidad['ingreso_id']) != (
        args[TRIBUNAL],
        args[CAUSA],
        args[INGRESO],
    ):
        raise ValueError('La respuesta no corresponde al ingreso solicitado.')
    tablas = []
    for tabla in documento.xpath('//table'):
        filas = tabla.xpath('./tr | ./tbody/tr | ./thead/tr')
        if filas and tuple(normalize(_texto(c)) for c in filas[0].xpath('./td | ./th')) == ENCABEZADO:
            tablas.append(filas)
    if len(tablas) != 1:
        raise ValueError('Cambió la estructura del calendario; no se leyó para no perder proyecciones.')
    proyecciones = []
    for numero, fila in enumerate(tablas[0][1:], 1):
        celdas = [_texto(c) for c in fila.xpath('./td | ./th')]
        if not any(celdas):
            continue
        if len(celdas) != 3:
            raise ValueError('Fila de calendario incompleta.')
        estado = celdas[2].strip().upper()
        if estado not in ESTADOS:
            raise ValueError('Estado de calendario no reconocido.')
        informe = _fecha(celdas[1], opcional=True)
        if (estado == 'ENVIADO') != bool(informe):
            raise ValueError('Estado y fecha de informe del calendario no concuerdan.')
        proyecciones.append(
            {'fila': numero, 'fecha_proyeccion': _fecha(celdas[0]), 'fecha_informe': informe, 'estado': estado}
        )
    return {
        **identidad,
        'proyecciones': proyecciones,
        'plazo_remision_meses': _plazo(documento, 'plazo remision'),
        'plazo_intervencion_meses': _plazo(documento, 'plazo intervencion'),
        'source': source,
        'captured_at': captured_at,
        'html_sha256': sha256(raw).hexdigest(),
    }
