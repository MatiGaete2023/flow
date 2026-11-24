"""E-A4 · Oráculo de los ajustes de Flow Python: contactos con alias (`personalization.save_contact`), plantillas de
correo (`duplicate_template`, `archive_template`, `restore_template`), su gramática de variables
(`template_validation.variables`) y la importación de matrices Word (`template_package.import_templates`).

Uso (desde la raíz): python app/qa/generar_personalizacion.py [salida.json]   (por omisión app/tests/datos/personalizacion.json)
"""

import io
import json
import sys
import tempfile
import zipfile
from copy import deepcopy
from pathlib import Path

sys.stdout.reconfigure(encoding='utf-8')
RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'src')]

from unittest.mock import MagicMock  # noqa: E402

# `manual.py` importa tkinter al cargarse; el oráculo solo usa sus funciones puras.
for _m in ('tkinter', 'tkinter.ttk', 'tkinter.filedialog', 'tkinter.messagebox', 'tkinter.simpledialog'):
    sys.modules.setdefault(_m, MagicMock())

from docx import Document  # noqa: E402
from nurus.personal.config import VARIABLES, defaults  # noqa: E402
from nurus.personal.manual import block_variables, generate_word, render_block  # noqa: E402
from nurus.personal.personalization import archive_template, duplicate_template, restore_template, save_contact  # noqa: E402
from nurus.personal.template_package import import_templates  # noqa: E402
from nurus.personal.template_validation import variables  # noqa: E402


def estado(cfg):
    return {
        'contactos': dict(sorted(cfg['contactos'].items())),
        'aliases': dict(sorted(cfg['aliases'].items())),
        'tribunales': {k: '; '.join(v['para']) for k, v in sorted(cfg['correos']['tribunales'].items()) if v['para']},
    }


def base(contactos, aliases):
    cfg = defaults()
    cfg['contactos'] = dict(contactos)
    cfg['aliases'] = dict(aliases)
    return cfg


def contacto(nombre, direccion, alias=(), previo=None, contactos=None, aliases=None):
    caso = {'nombre': nombre, 'direccion': direccion, 'alias': list(alias), 'previo': previo,
            'contactos': contactos or {}, 'aliases': aliases or {}}
    try:
        caso['resultado'] = estado(save_contact(base(caso['contactos'], caso['aliases']), nombre, direccion, alias, previous=previo))
    except ValueError as e:
        caso['error'] = str(e)
    return caso


def plantilla(operacion, clave, nombre='', respaldo=None):
    cfg = defaults()
    if respaldo is not None:
        cfg['template_backups'] = {clave: respaldo}
    caso = {'operacion': operacion, 'clave': clave, 'nombre': nombre, 'respaldo': respaldo}
    campos = ('nombre', 'asunto', 'cuerpo', 'archivada', 'categoria')
    try:
        if operacion == 'duplicar':
            nuevo, destino = duplicate_template(cfg, clave, nombre)
            caso['prefijo'] = destino[:-10]
            caso['largo_sufijo'] = len(destino) - len(caso['prefijo'])
            caso['plantilla'] = {k: nuevo['correos']['plantillas'][destino].get(k) for k in campos}
        elif operacion == 'archivar':
            nuevo = archive_template(cfg, clave, True)
            caso['plantilla'] = {k: nuevo['correos']['plantillas'][clave].get(k) for k in campos}
        elif operacion == 'desarchivar':
            cfg['correos']['plantillas'][clave]['archivada'] = True
            nuevo = archive_template(cfg, clave, False)
            caso['plantilla'] = {k: nuevo['correos']['plantillas'][clave].get(k) for k in campos}
        else:
            nuevo = restore_template(cfg, clave)
            caso['plantilla'] = {k: nuevo['correos']['plantillas'][clave].get(k) for k in campos}
    except (ValueError, KeyError) as e:
        caso['error'] = str(e) if isinstance(e, ValueError) else 'KeyError'
    return caso


def gramatica(texto):
    caso = {'texto': texto}
    try:
        caso['variables'] = sorted(variables(texto, VARIABLES))
    except ValueError as e:
        caso['error'] = str(e)
    return caso


def docx_bytes():
    salida = io.BytesIO()
    Document().save(salida)
    return salida.getvalue()


def matrices(nombres, invalido=()):
    contenido = docx_bytes()
    with tempfile.TemporaryDirectory() as t:
        t = Path(t)
        zip_ruta = t / 'matrices.zip'
        with zipfile.ZipFile(zip_ruta, 'w') as z:
            for n in nombres:
                z.writestr(n, b'no es un docx' if n in invalido else contenido)
        caso = {'nombres': list(nombres), 'invalidos': list(invalido)}
        try:
            r = import_templates(zip_ruta, t / 'plantillas')
            caso['importadas'] = sorted(str(Path(p).relative_to(t / 'plantillas')).replace('\\', '/') for p in r['imported'])
            caso['sin_clasificar'] = sorted(r['unmatched'])
        except Exception as e:  # noqa: BLE001  (el rechazo y su mensaje son parte del contrato)
            caso['error'] = str(e) if isinstance(e, ValueError) else type(e).__name__
    return caso


def word_manual(titulo, cuerpo, matriz=None, valores=None):
    """Un Word manual (libre o con matriz); `matriz` es una lista de párrafos y cada párrafo una lista de runs."""
    caso = {'titulo': titulo, 'cuerpo': cuerpo, 'matriz': matriz, 'valores': valores or {}}
    with tempfile.TemporaryDirectory() as t:
        t = Path(t)
        ruta_matriz = ''
        if matriz is not None:
            doc = Document()
            for runs in matriz:
                parrafo = doc.add_paragraph()
                for texto in runs:
                    parrafo.add_run(texto)
            ruta_matriz = str(t / 'matriz.docx')
            doc.save(ruta_matriz)
        destino = t / 'salida.docx'
        try:
            generate_word(destino, titulo, cuerpo, ruta_matriz, caso['valores'])
            caso['parrafos'] = [p.text for p in Document(destino).paragraphs]
        except ValueError as e:
            caso['error'] = str(e)
            caso['creo_archivo'] = destino.exists()
    return caso


def bloque(texto, valores=None):
    caso = {'texto': texto, 'valores': valores}
    try:
        caso['variables'] = sorted(block_variables(texto))
        if valores is not None:
            caso['render'] = render_block(texto, valores)
    except ValueError as e:
        caso['error'] = str(e)
    return caso


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'personalizacion.json'
    c = lambda **kw: kw  # noqa: E731
    contactos = [
        contacto('Programa Uno', 'uno@ejemplo.cl'),
        contacto('Programa Uno', 'uno@ejemplo.cl; dos@ejemplo.cl, UNO@ejemplo.cl\ntres@ejemplo.cl'),
        contacto('', 'uno@ejemplo.cl'),
        contacto('Programa Uno', 'no es un correo'),
        contacto('Tribunal: LAJA', 'laja@ejemplo.cl; laja2@ejemplo.cl'),
        contacto('Tribunal: INEXISTENTE', 'x@ejemplo.cl'),
        contacto('Programa Nuevo', 'x@ejemplo.cl', previo='Tribunal: LAJA'),
        contacto('Programa Dos', 'dos@ejemplo.cl', alias=['PD', 'Prog 2', 'PD', ' ']),
        contacto('Programa Uno', 'otro@ejemplo.cl', contactos={'Programa Uno': 'uno@ejemplo.cl'}),
        contacto('programa uno', 'otro@ejemplo.cl', contactos={'Programa Uno': 'uno@ejemplo.cl'}),
        contacto('Programa Uno Renombrado', 'uno@ejemplo.cl', previo='Programa Uno',
                 contactos={'Programa Uno': 'uno@ejemplo.cl'}, aliases={'PU': 'Programa Uno'}),
        contacto('PU', 'x@ejemplo.cl', aliases={'PU': 'Programa Uno'}, contactos={'Programa Uno': 'uno@ejemplo.cl'}),
        contacto('Programa Tres', 'tres@ejemplo.cl', alias=['Programa Uno'], contactos={'Programa Uno': 'uno@ejemplo.cl'}),
        contacto('Programa Tres', 'tres@ejemplo.cl', alias=['PU'], contactos={'Programa Uno': 'uno@ejemplo.cl'},
                 aliases={'PU': 'Programa Uno'}),
        contacto('Programa Uno', 'uno@ejemplo.cl', alias=['PU', 'P1'], previo='Programa Uno',
                 contactos={'Programa Uno': 'uno@ejemplo.cl'}, aliases={'PU': 'Programa Uno', 'Viejo': 'Programa Uno'}),
    ]
    plantillas = [
        plantilla('duplicar', 'programa_espera', 'Mi plantilla'),
        plantilla('duplicar', 'programa_espera', '  '),
        plantilla('duplicar', 'espera', 'Otra'),
        plantilla('duplicar', 'inexistente', 'X'),
        plantilla('archivar', 'medidas'),
        plantilla('desarchivar', 'medidas'),
        plantilla('restaurar', 'medidas'),
        plantilla('restaurar', 'inexistente'),
        plantilla('restaurar', 'medidas', respaldo={'nombre': 'Anterior', 'asunto': 'Asunto anterior {TRIBUNAL}',
                                                    'cuerpo': 'Cuerpo anterior {FECHA}', 'adjunto': 'opcional',
                                                    'usa_modalidades': True, 'archivada': True}),
    ]
    gram = [gramatica(t) for t in [
        'Hola {TRIBUNAL}, {FECHA}', 'Sin variables', '{{TRIBUNAL}} literal y {PROGRAMA}', '{TRIBUNAL', 'TRIBUNAL}',
        '{TRIBUNAL!r}', '{TRIBUNAL:>10}', '{}', '{DESCONOCIDA}', '{TRIBUNAL} {TRIBUNAL}', 'a { b', '{{', '}}', '{TABLA_REGISTROS}',
    ]]
    mat = [
        matrices(['LAJA_PC_IE.docx', 'MULCHEN PC INFO.docx', 'tomé/NOMENCLATURA TOME.docx']),
        matrices(['Plantillas/laja/PC_IE.docx', 'Mulchén_pc_info.docx', 'Sin sentido.docx', 'leeme.txt']),
        matrices(['LAJA_PC_IE.docx', 'LAJA PC IE copia.docx']),
        matrices(['LAJA_PC_IE.docx'], invalido=['LAJA_PC_IE.docx']),
        matrices(['LAJA_MULCHEN_PC_IE.docx']),
        matrices(['TOME_NOMENCL.docx', 'TOME_PC_IE.docx']),
    ]
    palabras = [
        word_manual('Título', 'Párrafo uno\nPárrafo dos'),
        word_manual('', 'Solo cuerpo'),
        word_manual('Título', '  \n '),
        word_manual('Título', 'Hola {{NOMBRE}}'),
        word_manual('Título', 'Texto [COMPLETAR dato]'),
        word_manual('', '', [['{{NOMBRE}}']], {}),
        word_manual('', '', [['{{NOMBRE}}']], {'NOMBRE': 'Persona'}),
        word_manual('', '', [['Estimado {{NO', 'MBRE}}, rit ', '{{RIT}}']], {'NOMBRE': 'Ana', 'RIT': 'X-1-2099'}),
        word_manual('', '', [['{{RIT}} {{NOMBRE}}']], {'RIT': 'A'}),
        word_manual('', '', [['{{NOMBRE}}']], {'NOMBRE': '  '}),
        word_manual('', '', [['Hola {{NOMBRE}}'], ['Sin campos'], ['Adiós {{NOMBRE}}']], {'NOMBRE': 'Ana'}),
    ]
    bloques = [
        bloque('Hola {NOMBRE}, causa {RIT}', {'NOMBRE': 'Ana', 'RIT': 'X-1-2099'}),
        bloque('Hola {NOMBRE}', {'NOMBRE': ' '}),
        bloque('Hola {NOMBRE}', {}),
        bloque('Sin variables', {}),
        bloque('{DESCONOCIDA}'),
        bloque('{NOMBRE!r}'),
        bloque('{{NOMBRE}} y {FECHA}', {'FECHA': '01/01/2026'}),
        bloque('{NOMBRE'),
    ]
    _ = (c, deepcopy)
    destino.write_bytes((json.dumps({'contactos': contactos, 'plantillas': plantillas, 'gramatica': gram, 'matrices': mat, 'palabras': palabras, 'bloques': bloques},
                                    ensure_ascii=False, indent=1) + '\n').encode('utf-8'))
    print(f'{len(contactos)} contactos, {len(plantillas)} plantillas, {len(gram)} textos, {len(mat)} paquetes, {len(palabras)} Word y {len(bloques)} bloques en {destino}')


if __name__ == '__main__':
    main()
