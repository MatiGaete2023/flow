"""Prueba de usuario 13-oct: bitácoras de Espera leídas sin entradas aunque el HTML las mostraba."""

import pytest

from nurus.bitacora_html import read_snapshot
from test_lectura_bitacoras import popup

PARAMS = {
    'tipo_popUp': '12',
    'COD_Tribunal': '113',
    'CRR_IdCausa': '100',
    'ID_Ingreso': '201',
    'COD_Etapa': '3',
    'TIP_Consulta': '2',
    'FLG_MejorNinez': '0',
}


def test_tabla_integra_se_lee():
    assert len(read_snapshot(popup(PARAMS), PARAMS)['entries']) == 1


@pytest.mark.parametrize('corte', ['</table><table>', '</table>'])
def test_observaciones_fuera_de_la_tabla_no_se_leen_como_vacia(corte):
    html = popup(PARAMS)
    cabecera_fin = html.index('</tr>') + len('</tr>')
    cortado = html[:cabecera_fin] + corte + html[cabecera_fin:]
    with pytest.raises(ValueError, match='fuera de la tabla'):
        read_snapshot(cortado, PARAMS)
