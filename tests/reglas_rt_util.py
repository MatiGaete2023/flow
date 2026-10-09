"""Utilidades compartidas por las pruebas de aceptación R-T (documento 03 §12)."""
from datetime import date

from openpyxl import Workbook

from nurus.personal.config import defaults
from nurus.personal.work import Work

CORTE = date(2026, 10, 9)


def analizar(tmp_path, modo, encabezados, filas, cruce=None, configuracion=None, corte=CORTE):
    libro = Workbook()
    hoja = libro.active
    hoja.title = modo
    hoja.append(encabezados)
    for fila in filas:
        hoja.append(fila)
    if cruce:
        hoja2 = libro.create_sheet('Hoja2')
        for fila in cruce:
            hoja2.append(fila)
    ruta = tmp_path / f'rt_{modo.lower()}.xlsx'
    libro.save(ruta)
    trabajo = Work(configuracion or defaults())
    trabajo.analyze(ruta, modo, as_of=corte)
    return trabajo


def persona(numero, tribunal='LAJA', programa='PRM UNO'):
    return [f'X-{numero}-2026', tribunal, f'Persona Ficticia {numero}', '11111111-1', programa]
