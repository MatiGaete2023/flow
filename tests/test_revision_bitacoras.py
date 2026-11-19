"""P2: detector de recorte de la ventana de bitácora y período por defecto (sin red ni Tk)."""

from datetime import date
from pathlib import Path

import pytest
from lxml import html

from nurus.bitacora_html import alcance_bitacora, decode_snapshot
from nurus.personal.revision_bitacoras import NO_VERIFICABLE, SIN, clasificar, periodo_por_defecto

GOLDEN = Path(__file__).parents[1] / 'docs/evidencia_p0/golden/Bitacora_ficticia/captura.html'


def _doc(extra=''):
    texto = decode_snapshot(GOLDEN.read_bytes()).replace('</body>', extra + '</body>', 1)
    return html.fromstring(texto)


def test_ventana_con_estructura_real_no_se_marca_como_recortada():
    assert alcance_bitacora(_doc(), False) == 'COMPLETA'
    assert alcance_bitacora(_doc(), True) == 'VACIA_COMPROBADA'


@pytest.mark.parametrize(
    'extra',
    [
        '<a href="#" onclick="irPagina(2)">2</a>',
        '<button>Ver más</button>',
        '<input type="button" value="Siguiente">',
        '<iframe src="x"></iframe>',
        '<div class="pagination"></div>',
        '<input type="hidden" name="FechaDesde" value="">',
        '<a href="#">&raquo;</a>',
    ],
)
def test_indicios_de_recorte(extra):
    assert alcance_bitacora(_doc(extra), False) == 'PARCIAL'


def test_texto_de_observacion_en_campo_oculto_no_cuenta_como_rotulo():
    assert alcance_bitacora(_doc('<input type="hidden" name="obs" value="ver más antecedentes">'), False) == 'COMPLETA'


def test_periodo_por_defecto_es_el_mes_que_termina_en_el_corte():
    assert periodo_por_defecto(date(2026, 10, 16)) == (date(2026, 9, 16), date(2026, 10, 16))
    assert periodo_por_defecto(date(2026, 3, 31)) == (date(2026, 2, 28), date(2026, 3, 31))
    assert periodo_por_defecto(date(2026, 1, 15)) == (date(2025, 12, 15), date(2026, 1, 15))


def test_sin_datos_o_con_advertencias_no_es_sin_observacion():
    base = {'leida': True, 'alcance': 'VACIA_COMPROBADA', 'completa': True, 'capturada': '2026-10-20', 'fechas_centro': []}
    assert clasificar(base, date(2026, 9, 20), date(2026, 10, 20))[0] == SIN
    assert clasificar({**base, 'completa': False}, date(2026, 9, 20), date(2026, 10, 20))[0] == NO_VERIFICABLE
    assert clasificar({**base, 'alcance': ''}, date(2026, 9, 20), date(2026, 10, 20))[0] == NO_VERIFICABLE
    assert clasificar({}, date(2026, 9, 20), date(2026, 10, 20))[0] == NO_VERIFICABLE


def test_leer_fecha_y_periodo():
    from nurus.personal.revision_bitacoras import leer_fecha, periodo

    assert leer_fecha('16/09/2026') == leer_fecha('2026-09-16') == leer_fecha('16-09-2026') == date(2026, 9, 16)
    with pytest.raises(ValueError, match='dd/mm/aaaa'):
        leer_fecha('31/02/2026')
    with pytest.raises(ValueError, match='posterior'):
        periodo('17/10/2026', '16/10/2026')
