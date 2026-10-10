"""Exportación: sin archivo residual ante error y sin fórmulas en la copia."""
from datetime import date

import pytest
from openpyxl import Workbook, load_workbook

from nurus.personal.config import defaults
from nurus.personal.work import Work

ENCABEZADOS = ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'NOMBRE CENTRO', 'T ESPERA', 'OBSERVACION']


def _trabajo(tmp_path, filas):
    libro = Workbook()
    hoja = libro.active
    hoja.title = 'ESPERA'
    hoja.append(ENCABEZADOS)
    for fila in filas:
        hoja.append(fila)
    ruta = tmp_path / 'fuente.xlsx'
    libro.save(ruta)
    trabajo = Work(defaults())
    trabajo.analyze(ruta, 'ESPERA', as_of=date(2026, 10, 9))
    return trabajo


def test_exportacion_fallida_no_deja_archivo_y_restaura_output(tmp_path):
    trabajo = _trabajo(tmp_path, [
        ['X-1-2026', 'LAJA', '=HYPERLINK("https://ejemplo.invalid","Persona")', '11111111-1', 'PRM UNO', 40, ''],
        ['X-2-2026', 'LAJA', 'Persona Ficticia', '22222222-2', 'PRM UNO', 40, ''],
    ])
    antes = (trabajo.output, trabajo.output_hash)
    destino = tmp_path / 'salida.xlsx'
    with pytest.raises(Exception):
        trabajo.export(destino, backend='portable', reduced_fidelity=True)
    assert not destino.exists()
    assert (trabajo.output, trabajo.output_hash) == antes


def test_observacion_formula_se_exporta_como_texto(tmp_path):
    trabajo = _trabajo(tmp_path, [['X-1-2026', 'LAJA', 'Persona Ficticia', '11111111-1', 'PRM UNO', 40, '=1+1']])
    destino = tmp_path / 'salida.xlsx'
    trabajo.export(destino, backend='portable', reduced_fidelity=True)
    hoja = load_workbook(destino)['ESPERA']
    columnas = {celda.value: celda.column for celda in hoja[1]}
    celda = hoja.cell(2, columnas['OBSERVACION'])
    assert celda.value == '=1+1' and celda.data_type == 's'
    assert not any(c.data_type == 'f' for fila in hoja.iter_rows() for c in fila)
