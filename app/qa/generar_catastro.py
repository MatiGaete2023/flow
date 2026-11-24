"""E-A6 · Oráculo de la importación de contactos de Flow Python: `outputs.import_contacts` (catastro Excel con columnas
Nombre/Programa y Mail/Correo) y `config.parse_contacts` (lista JSON de {Nombre, Mail} o el formato histórico).

Cada caso de catastro lleva el `.xlsx` armado con openpyxl (en base64), los contactos previos y lo que entrega Python:
los contactos resultantes y los conflictos, o el mensaje del error. Los de `parse_contacts` llevan el texto y el diccionario.

Uso (desde la raíz): python app/qa/generar_catastro.py [salida.json]   (por omisión app/tests/datos/catastro.json)
"""

import base64
import io
import json
import re
import sys
import tempfile
import zipfile
from datetime import datetime
from pathlib import Path

sys.stdout.reconfigure(encoding='utf-8')
RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'src')]

from openpyxl import Workbook  # noqa: E402
from nurus.personal.config import parse_contacts  # noqa: E402
from nurus.personal.outputs import import_contacts  # noqa: E402


def xlsx_fijo(libro):
    """Bytes del libro sin hora de creación ni fechas de ZIP (sin compresión): el oráculo se regenera igual en cada corrida."""
    libro.properties.created = libro.properties.modified = datetime(2026, 1, 1)
    origen = io.BytesIO()
    libro.save(origen)
    salida = io.BytesIO()
    with zipfile.ZipFile(origen) as entrada, zipfile.ZipFile(salida, 'w', zipfile.ZIP_STORED) as destino:
        for nombre in sorted(entrada.namelist()):
            info = zipfile.ZipInfo(nombre, (1980, 1, 1, 0, 0, 0))
            info.create_system = 3  # por omisión es 0 en Windows y 3 en Linux: cambiaría la cabecera
            info.external_attr = 0o600 << 16
            datos = entrada.read(nombre)
            if nombre == 'docProps/core.xml':  # `save` pone la hora actual en «modificado»
                datos = re.sub(rb'(<dcterms:modified[^>]*>)[^<]*', rb'\g<1>2026-01-01T00:00:00Z', datos)
            destino.writestr(info, datos)
    return salida.getvalue()


def catastro(filas, contactos=None, otra_hoja=None):
    libro = Workbook()
    hoja = libro.active
    for fila in filas:
        hoja.append(fila)
    if otra_hoja:
        for fila in otra_hoja:
            libro.create_sheet('Otra').append(fila)
    contenido = xlsx_fijo(libro)
    previos = dict(contactos or {})
    caso = {'contactos': previos, 'filas': filas, 'xlsx': base64.b64encode(contenido).decode()}
    with tempfile.TemporaryDirectory() as carpeta:
        ruta = Path(carpeta) / 'catastro.xlsx'
        ruta.write_bytes(contenido)
        try:
            candidatos, conflictos = import_contacts({'contactos': dict(previos)}, str(ruta))
            caso['resultado'] = {'contactos': dict(sorted(candidatos.items())), 'conflictos': conflictos}
        except ValueError as error:
            caso['error'] = str(error)
    return caso


def texto(raw):
    return {'texto': raw, 'contactos': dict(sorted(parse_contacts(raw).items()))}


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'catastro.json'
    previos = {'Programa Uno': 'uno@ejemplo.cl', 'Programa Dos': 'dos@ejemplo.cl; otro@ejemplo.cl'}
    catastros = [
        catastro([['Nombre', 'Mail'], ['Programa Tres', 'tres@ejemplo.cl'], ['Programa Cuatro', 'cuatro@ejemplo.cl']]),
        catastro([['Programa', 'Correo electrónico'], ['Programa Tres', 'tres@ejemplo.cl']], previos),
        catastro([['Nombre Programa', 'EMAIL'], ['Programa Tres', 'tres@ejemplo.cl']], previos),
        catastro([[' NOMBRE ', ' Correo '], ['Programa Tres', 'tres@ejemplo.cl']]),
        catastro([['Nombre', 'e-mail'], ['Programa Tres', 'tres@ejemplo.cl']]),
        catastro([['Nombre', 'Teléfono'], ['Programa Tres', '123']]),
        catastro([['Mail'], ['tres@ejemplo.cl']]),
        catastro([['Nombre', 'Mail'], ['Programa Uno', 'distinto@ejemplo.cl'], ['Programa Tres', 'tres@ejemplo.cl']], previos),
        catastro([['Nombre', 'Mail'], ['programa uno', 'distinto@ejemplo.cl']], previos),
        catastro([['Nombre', 'Mail'], ['programa uno', 'uno@ejemplo.cl']], previos),
        catastro([['Nombre', 'Mail'], ['Programa Uno', 'uno@ejemplo.cl']], previos),
        catastro([['Nombre', 'Mail'], ['Programa Dos', 'otro@ejemplo.cl; dos@ejemplo.cl']], previos),
        catastro([['Nombre', 'Mail'], ['Programa Tres', 'a@ejemplo.cl'], ['Programa Tres', 'b@ejemplo.cl']]),
        catastro([['Nombre', 'Mail'], ['Programa Tres', 'a@ejemplo.cl'], ['Programa Tres', 'a@ejemplo.cl']]),
        catastro([['Nombre', 'Mail'], ['Programa Tres', 'no es un correo']]),
        catastro([['Nombre', 'Mail'], ['Programa Tres', 'a@ejemplo.cl'], ['Programa Cuatro', 'x@ejemplo.cl; malo']]),
        catastro([['Nombre', 'Mail'], ['Programa Tres', 'a@ejemplo.cl, B@ejemplo.cl\nb@ejemplo.cl;c@ejemplo.cl']]),
        catastro([['Nombre', 'Mail'], [None, 'a@ejemplo.cl'], ['  ', 'b@ejemplo.cl'], ['Programa Tres', 'c@ejemplo.cl']]),
        catastro([['Nombre', 'Mail'], ['Programa Tres', None], ['Programa Cuatro', '']]),
        catastro([['Nombre', 'Mail'], ['  Programa Tres  ', ' tres@ejemplo.cl ']]),
        catastro([['Nombre', 'Mail'], ['Programa Tres', 'a@ejemplo.cl']], otra_hoja=[['Nombre', 'Mail'], ['Hoja Dos', 'b@ejemplo.cl']]),
        catastro([['Nombre', 'Mail']]),
        catastro([['Nombre', 'Mail'], ['Tribunal: LAJA', 'laja@ejemplo.cl']]),
        catastro([['Nombre', 'Mail', 'Nombre'], ['Programa Tres', 'a@ejemplo.cl', 'Otro']]),
    ]
    textos = [
        texto('[{"Nombre": "Programa Uno", "Mail": "uno@ejemplo.cl"}, {"Nombre": "Programa Dos", "Mail": "dos@ejemplo.cl; x@ejemplo.cl"}]'),
        texto('{"Nombre": "Programa Uno", "Mail": "uno@ejemplo.cl"},\n{"Nombre": "Programa Dos", "Mail": "dos@ejemplo.cl"},'),
        texto('[{"Nombre": "Programa Uno", "Mail": "a@ejemplo.cl"}, {"Nombre": "Programa Uno", "Mail": "b@ejemplo.cl"}, '
              '{"Nombre": "Programa Dos", "Mail": "dos@ejemplo.cl"}]'),
        texto('[{"Nombre": "Programa Uno", "Mail": "a@ejemplo.cl"}, {"Nombre": "Programa Uno", "Mail": "a@ejemplo.cl"}]'),
        texto('[{"Nombre": "Programa Uno", "Mail": "malo"}, {"Nombre": "Programa Dos", "Mail": "dos@ejemplo.cl"}]'),
        texto('[{"Nombre": "", "Mail": "a@ejemplo.cl"}, {"Mail": "b@ejemplo.cl"}, {"Nombre": "Programa Dos", "Mail": "dos@ejemplo.cl"}]'),
        texto('[{"Nombre": "  Programa Uno ", "Mail": " uno@ejemplo.cl "}]'),
        texto('[{"Nombre": "Tribunal: LAJA", "Mail": "laja@ejemplo.cl"}]'),
        texto('[{"Nombre": "Programa Uno"}]'),
        texto('[{"Nombre": "Programa Uno", "Mail": ""}]'),
        texto('[]'),
        texto('  [{"Nombre": "Programa Uno", "Mail": "a@ejemplo.cl"}, {"Nombre": "Programa Uno", "Mail": "b@ejemplo.cl"}, '
              '{"Nombre": "Programa Uno", "Mail": "a@ejemplo.cl"}]'),
    ]
    destino.write_bytes((json.dumps({'catastros': catastros, 'textos': textos}, ensure_ascii=False, indent=1) + '\n').encode('utf-8'))
    print(f'{len(catastros)} catastros y {len(textos)} textos en {destino}')


if __name__ == '__main__':
    main()
