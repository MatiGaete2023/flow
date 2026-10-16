import html
import pytest
from nurus.personal.bitacora_har import read_popup, HEADERS
from nurus.personal.bitacoras import cc_for_type

PARAMS = {
    'tipo_popUp': '12',
    'COD_Tribunal': '11',
    'CRR_IdCausa': '22',
    'ID_Ingreso': '33',
    'COD_Etapa': '3',
    'TIP_Consulta': '2',
}


def popup(text='Texto completo que la tabla oculta.', response='---', kind='Al tribunal'):
    fields = {
        **PARAMS,
        'tipo_popUp': '',
        'RIT_Causa': 'P-12-2026',
        'GLS_Nombre': 'Persona de prueba',
        'GLS_Centro': 'Centro de prueba',
    }
    inputs = ''.join(
        f'<input type="hidden" name="{k}" value="{html.escape(v, quote=True)}">' for k, v in fields.items()
    )
    hidden = {
        'HOBS_Centro_': '',
        'HOBS_Tribunal_': response,
        'HCOD_Estado_': '6',
        'HFUN_Tribunal_': 'USR',
        'HFLG_Envio_': '1',
    }
    hidden['HOBS_Centro_'] = text
    hidden = ''.join(
        f'<input type="hidden" name="{k}44" value="{html.escape(v, quote=True)}">' for k, v in hidden.items()
    )
    headers = ''.join(f'<td>{h}</td>' for h in HEADERS)
    values = [kind, 'Cumplimiento', '12/08/2026 10:13', 'Centro', 'Texto recortado', '---', '---', 'Recortado']
    row = (
        ''.join(f'<td>{v}</td>' for v in values) + '<td><img id="44" onclick="MuestraDiv(this.id,3)"><img id="44"></td>'
    )
    return f'<html><body><form name="InformesPpalForm" action="/SITFAWEB/InformesDAction.do">{inputs}<table id="TablaInforme"><tr>{headers}</tr><tr>{row}</tr>{hidden}</table></form></body></html><script>/* RUS añade un script al final */</script>'


def test_full_text_response_and_actual_identity():
    text = 'Línea uno & comillas "exactas"\nLínea dos ' * 12
    query = read_popup(popup(text), PARAMS)
    entry = query['entries'][0]
    assert entry.texto == text and entry.entry_id == '44'
    assert entry.respuesta == '' and entry.respuesta_comprobada
    assert cc_for_type(entry.tipo) == 1
    assert query['coverage'] == 'PARCIAL'
    assert entry.fecha == '2026-08-12T10:13:00'


@pytest.mark.parametrize(
    'change',
    [
        lambda t: t.replace('name="ID_Ingreso" value="33"', 'name="ID_Ingreso" value="99"'),
        lambda t: t.replace('HOBS_Centro_44', 'HOBS_Centro_55'),
        lambda t: t.replace('Fecha Centro', 'Fecha desconocida'),
        lambda t: t.replace('</html>', ''),
        lambda t: t.replace('<img id="44"></td>', '<img id="45"></td>'),
    ],
)
def test_unverifiable_content_rejected(change):
    with pytest.raises(ValueError):
        read_popup(change(popup()), PARAMS)
