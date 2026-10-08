"""Contrato de lectura de la ventana de bitácora RUS, sin red ni escritura."""
from datetime import datetime
from hashlib import sha256
import re
from urllib.parse import urlsplit
import unicodedata
from lxml import html

def normalize(value):
    text=unicodedata.normalize('NFKD',str(value or '')).encode('ascii','ignore').decode()
    return re.sub(r'\s+',' ',re.sub(r'[-().]+',' ',text.lower().strip()))

def visible_identity_warnings(snapshot, rit, persona):
    """Advertir diferencias de pantalla sin invalidar la identidad remota fuerte."""
    warnings = []
    if normalize(snapshot.get('rit')) != normalize(rit):
        warnings.append('Advertencia: RIT visible de bitácora distinto al listado; revisar el ingreso.')
    if normalize(snapshot.get('persona')) != normalize(persona):
        warnings.append('Advertencia: nombre visible de bitácora distinto al listado; revisar el ingreso.')
    return warnings

ORIGIN = 'https://familia.pjud.cl'
POPUP_PATH = '/SITFAWEB/IrPopUpInformesAccion.do'
HEADERS = ('Tipo Observación', 'Etapa', 'Fecha Centro', 'Usuario Centro',
           'Observación Centro', 'Fecha Tribunal', 'Usuario Tribunal',
           'Observación Tribunal', 'Acción')
IDENTITY_FIELDS = ('COD_Tribunal', 'CRR_IdCausa', 'ID_Ingreso', 'COD_Etapa', 'TIP_Consulta')
FULL_FIELDS = ('HOBS_Centro_', 'HOBS_Tribunal_', 'HCOD_Estado_', 'HFUN_Tribunal_', 'HFLG_Envio_')


def _date(text, *, optional=False):
    text = text.strip()
    if optional and text in ('', '---'): return ''
    for pattern in ('%d/%m/%Y %H:%M:%S', '%d/%m/%Y %H:%M', '%d/%m/%Y'):
        try: return datetime.strptime(text, pattern).isoformat()
        except ValueError: pass
    raise ValueError('La captura contiene una fecha de bitácora ilegible.')


def _one_input(form, name):
    found = form.xpath('.//input[@name=$name]', name=name)
    if len(found) != 1: raise ValueError('Campo ausente o repetido en bitácora: '+name)
    return found[0].get('value', '')


def read_snapshot(text, params, *, source='', captured_at=''):
    """Vincula cada fila con sus campos ocultos por ID, nunca por posición."""
    if params.get('tipo_popUp') != '12': raise ValueError('La captura no corresponde a una bitácora RUS.')
    # RUS añade scripts después de </html> en esta ventana.
    if not re.search(r'</html\s*>', text, re.I):
        raise ValueError('La respuesta HTML está incompleta.')
    document = html.fromstring(text)
    forms = document.xpath('//form[@name="InformesPpalForm"]')
    if len(forms) != 1: raise ValueError('No se encontró el formulario de bitácora; comprueba la sesión.')
    form = forms[0]
    action = urlsplit(form.get('action', ''))
    if (action.path != '/SITFAWEB/InformesDAction.do' or
            (action.netloc and f'{action.scheme}://{action.netloc}' != ORIGIN)):
        raise ValueError('El formulario de bitácora no tiene el destino esperado.')
    context = {key: _one_input(form, key) for key in IDENTITY_FIELDS}
    for key, value in context.items():
        if not value or value != params.get(key):
            raise ValueError('La respuesta no corresponde al ingreso solicitado: '+key)
    for key in IDENTITY_FIELDS[:4]:
        if not context[key].isdigit() or int(context[key]) <= 0:
            raise ValueError('Identidad remota inválida: '+key)
    kind = _one_input(form, 'tipo_popUp')
    if kind not in ('', '12'): raise ValueError('El tipo de ventana no corresponde a bitácora.')
    tables = form.xpath('.//table[@id="TablaInforme"]')
    if len(tables) != 1: raise ValueError('La captura no contiene una tabla de bitácora única.')
    table = tables[0]; rows = table.xpath('.//tr')
    if not rows or tuple(normalize(' '.join(c.text_content().split())) for c in rows[0].xpath('./td|./th')) != tuple(map(normalize, HEADERS)):
        raise ValueError('Cambió la estructura de la tabla de bitácora.')
    hidden = {}
    for field in table.xpath('.//input'):
        name = field.get('name', '')
        if not name.startswith(FULL_FIELDS): continue
        if field.get('type', '').lower() != 'hidden' or name in hidden:
            raise ValueError('Campo de texto completo inválido o repetido.')
        hidden[name] = field.get('value', '')
    base = {'tribunal_codigo': context['COD_Tribunal'], 'causa_id': context['CRR_IdCausa'],
            'ingreso_id': context['ID_Ingreso'], 'rit': _one_input(form, 'RIT_Causa'),
            'persona': _one_input(form, 'GLS_Nombre'), 'centro': _one_input(form, 'GLS_Centro')}
    entries = []; ids = set(); used = set(); warnings = []; row_metadata = []
    for number, row in enumerate(rows[1:], 1):
        cells = row.xpath('./td')
        if len(cells) != len(HEADERS): raise ValueError('Fila de bitácora incompleta.')
        values = [' '.join(cell.text_content().split()) for cell in cells]
        actions = cells[-1].xpath('.//img[@id]')
        entry_ids = {field.get('id') for field in actions}
        if len(entry_ids) != 1: raise ValueError('No se puede vincular la fila con su observación.')
        entry_id = entry_ids.pop()
        if not entry_id.isdigit() or int(entry_id) <= 0 or entry_id in ids:
            raise ValueError('La tabla contiene una identidad de observación inválida o repetida.')
        ids.add(entry_id)
        names = [prefix+entry_id for prefix in FULL_FIELDS]
        if any(name not in hidden for name in names):
            raise ValueError('Falta el texto completo o metadatos de una observación.')
        used.update(names)
        center, response, state, tribunal_user, send = [hidden[name] for name in names]
        if response.strip() == '---': response = ''
        response_date = _date(values[5], optional=True)
        if bool(response.strip()) != bool(response_date):
            warnings.append('Entrada '+entry_id+': respuesta y fecha del tribunal no concuerdan.')
        entries.append(dict(**base, entry_id=entry_id, fecha=_date(values[2]), autor=values[3],
            tipo=values[0], etapa=values[1], texto=center, origen='centro', respuesta=response,
            fecha_respuesta=response_date, respuesta_comprobada=True, fuente=source, fila_fuente=number))
        row_metadata.append({'entry_id': entry_id, 'state': state, 'send_to_tribunal': send,
                             'tribunal_user': tribunal_user})
    if set(hidden) != used: raise ValueError('Hay textos ocultos sin una fila correspondiente.')
    # Una captura no acredita la cobertura actual del sitio ni una consulta completa de otros ingresos.
    return {**base, 'entries': entries, 'coverage': 'PARCIAL', 'captured_at': captured_at,
            'stage_code': context['COD_Etapa'], 'source': source, 'row_metadata': row_metadata,
            'warnings': warnings, 'html_sha256': sha256(text.encode('utf-8')).hexdigest(),
            'captures': 1, 'error': 'Tabla recuperada de una respuesta de bitácora. No acredita historial remoto completo.'}
