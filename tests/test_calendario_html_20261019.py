"""WD-1/WA-6: lector del calendario de informes por ingreso y cotejo CAL-03; estructura ficticia."""

from datetime import date

import pytest

from nurus.calendario_html import read_calendar
from nurus.personal.calendario_ingreso import avisos_cotejo, primera_pendiente, proxima_pendiente

ARGS = ['100', '300', '49', '400', 'X-1-2026', '01/03/2026', 'PRM Centro', 'Persona', '6 Meses', '3 Meses', '201', '1']


def page(rows, *, ingreso='201', header=('Fecha Proyección', 'Fecha Informe', 'Estado'), extra=''):
    head = ''.join(f'<th>{h}</th>' for h in header)
    body = ''.join('<tr>' + ''.join(f'<td>{v}</td>' for v in row) + '</tr>' for row in rows)
    return (
        '<html><body><form name="InformesPpalForm">'
        '<input type="hidden" name="COD_Tribunal" value="49"><input type="hidden" name="CRR_IdCausa" value="100">'
        f'<input type="hidden" name="ID_Ingreso" value="{ingreso}">{extra}'
        '<table><tr><td>Plazo Remisión: 3 Meses</td><td>Plazo Intervención: 6 Meses</td></tr></table>'
        f'<table><tr>{head}</tr>{body}</table></form></body></html>'
    )


def test_lee_proyecciones_estados_y_plazos():
    result = read_calendar(
        page([('01/06/2026', '28/05/2026', 'ENVIADO'), ('01/09/2026', '---', 'PENDIENTE')]), ARGS, source='c.html'
    )
    assert result['proyecciones'] == [
        {'fila': 1, 'fecha_proyeccion': '2026-06-01', 'fecha_informe': '2026-05-28', 'estado': 'ENVIADO'},
        {'fila': 2, 'fecha_proyeccion': '2026-09-01', 'fecha_informe': '', 'estado': 'PENDIENTE'},
    ]
    assert (result['plazo_remision_meses'], result['plazo_intervencion_meses']) == (3, 6)
    assert len(result['html_sha256']) == 64 and result['ingreso_id'] == '201'
    assert read_calendar(page([]), ARGS)['proyecciones'] == []


@pytest.mark.parametrize(
    'text, message',
    [
        (page([('01/06/2026', '', 'PENDIENTE')], ingreso='999'), 'no corresponde'),
        (page([('01/06/2026', '', 'PENDIENTE')]).replace('</html>', ''), 'incompleta'),
        (page([], extra='<input type="password" name="clave">'), 'ingreso de SITFA'),
        (page([('01/06/2026', '', 'RECIBIDO')]), 'no reconocido'),
        (page([('01/06/2026', '', 'ENVIADO')]), 'no concuerdan'),
        (page([('01/06/2026', '02/06/2026', 'PENDIENTE')]), 'no concuerdan'),
        (page([('1 de junio', '', 'PENDIENTE')]), 'ilegible'),
        (page([('01/06/2026', 'PENDIENTE')]), 'incompleta'),
        (page([], header=('Fecha', 'Informe', 'Estado')), 'estructura'),
    ],
)
def test_cualquier_desvio_es_error_y_nunca_sin_proyecciones(text, message):
    with pytest.raises(ValueError, match=message):
        read_calendar(text, ARGS)


ITEMS = [(date(2026, 6, 1), 'ENVIADO'), (date(2026, 9, 1), 'PENDIENTE'), (date(2026, 12, 1), 'PENDIENTE')]
CORTE = date(2026, 10, 19)


def test_vencimiento_es_la_primera_pendiente_y_solo_pendiente_vence():
    assert primera_pendiente(ITEMS) == date(2026, 9, 1)
    assert proxima_pendiente(ITEMS, CORTE) == date(2026, 12, 1)
    assert proxima_pendiente(ITEMS[:2], CORTE) is None


def test_cal03_un_aviso_por_vista_discordante():
    assert avisos_cotejo(ITEMS, CORTE, vencimiento_informes=date(2026, 9, 1), fecha_hoja2=date(2026, 12, 1)) == []
    assert avisos_cotejo([], CORTE, vencimiento_informes=date(2026, 9, 2)) == []
    assert avisos_cotejo(ITEMS, CORTE, vencimiento_informes=date(2026, 9, 2)) == [
        'CAL-03: la pestaña Informes indica vencimiento el 2 de septiembre de 2026 y el calendario del ingreso '
        'proyecta el informe pendiente el 1 de septiembre de 2026; revisar en RUS.'
    ]
    assert avisos_cotejo(ITEMS[:2], CORTE, fecha_hoja2=date(2026, 12, 1)) == [
        'CAL-03: Hoja2 (calendario 91) indica el 1 de diciembre de 2026 y el calendario del ingreso no tiene un '
        'próximo informe pendiente; revisar en RUS.'
    ]
