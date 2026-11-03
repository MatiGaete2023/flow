"""Lector de Excel del Descargador: formatos, codificación y límites. Datos ficticios."""
import datetime as dt
import io
from pathlib import Path
import re
import sys
import zipfile

import pytest
from openpyxl import Workbook

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import lector_excel as lx  # noqa: E402
import biff_escritor  # noqa: E402


def tabla_html(nombre, meta=''):
    return (f'<html><head>{meta}</head><body><table><tr><td>RIT</td><td>NOMBRE</td><td>DERIVACIÓN</td></tr>'
            f'<tr><td>C-1-2024</td><td>{nombre}</td><td>FAE</td></tr></table></body></html>')


@pytest.mark.parametrize('data', [
    tabla_html('MUÑOZ – PÉREZ').encode('utf-8'),
    tabla_html('MUÑOZ – PÉREZ').encode('cp1252'),
    tabla_html('MUÑOZ – PÉREZ', '<meta charset="iso-8859-1">').encode('cp1252'),
    tabla_html('MUÑOZ – PÉREZ', '<meta http-equiv="Content-Type" content="text/html; charset=windows-1252">').encode('cp1252'),
    tabla_html('MUÑOZ – PÉREZ', '<meta charset="utf-8">').encode('utf-8'),
    b'\xef\xbb\xbf' + tabla_html('MUÑOZ – PÉREZ').encode('utf-8'),
], ids=['utf8-sin-meta', 'cp1252-sin-meta', 'latin1-declarado', 'cp1252-declarado', 'utf8-declarado', 'utf8-bom'])
def test_html_tabular_conserva_tildes(data):
    info = lx.read_excel(data)
    assert info.format == 'HTML-tabular-con-extension-xls'
    assert info.grid == [['RIT', 'NOMBRE', 'DERIVACIÓN'], ['C-1-2024', 'MUÑOZ – PÉREZ', 'FAE']]


def xlsx(rows, dimension=None):
    wb = Workbook()
    for row in rows:
        wb.active.append(row)
    buffer = io.BytesIO()
    wb.save(buffer)
    if dimension is None:
        return buffer.getvalue()
    source, target = zipfile.ZipFile(io.BytesIO(buffer.getvalue())), io.BytesIO()
    with zipfile.ZipFile(target, 'w') as out:
        for item in source.infolist():
            body = source.read(item.filename)
            if item.filename == 'xl/worksheets/sheet1.xml':
                body = re.sub(rb'<dimension ref="[^"]+"', b'<dimension ref="' + dimension.encode() + b'"', body)
            out.writestr(item, body)
    return target.getvalue()


def test_ooxml_con_dimension_desactualizada_lee_todas_las_filas():
    rows = [['RIT', 'NOMBRE', 'DERIVACIÓN']] + [[f'C-{i}-2024', f'PERSONA {i}', 'FAE'] for i in range(1, 7)]
    for dimension in (None, 'A1:C3', 'A1'):
        info = lx.read_excel(xlsx(rows, dimension))
        assert sum(info.records.values()) == 6, dimension
        assert all(len(row) == 3 for row in info.grid)


def test_ooxml_filas_irregulares_se_completan_con_vacios():
    info = lx.read_excel(xlsx([['Título'], [], ['RIT', 'NOMBRE', 'X', 'Y'], ['C-1-2024', 'ANA']]))
    assert info.grid == [('Título', None, None, None), (None, None, None, None),
                         ('RIT', 'NOMBRE', 'X', 'Y'), ('C-1-2024', 'ANA', None, None)]


def test_biff_lee_texto_numero_y_fecha():
    data = biff_escritor.xls([('Hoja1', [['RIT', 'NOMBRE', 'T ESPERA', 'FECHA'],
                                         ['C-1-2024', 'ANA', 13, ('date', 46188)]])])
    info = lx.read_excel(data)
    assert info.format == 'XLS-BIFF'
    assert info.grid[1] == ['C-1-2024', 'ANA', 13.0, dt.datetime(2026, 6, 15)]


def test_xml_spreadsheet_2003_con_indices_y_fecha():
    data = ('<?xml version="1.0"?><Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet" '
            'xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet"><Worksheet ss:Name="H"><Table>'
            '<Row><Cell><Data ss:Type="String">RIT</Data></Cell><Cell><Data ss:Type="String">NOMBRE</Data></Cell>'
            '<Cell ss:Index="4"><Data ss:Type="String">FECHA</Data></Cell></Row>'
            '<Row ss:Index="3"><Cell><Data ss:Type="String">C-1-2024</Data></Cell><Cell><Data ss:Type="String">ANA'
            '</Data></Cell><Cell ss:Index="4"><Data ss:Type="DateTime">2026-06-15T00:00:00.000</Data></Cell></Row>'
            '</Table></Worksheet></Workbook>').encode()
    info = lx.read_excel(data)
    assert info.format == 'XML-Spreadsheet-2003'
    assert info.grid == [['RIT', 'NOMBRE', '', 'FECHA'], [], ['C-1-2024', 'ANA', '', dt.date(2026, 6, 15)]]


@pytest.mark.parametrize('data,mensaje', [
    (b'', 'vacio'),
    (b'<html><form><input type="password"></form></html>', 'Sesion expirada'),
    (b'<html><script>x()</script>' + tabla_html('ANA').encode(), 'pagina web activa'),
    (b'<html><table><tr><td>sin listado</td></tr></table></html>', 'unico listado'),
    (tabla_html('ANA').encode() * 2, 'unico listado'),
    (b'PK\x03\x04basura', 'parsear'),
])
def test_rechaza_lo_que_no_es_un_listado_unico(data, mensaje):
    with pytest.raises(lx.PocError, match=mensaje):
        lx.read_excel(data)


def test_ooxml_demasiado_ancho_se_rechaza():
    with pytest.raises(lx.PocError, match='límites'):
        lx.read_excel(xlsx([['RIT', 'NOMBRE'] + ['x'] * 205, ['C-1-2024', 'ANA']], 'A1:B2'))
