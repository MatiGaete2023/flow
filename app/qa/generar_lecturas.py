"""E-Q1 · Oráculo vigente de las ventanas de contexto: bitácora, historia, calendario, Escritos e Histórico.

Ejecuta los lectores de Flow Python (`nurus.bitacora_html`, `historia_html`, `calendario_html` y `escritos_html`) sobre
ventanas HTML ficticias y guarda entrada y salida en `app/tests/datos/lecturas.json`. Sin red ni datos reales.

Uso (desde la raíz): python app/qa/generar_lecturas.py [salida.json]
"""

import json
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'src'), str(RAIZ / 'tests'), str(RAIZ / 'descargador')]

from bitacora_fixtures import PARAMS, popup  # noqa: E402
from nurus.bitacora_html import read_snapshot  # noqa: E402
from nurus.calendario_html import read_calendar  # noqa: E402
from nurus.escritos_html import read_escritos, read_historico  # noqa: E402
from nurus.historia_html import read_historia  # noqa: E402

CAPTURADA = '2026-11-12T12:00:00Z'


def salida(funcion, *args, **kwargs):
    try:
        return {'ok': funcion(*args, **kwargs)}
    except ValueError as e:
        return {'error': str(e)}


def casos():
    lista = []
    for texto, respuesta in [('Texto completo que la tabla oculta.', '---'),
                             ('Observación ficticia con tildes, ñ y = literal.', 'Respuesta del tribunal')]:
        html = popup(texto, respuesta).replace('P-12-2026', 'P-12-2099').replace('Persona de prueba', 'Persona Ficticia')
        if respuesta != '---':
            html = html.replace('<td>---</td>', '<td>13/08/2026 11:00</td>', 1)
        lista.append({'tipo': 'bitacora', 'html': html, 'params': PARAMS,
                      'python': salida(read_snapshot, html, PARAMS, source='ficticio.html', captured_at=CAPTURADA)})
    html = (RAIZ / 'descargador/fixtures/historia_ficticia/ventana_historia.html').read_text(encoding='utf-8')
    vacia = html[:html.index('<tbody>') + 7] + html[html.index('</tbody>'):]
    for variante in [html, vacia, html.replace('>09/10/2099<', '>ilegible<'),
                     html.replace('</body>', '<ul class="pagination"></ul></body>')]:
        lista.append({'tipo': 'historia', 'html': variante,
                      'python': salida(read_historia, variante, source='ficticio.html', captured_at=CAPTURADA)})
    args = ['100', '300', '49', '400', 'X-1-2099', '01/03/2026', 'PRM Ficticio', 'Persona Ficticia', '6 Meses', '3 Meses',
            '201', '1']
    for filas in ['', '<tr><td>01/06/2026 00:00</td><td>28/05/2026 13:16:05</td><td>ENVIADO</td></tr>'
                      '<tr><td>01/09/2026</td><td>---</td><td>PENDIENTE</td></tr>']:
        html = ('<html><body><form name="InformesPpalForm"><input name="COD_Tribunal" value="49">'
                '<input name="CRR_IdCausa" value="100"><input name="ID_Ingreso" value="201"><table><tr>'
                '<td>Plazo Remisión: 3 Meses</td><td>Plazo Intervención: 6 Meses</td></tr></table><table><tr>'
                '<th>Fecha Proyección</th><th>Fecha Informe</th><th>Estado</th></tr>' + filas + '</table></form></body></html>')
        lista.append({'tipo': 'calendario', 'html': html, 'args': args,
                      'python': salida(read_calendar, html, args, source='ficticio.html', captured_at=CAPTURADA)})

    cabecera = '<thead><tr><th>Estado</th><th>Solicitante</th><th>Doc.</th><th>Tipo Escrito</th></tr></thead>'
    escritos = {
        'con_filas': '<table>' + cabecera + '<tbody><tr><td>Pendiente</td><td>Programa Ficticio</td><td>DOC-1</td>'
                     '<td>Inf. Cump Programa Ambulatorio</td></tr><tr><td>Resuelto</td><td>Programa Ficticio</td>'
                     '<td>DOC-2</td><td>Otro escrito</td></tr></tbody></table>',
        'vacia': '<table>' + cabecera + '<tbody></tbody></table>',
        'ilegible': '<table><thead><tr><th>Tipo Escrito</th><th>Estado</th></tr></thead><tbody><tr><td>x</td></tr>'
                    '</tbody></table>',
        'ausente': '<p>Sin bandeja</p>',
    }
    for nombre, bloque in escritos.items():
        html = '<html><body>' + bloque + '</body></html>'
        lista.append({'tipo': 'escritos', 'caso': nombre, 'html': html,
                      'python': salida(read_escritos, html.encode(), source='ficticio.html', captured_at=CAPTURADA)})

    historico = ('<html><body><div>Rut : 11.111.111-2</div><table><thead><tr><th>Nombre Proyecto</th><th>Fecha egreso</th>'
                 '<th>Causal egreso</th></tr></thead><tbody><tr><td>PRM Ficticio</td><td>--</td><td>SIN INFORMACION</td>'
                 '</tr><tr><td>FAE Otro Ficticio</td><td>--</td><td>SIN INFORMACION</td></tr><tr><td>PDE Antiguo</td>'
                 '<td>01/05/2026</td><td>EGRESO</td></tr><tr><td></td><td></td><td></td></tr></tbody></table></body></html>')
    for nombre, persona, rut in [('rut_esperado', ['11.111.111-2'], ''), ('sin_persona', [], ''),
                                 ('nombre_suelto', ['Ana'], ''), ('rut_visible_igual', [], '11111111-2'),
                                 ('rut_visible_distinto', ['11.111.111-2'], '22222222-3')]:
        lista.append({'tipo': 'historico', 'caso': nombre, 'html': historico, 'persona': persona, 'rut_visible': rut,
                      'python': salida(read_historico, historico.encode(), source='ficticio.html', captured_at=CAPTURADA,
                                       persona_esperada=persona, rut_visible=rut)})
    lista.append({'tipo': 'historico', 'caso': 'sin_tabla', 'html': '<html><body><p>x</p></body></html>', 'persona': [],
                  'rut_visible': '', 'python': salida(read_historico, b'<html><body><p>x</p></body></html>',
                                                      source='ficticio.html', captured_at=CAPTURADA)})
    return lista


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'lecturas.json'
    lista = casos()
    destino.parent.mkdir(parents=True, exist_ok=True)
    destino.write_text(json.dumps(lista, ensure_ascii=False, indent=1) + '\n', encoding='utf-8', newline='\n')
    print(f'{len(lista)} ventanas ficticias cotejadas con Python → {destino}')


if __name__ == '__main__':
    main()
