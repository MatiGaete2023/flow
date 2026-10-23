from datetime import date
from pathlib import Path

import pytest

from nurus.historia_html import HEADERS, read_historia, separar_ventana

FIXTURE = (
    Path(__file__).resolve().parents[1] / 'descargador' / 'fixtures' / 'historia_ficticia' / 'ventana_historia.html'
)


def base():
    return FIXTURE.read_text(encoding='utf-8')


def test_lee_filas_visibles_y_fila_final_incompleta():
    resultado = read_historia(base().encode('cp1252'))
    assert [f['folio'] for f in resultado['filas']] == ['9120', '9119', '9117']
    primera, segunda, tercera = resultado['filas']
    assert primera['tramite'] == 'Resolución' and primera['fecha_iso'] == '2099-10-13'
    assert primera['descripcion'] == 'Tengase presente canario'
    assert segunda['referencia'] == 'Sin referencia.' and segunda['anexo'] is True
    assert primera['anexo'] is False and tercera['etapa'] == 'Cumplimiento'
    assert len(tercera['descripcion']) > 60
    assert resultado['advertencias'] == [{'codigo': 'HISTORIA_FILA_INCOMPLETA', 'fila': 4, 'folio': '9001'}]
    assert resultado['vacio_explicito'] is False and len(resultado['sha256']) == 64


def test_no_lee_scripts_ni_enlaces_de_documentos():
    resultado = read_historia(base())
    texto = repr(resultado)
    for dato in ('TOKENDOCCANARIO', 'InformeHistoria', 'ShowPDF', 'REQCANARIO'):
        assert dato not in texto


def test_encabezado_de_causa_y_reserva():
    resultado = read_historia(base())
    assert resultado['encabezado']['RIT'] == 'C-99999-2099'
    assert resultado['encabezado']['Tribunal'] == 'Juzgado Canario de Prueba'
    assert resultado['encabezado']['Estado Proc.'] == 'Concluido'
    assert resultado['reservada'] is True
    sin_reserva = read_historia(base().replace('<strong>[Causa Reservada]</strong>', ''))
    assert sin_reserva['reservada'] is False


def test_tabla_vacia_con_encabezado_exacto_es_vacio_explicito():
    html = base()
    inicio, fin = html.index('<tbody>') + len('<tbody>'), html.index('</tbody>')
    resultado = read_historia(html[:inicio] + html[fin:])
    assert resultado['filas'] == [] and resultado['vacio_explicito'] is True


@pytest.mark.parametrize(
    ('cambio', 'codigo'),
    [
        (lambda h: h.replace('<th>Referencia</th>', '<th>Referencias</th>'), 'HISTORIA_ENCABEZADO_DISTINTO'),
        (lambda h: h.replace('<td class="textoBarra">Sin referencia.</td>', ''), 'HISTORIA_ENCABEZADO_DISTINTO'),
        (
            lambda h: h.replace('<table class="table', '<div class="table').replace('</table>', '</div>'),
            'HISTORIA_TABLA_NO_RECONOCIDA',
        ),
        (lambda h: h + h[h.index('<table') : h.index('</table>') + 8], 'HISTORIA_TABLA_NO_RECONOCIDA'),
        (lambda h: '<html><body><form><input type="password" name="clave"></form></body></html>', 'HISTORIA_SESION'),
        (lambda h: '', 'HISTORIA_TABLA_NO_RECONOCIDA'),
        (lambda h: h.replace('</html>', ''), 'HISTORIA_RESPUESTA_INCOMPLETA'),
    ],
)
def test_desvios_son_error_nunca_vacio(cambio, codigo):
    with pytest.raises(ValueError, match=codigo):
        read_historia(cambio(base()))


def test_fecha_ilegible_se_conserva_con_advertencia():
    resultado = read_historia(base().replace('>09/10/2099<', '>9-10-2099<'))
    fila = next(f for f in resultado['filas'] if f['folio'] == '9119')
    assert fila['fecha_iso'] == '' and fila['fecha_texto'] == '9-10-2099'
    assert {'codigo': 'HISTORIA_FECHA_ILEGIBLE', 'fila': 2, 'folio': '9119'} in resultado['advertencias']


def test_texto_con_igual_inicial_queda_literal():
    resultado = read_historia(base().replace('Tengase presente canario', '=HYPERLINK("x")'))
    assert resultado['filas'][0]['descripcion'] == '=HYPERLINK("x")'


def test_ventana_de_cuatro_meses():
    filas = read_historia(base().replace('>09/10/2099<', '>bad<'))['filas']
    dentro, anteriores, posteriores, sin_fecha = separar_ventana(filas, date(2099, 10, 13))
    assert [f['folio'] for f in dentro] == ['9120', '9117']
    assert [f['folio'] for f in sin_fecha] == ['9119']
    dentro, anteriores, posteriores, _ = separar_ventana(filas, date(2100, 3, 1))
    assert [f['folio'] for f in anteriores] == ['9120', '9117'] and not dentro
    _, _, posteriores, _ = separar_ventana(filas, date(2099, 10, 5))
    assert [f['folio'] for f in posteriores] == ['9120']


def test_encabezados_publicados():
    assert len(HEADERS) == 9 and HEADERS[0] == 'Folio' and HEADERS[-1] == 'Fecha Tram'


@pytest.mark.parametrize('corte', ['<tbody>', '9120</td>', '>9119</td>'])
def test_respuesta_cortada_es_error_nunca_vacio_ni_fila_incompleta(corte):
    html = base()
    with pytest.raises(ValueError, match='HISTORIA_RESPUESTA_INCOMPLETA'):
        read_historia(html[: html.index(corte) + len(corte)])


def test_encabezado_repetido_no_se_lee_como_tramite():
    html = base()
    encabezado = html[html.index('<thead>') + 7 : html.index('</thead>')]
    resultado = read_historia(html.replace('<tbody>', '<tbody>' + encabezado, 1))
    assert [f['folio'] for f in resultado['filas']] == ['9120', '9119', '9117']
    assert resultado['advertencias'][0]['codigo'] == 'HISTORIA_ENCABEZADO_REPETIDO'


@pytest.mark.parametrize(
    'marca',
    [
        '<ul class="pagination"><li>2</li></ul>',
        '<input type="hidden" name="nroPagina" value="1">',
        '<a href="#" onclick="x()">Siguiente</a>',
    ],
)
def test_posible_paginacion_se_advierte_y_la_muestra_sin_paginacion_no(marca):
    assert all(
        a['codigo'] != 'HISTORIA_POSIBLE_PAGINACION' for a in read_historia(base().encode('cp1252'))['advertencias']
    )
    paginada = base().replace('</body>', marca + '</body>')
    codigos = [a['codigo'] for a in read_historia(paginada.encode('cp1252'))['advertencias']]
    assert 'HISTORIA_POSIBLE_PAGINACION' in codigos
    assert 'Siguiente' not in base()
