"""P4: biblioteca de productos de Resultados (sin Tk)."""

import os
import time

import pytest

from nurus.personal.biblioteca import biblioteca, filtrar, tamano


def test_lista_productos_recientes_con_busqueda_sin_tildes(tmp_path):
    (tmp_path / 'Corrida 2026-10-01').mkdir()
    viejo = tmp_path / 'Corrida 2026-10-01' / 'Nómina Laja.xlsx'
    viejo.write_bytes(b'x')
    nuevo = tmp_path / 'Resoluciones_2026-10-08.docx'
    nuevo.write_bytes(b'xy')
    (tmp_path / 'notas.tmp').write_text('ignorado')
    (tmp_path / '~$Resoluciones.docx').write_text('bloqueo de Word')
    os.utime(viejo, (time.time() - 3600, time.time() - 3600))
    items = biblioteca(tmp_path)
    assert [i['nombre'] for i in items] == ['Resoluciones_2026-10-08.docx', 'Nómina Laja.xlsx']
    assert items[1]['carpeta'] == 'Corrida 2026-10-01'
    assert [i['nombre'] for i in filtrar(items, 'nomina laja')] == ['Nómina Laja.xlsx']
    assert [i['nombre'] for i in filtrar(items, 'corrida xlsx')] == ['Nómina Laja.xlsx']
    assert filtrar(items, '') == items and filtrar(items, 'inexistente') == []
    assert biblioteca(tmp_path / 'no existe') == []
    assert tamano(10) == '10 B' and tamano(2048) == '2.0 KB'


@pytest.mark.skipif(os.name == 'nt', reason='enlaces simbólicos requieren privilegio en Windows')
def test_no_sigue_enlaces_simbolicos(tmp_path):
    fuera = tmp_path / 'fuera'
    fuera.mkdir()
    (fuera / 'ajeno.xlsx').write_bytes(b'x')
    base = tmp_path / 'salidas'
    base.mkdir()
    (base / 'enlace').symlink_to(fuera, target_is_directory=True)
    (base / 'archivo.xlsx').symlink_to(fuera / 'ajeno.xlsx')
    assert biblioteca(base) == []
