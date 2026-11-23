"""Fixture de E-A78: salida de Flow Python sobre bytes y grillas sintéticas fijas.

A7: `descargador/motor.py::read_excel` sobre libros SpreadsheetML 2003 y HTML ficticios.
A8: `descargador/carga.py::records_in_grid` sobre grillas de 23 columnas y `read_excel(..., carga.records_in_grid)`
sobre un XLSX ficticio con fechas de Excel.

Uso (desde la raíz): python app/qa/generar_a78.py [salida.json]   (por omisión app/tests/datos/a78.json)
Datos solo ficticios: RIT de 2099 y nombres de fantasía.
"""

import base64
import io
import json
import sys
from datetime import date, datetime
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(RAIZ / 'descargador'))

import carga  # noqa: E402
import motor  # noqa: E402

SS = ('<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet" '
      'xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet" xmlns:html="http://www.w3.org/TR/REC-html40">')


def libro(filas, prologo=''):
    return ('<?xml version="1.0"?>\n' + prologo + SS + '<Worksheet ss:Name="Hoja1"><Table>' + filas
            + '</Table></Worksheet></Workbook>').encode()


def celda(texto, indice=None, tipo='String'):
    attr = f' ss:Index="{indice}"' if indice else ''
    return f'<Cell{attr}><Data ss:Type="{tipo}">{texto}</Data></Cell>'


A7 = {
    'listado_con_indices_font_y_fecha': libro(
        '<Row ss:Index="1">' + celda('RIT', 1) + celda('Nombre', 2) + celda('Fecha', 4) + '</Row>'
        '<Row ss:Index="3">' + celda('X-100-2099', 1) + celda('<html:Font html:Color="#000000">Persona</html:Font> Ficticia', 2)
        + celda('2026-10-15T00:00:00.000', 4, 'DateTime') + '</Row>'
        '<Row>' + celda('X-101-2099') + celda('Otra Persona Ficticia') + '</Row>'
        '<Row>' + celda('X-100-2099') + celda('Persona Ficticia') + '</Row>'),
    'fila_no_creciente': libro('<Row ss:Index="5">' + celda('A') + '</Row><Row ss:Index="3">' + celda('B') + '</Row>'),
    'columna_201': libro('<Row>' + celda('X', 201) + '</Row>'),
    'mas_de_200_celdas': libro('<Row>' + celda('C') * 201 + '</Row>'),
    'entidad_externa': libro(
        '<Row>' + celda('RIT') + celda('Nombre') + '</Row><Row>' + celda('X-1-2099') + celda('&xxe;') + '</Row>',
        '<!DOCTYPE Workbook [<!ENTITY xxe SYSTEM "http://127.0.0.1:9/xxe">]>\n'),
    'sin_listado': libro('<Row>' + celda('Sin datos') + '</Row>'),
    'html_tabular': (b'<html><table><tr><td>RIT</td><td>Nombre</td></tr>'
                     b'<tr><td>X-1-2099</td><td>Persona Ficticia</td></tr></table></html>'),
    'html_activo': b'<html><form></form><table><tr><td>RIT</td></tr></table></html>',
}


def fila(rit, extra='V'):
    return [rit] + [f'{extra}{i}' for i in range(1, 23)]


ENC = list(carga.HEADERS)
RENOMBRADO = ENC[:1] + ['JUZGADO FICTICIO'] + ENC[2:]
A8 = {
    'exacto_con_repetidas': [['Informe de carga ficticio'], ENC, fila('X-1-2099'), fila('X-1-2099'), fila('X-2-2099')],
    'columna_renombrada': [RENOMBRADO, fila('X-1-2099')],
    'dos_encabezados': [ENC, ENC, fila('X-1-2099')],
    'registro_antes': [fila('X-1-2099'), ENC, fila('X-2-2099')],
    'fila_de_22': [ENC, fila('X-1-2099')[:22]],
    'titulo_concepcion': [['Concepción'] + [''] * 22, ENC, fila('X-3-2099')],
    'sin_encabezado': [fila('X-1-2099')],
}


def xlsx_carga():
    import openpyxl

    wb = openpyxl.Workbook()
    ws = wb.active
    ws.append(['Informe de carga ficticio'])
    ws.append(ENC)
    for rit in ('X-5-2099', 'X-5-2099', 'X-6-2099'):
        r = fila(rit)
        r[10] = datetime(2026, 9, 3)
        r[21] = datetime(2026, 10, 2)
        ws.append(r)
    fijo = datetime(2026, 11, 23)
    wb.properties.created = wb.properties.modified = fijo
    out = io.BytesIO()
    wb.save(out)
    # Normaliza la fecha de cada entrada del ZIP para que el fixture sea byte a byte reproducible.
    import zipfile

    entrada = zipfile.ZipFile(io.BytesIO(out.getvalue()))
    final = io.BytesIO()
    with zipfile.ZipFile(final, 'w', zipfile.ZIP_DEFLATED) as z:
        for info in entrada.infolist():
            datos = entrada.read(info.filename)
            if info.filename == 'docProps/core.xml':
                # openpyxl fija `modified` al momento de guardar; se reemplaza por la fecha fija.
                import re

                datos = re.sub(rb'(<dcterms:modified[^>]*>)[^<]*', rb'\g<1>2026-11-23T00:00:00Z', datos)
            nuevo = zipfile.ZipInfo(info.filename, date_time=(2026, 11, 23, 0, 0, 0))
            nuevo.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(nuevo, datos)
    return final.getvalue()


def celda_json(v):
    if isinstance(v, datetime):
        return v.isoformat()
    if isinstance(v, date):
        return v.isoformat()
    return '' if v is None else str(v)


def a7(datos):
    try:
        info = motor.read_excel(datos)
    except motor.PocError as e:
        return {'error': str(e)}
    return {
        'formato': info.format,
        'registros': sorted([list(k) + [n] for k, n in info.records.items()]),
        'grilla': [[celda_json(v) for v in r] for r in info.grid],
    }


def a8(grilla):
    try:
        filas = carga.records_in_grid(grilla)
    except motor.PocError as e:
        return {'error': str(e)}
    return {'filas': sorted([list(k) + [n] for k, n in filas.items()])}


def a8_libro(datos):
    try:
        info = motor.read_excel(datos, record_reader=carga.records_in_grid)
    except motor.PocError as e:
        return {'error': str(e)}
    return {'formato': info.format, 'filas': sorted([list(k) + [n] for k, n in info.records.items()])}


def main():
    salida = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'a78.json'
    libro_carga = xlsx_carga()
    fixture = {
        'generador': 'app/qa/generar_a78.py',
        'a7': [{'caso': k, 'bytes_b64': base64.b64encode(v).decode(), 'python': a7(v)} for k, v in A7.items()],
        'a8': [{'caso': k, 'grilla': g, 'python': a8(g)} for k, g in A8.items()],
        'a8_libro': [{'caso': 'xlsx_con_fechas_excel', 'bytes_b64': base64.b64encode(libro_carga).decode(),
                      'python': a8_libro(libro_carga)}],
    }
    salida.parent.mkdir(parents=True, exist_ok=True)
    salida.write_text(json.dumps(fixture, ensure_ascii=False, indent=1) + '\n', encoding='utf-8', newline='\n')
    print(f"A7 {len(fixture['a7'])} casos, A8 {len(fixture['a8'])} grillas, A8 libro 1 → {salida}")


if __name__ == '__main__':
    main()
