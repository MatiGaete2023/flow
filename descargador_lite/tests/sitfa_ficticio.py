"""SITFA ficticio para pruebas: personas, RIT y tribunales inventados. No accede a la red."""

import base64
import hashlib
import io
import json
from html import escape

from PIL import Image

TRIBUNALES = {'901': 'Juzgado de Familia Ficticio Norte', '902': 'Juzgado de Familia Ficticio Sur'}
CATALOGS = {
    'seguimiento': {
        'pantalla': 'seguimiento',
        'tribunales': TRIBUNALES,
        'modalidades': {'1': 'Residencia', '2': 'Ambulatorio', '3': 'FAE', '4': 'DCE'},
        'informes': {'1': 'Por vencer (a 25 días)', '2': 'Vencidos', '3': 'Recibidos'},
        'medidas': {'4': 'Por vencer (a 45 días)', '5': 'Vencidos'},
        'pestanas': ['Espera', 'Cumplimiento', 'Informes', 'Egresados'],
        'seleccion': {'tribunal': '901', 'modalidad': '2', 'desde': '01/10/2026', 'hasta': '30/10/2026'},
    },
    'litigantes': {
        'pantalla': 'litigantes',
        'tribunales': TRIBUNALES,
        'modalidades': {},
        'informes': {},
        'medidas': {},
        'pestanas': [],
        'estados': {'1': 'Pendientes', '2': 'Cumplidas'},
        'meses': {},
        'anios': {},
        'seleccion': {'estado': '1'},
    },
    'calendario_informes': {
        'pantalla': 'calendario_informes',
        'tribunales': TRIBUNALES,
        'modalidades': {},
        'informes': {},
        'medidas': {},
        'pestanas': [],
        'estados': {},
        'meses': {str(i): n for i, n in enumerate(['Enero', 'Febrero', 'Marzo', 'Abril', 'Mayo', 'Junio', 'Julio',
                                                    'Agosto', 'Septiembre', 'Octubre', 'Noviembre', 'Diciembre'], 1)},
        'anios': {'2026': '2026', '2027': '2027'},
        'seleccion': {'mes': '10', 'anio': '2026'},
    },
}
CATALOGS['calendario_medidas'] = {**CATALOGS['calendario_informes'], 'pantalla': 'calendario_medidas'}
TABS = {'Espera': ('tdEspera', 'NUM_PaginaEspera', 'NUM_TotalEspera', 'tablaEspera'),
        'Cumplimiento': ('tdCumplimiento', 'NUM_PaginaCumplimiento', 'NUM_TotalCumplimiento', 'tablaCumplimiento'),
        'Informes': ('tdInforme', 'NUM_PaginaInforme', 'NUM_TotalInforme', 'tablaInforme'),
        'Egresados': ('tdEgreso', 'NUM_PaginaEgreso', 'NUM_TotalEgreso', 'tablaEgresados')}
TEMPLATES = {'litigantes': 'BusquedaLit', 'calendario_informes': 'InfoPorVencer', 'calendario_medidas': 'MedPorVencer'}
ACTIONS = {'seguimiento': 'Buscar medida', 'litigantes': 'Consulta Informe',
           'calendario_informes': 'Buscar Inf.', 'calendario_medidas': 'Buscar'}


def b64(data):
    return base64.b64encode(data).decode()


def png():
    out = io.BytesIO()
    Image.new('RGB', (800, 400), 'white').save(out, 'PNG')
    return out.getvalue()


class FakeBridge:
    """Responde como la extensión Lite. `pages` = {clave_consulta: número de páginas (0 = sin registros)}."""

    def __init__(self, screen, pages=None, default_pages=1):
        self.screen = screen
        self.pages = pages or {}
        self.default_pages = default_pages
        self.calls = []
        self.current = None
        self.connected = True

    def key(self, p):
        if self.screen == 'seguimiento':
            tab = next(k for k, v in TABS.items() if v[0] == p['COD_Lengueta'])
            return (p['COD_Tribunal_sel'], tab, p['TIP_Consulta'], p.get('TIP_Informe'))
        if self.screen == 'litigantes':
            return (p['COD_Tribunal_sel'], p['COD_EstBusqueda'])
        return (p['COD_Tribunal_sel'], p['COD_Mes_sel'])

    def total(self, p):
        return self.pages.get(self.key(p), self.default_pages)

    def call(self, command, payload=None, timeout=75):
        self.calls.append(command)
        payload = payload or {}
        if command == 'capabilities':
            return {'tipo': 'SITFA_LITE_CAPACIDADES', 'version': '1.0.0',
                    'protocolos': {'descarga_xls': 1, 'pdf_sin_registros': 1}, 'escritura_rus': False}
        if command in ('lock', 'unlock', 'evidence_close'):
            return {'ok': True, 'restored': True}
        if command == 'catalog':
            return CATALOGS[self.screen]
        if command == 'prepare':
            return {'pairs': self.prepare(payload['selection'], payload['dates'])}
        if command == 'search':
            p = dict(payload['pairs'])
            self.current = p
            body = self.page_html(p).encode('utf-8')
            return {'status': 200, 'type': 'text/html; charset=utf-8', 'body': b64(body)}
        if command == 'download':
            return {'status': 200, 'type': 'application/vnd.ms-excel', 'body': b64(self.excel(self.current))}
        if command == 'evidence_open':
            p = dict(payload['pairs'])
            # Como restoreEvidenceSelection: la copia para el PDF muestra la pestaña consultada.
            html = self.page_html(p).replace('name="COD_Lengueta" value=""', f'name="COD_Lengueta" value="{p.get("COD_Lengueta", "")}"')
            context = payload['context']
            banner = (
                f'<div id="sitfaContextoConsulta" data-context="{escape(json.dumps(context, ensure_ascii=False, separators=(",", ":")))}">'
                + escape('Consulta solicitada · ' + ' · '.join(a + ': ' + b for a, b in context))
                + '</div>'
            )
            body = html.replace('<body>', '<body>' + banner).encode('utf-8')
            self.evidence_digest = hashlib.sha256(body).hexdigest()
            return {'status': 200, 'type': 'text/html; charset=utf-8', 'body': b64(body), 'digest': self.evidence_digest}
        if command == 'evidence_capture':
            return {'png': b64(png()), 'digest': payload['digest']}
        raise AssertionError('comando no permitido en Lite: ' + command)

    def prepare(self, s, dates):
        screen = s['screen']
        assert screen == self.screen
        pairs = [('COD_Tribunal_sel', s['tribunal'])]
        if screen == 'seguimiento':
            tab = TABS[s['tab']]
            pairs += [('COD_Lengueta', tab[0]), ('TIP_Consulta', s['modality'])]
            if s['tab'] == 'Informes' or s['tab'] == 'Cumplimiento':
                pairs.append(('TIP_Informe', s['report'] or '0'))
            pairs += [(tab[1], ''), (tab[2], '')]
        elif screen == 'litigantes':
            pairs += [('TIP_Consulta', '2'), ('COD_EstBusqueda', s['state']), ('NUM_PaginaInforme', ''), ('NUM_Total', '')]
        else:
            pairs += [('TIP_Consulta', '1'), ('COD_Mes_sel', s['month']), ('COD_Anio_Sel', s['year'])]
        if not screen.startswith('calendario_'):
            pairs.append(('FLG_Consulta', '1' if dates else '0'))
            if dates:
                pairs += [('CHK_Consulta', 'on'), ('FEC_Inicio', dates['start']), ('FEC_Fin', dates['end'])]
        pairs.append(('irAccion', ACTIONS[screen]))
        return [list(x) for x in pairs]

    def rows(self, p, page):
        if self.screen.startswith('calendario_'):
            return [(f'C-{page}{i}-2026', f'Persona ficticia {self.key(p)} {page}-{i}') for i in range(2)]
        return [(f'X-{page}{i}-2026', f'Persona ficticia {self.key(p)} {page}-{i}') for i in range(3)]

    def page(self, p):
        if self.screen == 'seguimiento':
            field = TABS[next(k for k, v in TABS.items() if v[0] == p['COD_Lengueta'])][1]
        elif self.screen == 'litigantes':
            field = 'NUM_PaginaInforme'
        else:
            return 1
        return int(p.get(field) or 1)

    def page_html(self, p):
        total = self.total(p)
        page = self.page(p)
        rows = self.rows(p, page) if total else []
        fields = ''.join(
            f'<select name="{k}"><option value="{escape(v)}" selected>{escape(v)}</option></select>'
            for k, v in p.items()
            if k in ('COD_Tribunal_sel', 'TIP_Consulta', 'TIP_Informe', 'COD_EstBusqueda', 'COD_Mes_sel', 'COD_Anio_Sel')
        )
        fields += '<input name="FEC_Inicio" value="{}"><input name="FEC_Fin" value="{}">'.format(
            escape(p.get('FEC_Inicio', '')), escape(p.get('FEC_Fin', ''))
        )
        template = TEMPLATES.get(self.screen, 'listdemo')
        if self.screen == 'seguimiento':
            tab = next(v for v in TABS.values() if v[0] == p['COD_Lengueta'])
            fields += '<input name="COD_Lengueta" value="">'
            fields += f'<input name="{tab[1]}" value="{page if total else 1}"><input name="{tab[2]}" value="{max(total, 1)}">'
            body = ''.join(f'<tr><td>{r}</td><td>{n}</td></tr>' for r, n in rows) or '<tr><td>Sin registros</td></tr>'
            tables = ''.join(
                f'<table id="{v[3]}"><tr><td>RIT</td><td>Nombre</td></tr>{body if v == tab else ""}</table>'
                for v in TABS.values()
            )
        elif self.screen == 'litigantes':
            fields += f'<input name="NUM_PaginaInforme" value="{page}"><input name="NUM_Total" value="{max(total, 1)}">'
            body = ''.join(f'<tr><td>{r}</td><td>{n}</td></tr>' for r, n in rows)
            tables = f'<table><tr><td>RIT</td><td>Nombre</td></tr>{body}</table>'
        else:
            body = ''.join(f'<tr><td>{r}</td></tr>' for r, _ in rows)
            tables = f'<table id="TablaCalendarioSemana">{body}</table><table id="TablaCalendarioDia">{body}</table>'
        scripts = 'function ShowExcel(){window.open("https://familia.pjud.cl/sitfa/reportes/u1_' + template + '.xls");}'
        if self.screen.startswith('calendario_'):
            scripts += ''.join(f'Ritxdias[dia] = "{r}"; Nombresxdias[dia] = "{n}";' for r, n in rows)
        return (
            '<html><body><form name="InformesPpalForm" method="post" action="/SITFAWEB/InformesDAction.do">'
            + fields + tables + f'</form><script>{scripts}</script></body></html>'
        )

    def excel(self, p):
        rows = self.rows(p, self.page(p))
        if self.screen.startswith('calendario_'):
            month = int(p['COD_Mes_sel'])
            body = ''.join(f'<tr><td>{r}</td><td>{n}</td><td>15/{month:02}/2026</td></tr>' for r, n in rows)
            return f'<html><table><tr><td>RIT</td><td>Nombre</td><td>Fecha Vencimiento</td></tr>{body}</table></html>'.encode()
        body = ''.join(f'<tr><td>{r}</td><td>{n}</td></tr>' for r, n in rows)
        return f'<html><table><tr><td>RIT</td><td>Nombre</td></tr>{body}</table></html>'.encode()
