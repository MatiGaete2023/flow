"""E-P1 · Oráculo de proyectos Word por tribunal (`resolutions.prepare_projects` y `generate_projects`):
arma casos ficticios con Laja y Mulchén, genera con Python el `.docx` de cada tribunal
y graba la secuencia de párrafos (`texto`, `salto`) y si quedan cursivas activas.

Uso (desde la raíz): python app/qa/generar_word_tribunal.py [salida.json]
(por omisión app/tests/datos/word_tribunal.json)
"""

import base64
import hashlib
import io
import json
import posixpath
import struct
import sys
import tempfile
import xml.etree.ElementTree as ET
import zipfile
import zlib
from datetime import date
from pathlib import Path
from unittest.mock import MagicMock

sys.stdout.reconfigure(encoding='utf-8')
RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'src')]
for _m in ('tkinter', 'tkinter.ttk', 'tkinter.filedialog', 'tkinter.messagebox', 'tkinter.simpledialog'):
    sys.modules.setdefault(_m, MagicMock())

from openpyxl import Workbook  # noqa: E402
from nurus.personal.config import defaults  # noqa: E402
import nurus.personal.outputs as outputs  # noqa: E402
from nurus.personal.resolutions import kind_label, preparacion_desde_interfaz, generate_projects  # noqa: E402
from nurus.personal.work import Work  # noqa: E402

CORTE = date(2026, 11, 20)
HOY = date(2099, 1, 1)
W_NS = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'
R_NS = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships'
CT_NS = 'http://schemas.openxmlformats.org/package/2006/content-types'
REL_NS = 'http://schemas.openxmlformats.org/package/2006/relationships'


def _png_sintetico(r: int, g: int, b: int) -> bytes:
    """Genera un PNG RGBA 2x2 mínimo y determinista en memoria (sin PIL ni fechas)."""
    def chunk(tag: bytes, data: bytes) -> bytes:
        crc = zlib.crc32(tag + data) & 0xFFFFFFFF
        return struct.pack('>I', len(data)) + tag + data + struct.pack('>I', crc)

    fila = bytes([0, r, g, b, r, g, b])
    raw = fila * 2
    ihdr = struct.pack('>IIBBBBB', 2, 2, 8, 2, 0, 0, 0)
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', ihdr) + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b'')


PNG_AZUL = _png_sintetico(12, 68, 130)
PNG_ROJO = _png_sintetico(175, 42, 42)


class _FijaMeta(type(date)):
    def __instancecheck__(cls, obj):
        return isinstance(obj, date)


class _Fija(date, metaclass=_FijaMeta):
    @classmethod
    def today(cls):
        return HOY


outputs.date = _Fija
PLANTILLAS = RAIZ / 'src' / 'nurus' / 'personal' / 'plantillas_word'


def _crear_docx_propio(
    *,
    con_cursivas: bool,
    etiqueta: str,
    imagen_cuerpo: bytes | None = None,
    imagen_encabezado: bytes | None = None,
    numeracion_fmt: str | None = None,
) -> str:
    """Construye un .docx determinista en memoria (ZipInfo.create_system = 3) para probar plantillas propias."""
    estilo_italic = '<w:i/>' if con_cursivas else ''
    rpr_italic = '<w:i/><w:i w:val="1"/><w:iCs/>' if con_cursivas else ''
    partes: list[tuple[str, bytes]] = []
    extra_rels: list[str] = []
    extra_ct: list[str] = []
    extra_body = ''
    header_ref = ''
    extra_styles = ''

    def _bloque_drawing(rid_attr: str, rid_val: str, nombre: str) -> str:
        return (
            '<w:p><w:r><w:drawing>'
            '<wp:inline xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">'
            '<wp:extent cx="9525" cy="9525"/><wp:docPr id="1" name="' + nombre + '"/>'
            '<a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">'
            '<a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">'
            '<pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">'
            '<pic:nvPicPr><pic:cNvPr id="1" name="' + nombre + '"/><pic:cNvPicPr/></pic:nvPicPr>'
            f'<pic:blipFill><a:blip {rid_attr}="{rid_val}"/></pic:blipFill>'
            '<pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="9525" cy="9525"/></a:xfrm>'
            '<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr>'
            '</pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>'
        )

    if imagen_cuerpo is not None:
        extra_body += _bloque_drawing('r:embed', 'rId2', 'LogoCuerpo')
        extra_rels.append(
            '<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/>'
        )
        extra_ct.append('<Default Extension="png" ContentType="image/png"/>')
        partes.append(('word/media/image1.png', imagen_cuerpo))

    if numeracion_fmt is not None:
        extra_body += (
            '<w:p><w:pPr><w:pStyle w:val="ListaPropia"/>'
            '<w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr>'
            '<w:r><w:t xml:space="preserve">Primer punto {{RIT}}</w:t></w:r></w:p>'
            '<w:p><w:pPr><w:pStyle w:val="ListaPropia"/>'
            '<w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr>'
            '<w:r><w:t xml:space="preserve">Segundo punto {{PROGRAMA}}</w:t></w:r></w:p>'
        )
        extra_styles += '<w:style w:type="paragraph" w:styleId="ListaPropia"><w:name w:val="ListaPropia"/></w:style>'
        extra_rels.append(
            '<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/>'
        )
        extra_ct.append(
            '<Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/>'
        )
        lvl_txt = '•' if numeracion_fmt == 'bullet' else '%1.'
        num_xml = (
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
            '<w:abstractNum w:abstractNumId="0">'
            '<w:nsid w:val="11223344"/><w:multiLevelType w:val="hybridMultilevel"/>'
            f'<w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="{numeracion_fmt}"/><w:lvlText w:val="{lvl_txt}"/></w:lvl>'
            '</w:abstractNum>'
            '<w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num>'
            '</w:numbering>'
        )
        partes.append(('word/numbering.xml', num_xml.encode('utf-8')))

    if imagen_encabezado is not None:
        header_ref = '<w:headerReference w:type="default" r:id="rId4"/>'
        extra_rels.append(
            '<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/>'
        )
        extra_ct.append(
            '<Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/>'
        )
        if '<Default Extension="png"' not in ''.join(extra_ct):
            extra_ct.append('<Default Extension="png" ContentType="image/png"/>')
        hdr_xml = (
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" '
            'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
            + _bloque_drawing('r:embed', 'rId1', 'LogoEncabezado')
            + f'<w:p><w:r><w:rPr>{rpr_italic}</w:rPr><w:t xml:space="preserve">Encabezado oficial</w:t></w:r></w:p>'
            '</w:hdr>'
        )
        hdr_rels = (
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/>'
            '</Relationships>'
        )
        partes.append(('word/header1.xml', hdr_xml.encode('utf-8')))
        partes.append(('word/_rels/header1.xml.rels', hdr_rels.encode('utf-8')))
        if not any(n == 'word/media/image1.png' for n, _ in partes):
            partes.append(('word/media/image1.png', imagen_encabezado))

    doc_xml = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" '
        'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
        '<w:body>'
        f'<w:p><w:pPr><w:pStyle w:val="EstiloPropio"/></w:pPr><w:r><w:rPr>{rpr_italic}</w:rPr>'
        f'<w:t xml:space="preserve">{etiqueta} · {{{{FECHA}}}}</w:t></w:r></w:p>'
        f'<w:p><w:r><w:rPr>{rpr_italic}</w:rPr>'
        '<w:t xml:space="preserve">Causa {{RIT}} de {{NOMBRE}}, programa {{PROGRAMA}}.</w:t></w:r></w:p>'
        + extra_body
        + f'<w:sectPr>{header_ref}<w:pgSz w:w="12240" w:h="15840"/>'
        '<w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr>'
        '</w:body></w:document>'
    )
    styles_xml = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        '<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>'
        f'<w:style w:type="paragraph" w:styleId="EstiloPropio"><w:name w:val="EstiloPropio"/><w:rPr>{estilo_italic}</w:rPr></w:style>'
        + extra_styles
        + '</w:styles>'
    )
    ct_xml = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
        '<Default Extension="xml" ContentType="application/xml"/>'
        + ''.join(extra_ct)
        + '<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>'
        '<Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>'
        '</Types>'
    )
    rels_xml = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>'
        '</Relationships>'
    )
    doc_rels_xml = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>'
        + ''.join(extra_rels)
        + '</Relationships>'
    )
    partes_ordenadas = [
        ('[Content_Types].xml', ct_xml.encode('utf-8')),
        ('_rels/.rels', rels_xml.encode('utf-8')),
        ('word/document.xml', doc_xml.encode('utf-8')),
        ('word/_rels/document.xml.rels', doc_rels_xml.encode('utf-8')),
        ('word/styles.xml', styles_xml.encode('utf-8')),
    ] + partes
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, 'w', compression=zipfile.ZIP_STORED) as z:
        for nombre, cuerpo in partes_ordenadas:
            zi = zipfile.ZipInfo(nombre, date_time=(2026, 11, 20, 0, 0, 0))
            zi.create_system = 3
            zi.external_attr = 0o644 << 16
            zi.compress_type = zipfile.ZIP_STORED
            z.writestr(zi, cuerpo)
    return base64.b64encode(buf.getvalue()).decode('ascii')


PLANTILLA_PROPIA_LAJA = _crear_docx_propio(con_cursivas=False, etiqueta='Matriz propia Laja')
PLANTILLA_CURSIVAS_MULCHEN = _crear_docx_propio(con_cursivas=True, etiqueta='Matriz cursivas Mulchén')
PLANTILLA_IMAGEN_LAJA = _crear_docx_propio(con_cursivas=False, etiqueta='Matriz imagen Laja', imagen_cuerpo=PNG_AZUL)
PLANTILLA_IMAGEN_LAJA_DOS = _crear_docx_propio(con_cursivas=False, etiqueta='Matriz imagen Laja II', imagen_cuerpo=PNG_AZUL)
PLANTILLA_VINETAS_LAJA_A = _crear_docx_propio(con_cursivas=False, etiqueta='Matriz vinetas A', numeracion_fmt='bullet')
PLANTILLA_VINETAS_LAJA_B = _crear_docx_propio(con_cursivas=False, etiqueta='Matriz vinetas B', numeracion_fmt='bullet')
PLANTILLA_NUMERADA_MULCHEN_A = _crear_docx_propio(con_cursivas=False, etiqueta='Matriz numerada A', numeracion_fmt='decimal')
PLANTILLA_NUMERADA_MULCHEN_B = _crear_docx_propio(con_cursivas=False, etiqueta='Matriz numerada B', numeracion_fmt='decimal')
PLANTILLA_ENCABEZADO_IMG_MULCHEN = _crear_docx_propio(
    con_cursivas=True, etiqueta='Matriz encabezado img Mulchen', imagen_encabezado=PNG_ROJO
)

CASOS = [
    {
        'nombre': 'un_proyecto',
        'modo': 'ESPERA',
        'encabezados': ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'T ESPERA', 'FEC. RESOLUCION', 'DURACION'],
        'filas': [
            ['X-10-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT Ficticio', 90, '15/10/2026', '6 meses'],
        ],
    },
    {
        'nombre': 'tres_proyectos_un_tribunal',
        'modo': 'ESPERA',
        'encabezados': ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'T ESPERA', 'FEC. RESOLUCION', 'DURACION'],
        'filas': [
            ['X-10-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT Ficticio', 90, '15/10/2026', '6 meses'],
            ['X-11-2026', 'Jgdo. L. y G. de Laja', 'Persona Dos', '22222222-2', 'AFT Ficticio', 95, '20/10/2026', '12 meses'],
            ['X-11-2026', 'Jgdo. L. y G. de Laja', 'Persona Tres', '33333333-3', 'AFT Ficticio', 95, '20/10/2026', '12 meses'],
            ['X-12-2026', 'Jgdo. L. y G. de Laja', 'Persona Cuatro', '44444444-4', 'AFT Ficticio', 100, '12/09/2026', '6 meses'],
            ['X-12-2026', 'Jgdo. L. y G. de Laja', 'Persona Cinco', '55555555-5', 'PRM Ficticio', 100, '12/09/2026', '6 meses'],
        ],
    },
    {
        'nombre': 'dos_tribunales',
        'modo': 'ESPERA',
        'encabezados': ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'T ESPERA', 'FEC. RESOLUCION', 'DURACION'],
        'filas': [
            ['X-50-2026', 'Jgdo. L. y G. de Laja', 'Persona Alfa', '13131313-1', 'AFT Ficticio', 90, '10/10/2026', '6 meses'],
            ['X-51-2026', 'Jgdo. L. y G. de Laja', 'Persona Beta', '14141414-2', 'AFT Ficticio', 10, '11/10/2026', '6 meses'],
            ['X-60-2026', 'Jgdo. L. y G. de Mulchen', 'Persona Gamma', '15151515-3', 'AFT Ficticio', 95, '12/10/2026', '6 meses'],
            ['X-61-2026', 'Jgdo. L. y G. de Mulchen', 'Persona Delta', '16161616-4', 'AFT Ficticio', 12, '13/10/2026', '6 meses'],
        ],
        'res': ['PC_IE', 'NOMENCL', 'PC_IE', 'NOMENCL'],
    },
    {
        'nombre': 'texto_editado_en_revision',
        'modo': 'ESPERA',
        'encabezados': ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'T ESPERA', 'FEC. RESOLUCION', 'DURACION'],
        'filas': [
            ['X-70-2026', 'Jgdo. L. y G. de Mulchen', 'Persona Editada', '17171717-5', 'AFT Ficticio', 15, '01/10/2026', '6 meses'],
        ],
        'res': ['NOMENCL'],
        'editadas': ['Aplicar nomenclatura «Ingreso informe de diagnóstico».'],
    },
    {
        'nombre': 'matriz_propia_del_usuario',
        'modo': 'ESPERA',
        'encabezados': ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'T ESPERA', 'FEC. RESOLUCION', 'DURACION'],
        'filas': [
            ['X-80-2026', 'Jgdo. L. y G. de Laja', 'Persona Propia', '18181818-6', 'AFT Ficticio', 90, '02/10/2026', '6 meses'],
            ['X-81-2026', 'Jgdo. L. y G. de Laja', 'Persona Estandar', '19191919-7', 'AFT Ficticio', 10, '03/10/2026', '6 meses'],
        ],
        'res': ['PC_IE', 'NOMENCL'],
        'plantillas_propias': {'LAJA/PC_IE.docx': PLANTILLA_PROPIA_LAJA},
    },
    {
        'nombre': 'matriz_con_cursivas',
        'modo': 'ESPERA',
        'encabezados': ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'T ESPERA', 'FEC. RESOLUCION', 'DURACION'],
        'filas': [
            ['X-90-2026', 'Jgdo. L. y G. de Mulchen', 'Persona Cursiva', '20202020-8', 'AFT Ficticio', 95, '04/10/2026', '6 meses'],
        ],
        'plantillas_propias': {'MULCHEN/PC_IE.docx': PLANTILLA_CURSIVAS_MULCHEN},
    },
    {
        'nombre': 'plantilla_propia_con_imagen',
        'modo': 'ESPERA',
        'encabezados': ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'T ESPERA', 'FEC. RESOLUCION', 'DURACION'],
        'filas': [
            ['X-91-2026', 'Jgdo. L. y G. de Laja', 'Persona Imagen Uno', '21212121-1', 'AFT Ficticio', 90, '05/10/2026', '6 meses'],
        ],
        'plantillas_propias': {'LAJA/PC_IE.docx': PLANTILLA_IMAGEN_LAJA},
    },
    {
        'nombre': 'dos_proyectos_misma_imagen',
        'modo': 'ESPERA',
        'encabezados': ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'T ESPERA', 'FEC. RESOLUCION', 'DURACION'],
        'filas': [
            ['X-92-2026', 'Jgdo. L. y G. de Laja', 'Persona Img A', '22222222-4', 'AFT Ficticio', 90, '06/10/2026', '6 meses'],
            ['X-93-2026', 'Jgdo. L. y G. de Laja', 'Persona Img B', '23232323-5', 'AFT Ficticio', 10, '07/10/2026', '6 meses'],
        ],
        'res': ['PC_IE', 'NOMENCL'],
        'plantillas_propias': {
            'LAJA/PC_IE.docx': PLANTILLA_IMAGEN_LAJA,
            'LAJA/NOMENCL.docx': PLANTILLA_IMAGEN_LAJA_DOS,
        },
    },
    {
        'nombre': 'institucional_mas_propia_con_imagen',
        'modo': 'ESPERA',
        'encabezados': ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'T ESPERA', 'FEC. RESOLUCION', 'DURACION'],
        'filas': [
            ['X-94-2026', 'Jgdo. L. y G. de Laja', 'Persona Base', '24242424-6', 'AFT Ficticio', 10, '08/10/2026', '6 meses'],
            ['X-95-2026', 'Jgdo. L. y G. de Laja', 'Persona Img Extra', '25252525-7', 'AFT Ficticio', 90, '09/10/2026', '6 meses'],
        ],
        'res': ['NOMENCL', 'PC_IE'],
        'plantillas_propias': {'LAJA/PC_IE.docx': PLANTILLA_IMAGEN_LAJA},
    },
    {
        'nombre': 'plantilla_lista_con_vinetas',
        'modo': 'ESPERA',
        'encabezados': ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'T ESPERA', 'FEC. RESOLUCION', 'DURACION'],
        'filas': [
            ['X-96-2026', 'Jgdo. L. y G. de Laja', 'Persona Vineta A', '26262626-8', 'AFT Ficticio', 90, '10/10/2026', '6 meses'],
            ['X-97-2026', 'Jgdo. L. y G. de Laja', 'Persona Vineta B', '27272727-9', 'AFT Ficticio', 10, '11/10/2026', '6 meses'],
        ],
        'res': ['PC_IE', 'NOMENCL'],
        'plantillas_propias': {
            'LAJA/PC_IE.docx': PLANTILLA_VINETAS_LAJA_A,
            'LAJA/NOMENCL.docx': PLANTILLA_VINETAS_LAJA_B,
        },
    },
    {
        'nombre': 'plantilla_lista_numerada_seguidos',
        'modo': 'ESPERA',
        'encabezados': ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'T ESPERA', 'FEC. RESOLUCION', 'DURACION'],
        'filas': [
            ['X-98-2026', 'Jgdo. L. y G. de Mulchen', 'Persona Num A', '28282828-0', 'AFT Ficticio', 95, '12/10/2026', '6 meses'],
            ['X-99-2026', 'Jgdo. L. y G. de Mulchen', 'Persona Num B', '29292929-1', 'AFT Ficticio', 10, '13/10/2026', '6 meses'],
        ],
        'res': ['PC_IE', 'NOMENCL'],
        'plantillas_propias': {
            'MULCHEN/PC_IE.docx': PLANTILLA_NUMERADA_MULCHEN_A,
            'MULCHEN/NOMENCL.docx': PLANTILLA_NUMERADA_MULCHEN_B,
        },
    },
    {
        'nombre': 'encabezado_con_imagen',
        'modo': 'ESPERA',
        'encabezados': ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'T ESPERA', 'FEC. RESOLUCION', 'DURACION'],
        'filas': [
            ['X-100-2026', 'Jgdo. L. y G. de Mulchen', 'Persona Hdr A', '30303030-2', 'AFT Ficticio', 95, '14/10/2026', '6 meses'],
            ['X-101-2026', 'Jgdo. L. y G. de Mulchen', 'Persona Hdr B', '31313131-3', 'AFT Ficticio', 10, '15/10/2026', '6 meses'],
        ],
        'res': ['PC_IE', 'NOMENCL'],
        'plantillas_propias': {'MULCHEN/PC_IE.docx': PLANTILLA_ENCABEZADO_IMG_MULCHEN},
    },
]


def crear_excel(ruta: Path, modo: str, encabezados: list, filas: list):
    wb = Workbook()
    ws = wb.active
    ws.title = modo
    for c, h in enumerate(encabezados, 1):
        ws.cell(3, c, h)
    for r_idx, fila in enumerate(filas, 4):
        for c_idx, val in enumerate(fila, 1):
            ws.cell(r_idx, c_idx, val)
    wb.save(ruta)


def _tiene_cursivas_activas(xml_bytes: bytes) -> bool:
    root = ET.fromstring(xml_bytes)
    for tag in (f'{{{W_NS}}}i', f'{{{W_NS}}}iCs'):
        for elem in root.iter(tag):
            val = elem.get(f'{{{W_NS}}}val')
            if val is None or val.lower() in ('1', 'true', 'on'):
                return True
    return False


def _resolver_rel_destino(parte_origen: str, target: str) -> str:
    if target.startswith('/'):
        return posixpath.normpath(target.lstrip('/'))
    base_dir = posixpath.dirname(parte_origen)
    return posixpath.normpath(posixpath.join(base_dir, target))


def _rels_de(z: zipfile.ZipFile, parte: str) -> dict[str, tuple[str, str, bool]]:
    base_dir, nombre = posixpath.split(parte)
    rels_path = f'{base_dir}/_rels/{nombre}.rels' if base_dir else f'_rels/{nombre}.rels'
    if rels_path not in z.namelist():
        return {}
    root = ET.fromstring(z.read(rels_path))
    out = {}
    for rel in root.findall(f'{{{REL_NS}}}Relationship'):
        rid = rel.get('Id')
        tipo = rel.get('Type', '')
        target = rel.get('Target', '')
        externa = rel.get('TargetMode', '').lower() == 'external'
        if rid:
            out[rid] = (tipo, target, externa)
    return out


def _inspeccionar_numeracion(z: zipfile.ZipFile):
    if 'word/numbering.xml' not in z.namelist():
        return {}, {}
    nroot = ET.fromstring(z.read('word/numbering.xml'))
    anums = {}
    for an in nroot.findall(f'{{{W_NS}}}abstractNum'):
        aid = an.get(f'{{{W_NS}}}abstractNumId')
        lvls = {}
        for lvl in an.findall(f'{{{W_NS}}}lvl'):
            ilvl = lvl.get(f'{{{W_NS}}}ilvl')
            fmt_el = lvl.find(f'{{{W_NS}}}numFmt')
            if ilvl is not None and fmt_el is not None:
                lvls[ilvl] = fmt_el.get(f'{{{W_NS}}}val')
        if aid is not None:
            anums[aid] = lvls
    nums = {}
    for nm in nroot.findall(f'{{{W_NS}}}num'):
        nid = nm.get(f'{{{W_NS}}}numId')
        aid_el = nm.find(f'{{{W_NS}}}abstractNumId')
        aid = aid_el.get(f'{{{W_NS}}}val') if aid_el is not None else None
        overrides = {}
        for ov in nm.findall(f'{{{W_NS}}}lvlOverride'):
            ilvl = ov.get(f'{{{W_NS}}}ilvl')
            st = ov.find(f'{{{W_NS}}}startOverride')
            if ilvl is not None and st is not None:
                overrides[ilvl] = st.get(f'{{{W_NS}}}val')
        if nid is not None and aid is not None:
            nums[nid] = (aid, overrides)
    return nums, anums


def _inspeccionar_content_types(z: zipfile.ZipFile):
    ct_root = ET.fromstring(z.read('[Content_Types].xml'))
    overrides = {}
    defaults = {}
    for ov in ct_root.findall(f'{{{CT_NS}}}Override'):
        pn = ov.get('PartName', '')
        ct = ov.get('ContentType', '')
        if pn:
            overrides[pn] = ct
    for df in ct_root.findall(f'{{{CT_NS}}}Default'):
        ext = df.get('Extension', '').lower()
        ct = df.get('ContentType', '')
        if ext:
            defaults[ext] = ct
    partes = []
    todos_cubiertos = True
    for nombre in sorted(set(z.namelist())):
        if nombre == '[Content_Types].xml':
            continue
        pn = '/' + nombre
        ext = nombre.rsplit('.', 1)[-1].lower() if '.' in nombre else ''
        tipo = overrides.get(pn) or defaults.get(ext, '')
        if not tipo:
            todos_cubiertos = False
        partes.append({'ruta': nombre, 'tipo': tipo})
    return partes, todos_cubiertos


def _verificar_relaciones_docx(z: zipfile.ZipFile) -> bool:
    nombres = set(z.namelist())
    atributos_rel = {f'{{{R_NS}}}embed', f'{{{R_NS}}}id', f'{{{R_NS}}}link', f'{{{R_NS}}}dm', f'{{{R_NS}}}lo', f'{{{R_NS}}}qs', f'{{{R_NS}}}cs'}
    revisar = ['word/document.xml'] + sorted(
        n for n in nombres if n.startswith('word/header') or n.startswith('word/footer')
    )
    rels_raiz = _rels_de(z, '')
    if not any(_resolver_rel_destino('', tgt) in nombres for _, tgt, ext in rels_raiz.values() if not ext):
        return False
    for parte in revisar:
        if parte not in nombres:
            return False
        rels = _rels_de(z, parte)
        xml_root = ET.fromstring(z.read(parte))
        for elem in xml_root.iter():
            for k, rid in elem.attrib.items():
                if k in atributos_rel:
                    if rid not in rels:
                        return False
                    _, target, externa = rels[rid]
                    if not externa:
                        resuelto = _resolver_rel_destino(parte, target)
                        if resuelto not in nombres:
                            return False
    return True


def _extraer_secuencia_docx(docx_path: Path):
    with zipfile.ZipFile(docx_path) as z:
        doc_xml = z.read('word/document.xml')
        otros = [z.read(n) for n in z.namelist() if n.startswith('word/header') or n.startswith('word/footer')]
        nums, anums = _inspeccionar_numeracion(z)
        partes, ct_ok = _inspeccionar_content_types(z)
        medios = [
            {'ruta': n, 'sha256': hashlib.sha256(z.read(n)).hexdigest()}
            for n in sorted(z.namelist())
            if n.startswith('word/media/')
        ]
        rels_ok = ct_ok and _verificar_relaciones_docx(z)

    root = ET.fromstring(doc_xml)
    body = root.find(f'{{{W_NS}}}body')
    parrafos = []
    pendiente_salto = False
    for p in body.findall(f'{{{W_NS}}}p'):
        tiene_br = any(br.get(f'{{{W_NS}}}type') == 'page' for br in p.findall(f'.//{{{W_NS}}}br'))
        tiene_pbb = p.find(f'.//{{{W_NS}}}pageBreakBefore') is not None
        if tiene_br or tiene_pbb:
            pendiente_salto = True
        txt = ''.join(p.itertext())
        if txt.strip():
            num_pr = p.find(f'./{{{W_NS}}}pPr/{{{W_NS}}}numPr')
            num_id = None
            ilvl = None
            num_fmt = None
            reinicia = False
            if num_pr is not None:
                nid_el = num_pr.find(f'./{{{W_NS}}}numId')
                ilvl_el = num_pr.find(f'./{{{W_NS}}}ilvl')
                num_id = nid_el.get(f'{{{W_NS}}}val') if nid_el is not None else None
                ilvl = ilvl_el.get(f'{{{W_NS}}}val') if ilvl_el is not None else None
                if num_id and num_id != '0' and num_id in nums:
                    aid, overrides = nums[num_id]
                    nivel = ilvl or '0'
                    num_fmt = anums.get(aid, {}).get(nivel)
                    reinicia = overrides.get(nivel) == '1'
            parrafos.append({
                'texto': txt,
                'salto': pendiente_salto,
                'num_id': num_id,
                'ilvl': ilvl,
                'num_fmt': num_fmt,
                'reinicia': reinicia,
            })
            pendiente_salto = False
    cursivas = _tiene_cursivas_activas(doc_xml) or any(_tiene_cursivas_activas(x) for x in otros)
    return {
        'parrafos': parrafos,
        'cursivas': cursivas,
        'partes': partes,
        'medios': medios,
        'relaciones_validas': rels_ok,
    }


def construir():
    cfg = defaults()
    salida_casos = []
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        for caso in CASOS:
            xlsx = tmp_path / f'{caso["nombre"]}.xlsx'
            crear_excel(xlsx, caso['modo'], caso['encabezados'], caso['filas'])
            work = Work(cfg).analyze(xlsx, caso['modo'], as_of=CORTE)
            for r_idx, res_val in enumerate(caso.get('res', [])):
                if res_val:
                    work.rows[r_idx].review['RES'] = res_val
                    work.rows[r_idx].decisions['resolution'] = 'review'
            for r_idx, obs_val in enumerate(caso.get('editadas', [])):
                if obs_val:
                    work.rows[r_idx].review['OBSERVACION'] = obs_val

            tdir = tmp_path / f'plantillas_{caso["nombre"]}'
            for rel, b64 in caso.get('plantillas_propias', {}).items():
                dest = tdir / rel
                dest.parent.mkdir(parents=True, exist_ok=True)
                dest.write_bytes(base64.b64decode(b64))
            # Copiar matrices institucionales restantes que el caso necesite.
            for src in PLANTILLAS.rglob('*.docx'):
                rel = src.relative_to(PLANTILLAS).as_posix()
                dest = tdir / rel
                if not dest.exists():
                    dest.parent.mkdir(parents=True, exist_ok=True)
                    dest.write_bytes(src.read_bytes())

            _, preparar = preparacion_desde_interfaz(work, [], None)
            proyectos, errores = preparar(tdir)
            assert not errores, f'Errores inesperados en {caso["nombre"]}: {errores}'

            por_tribunal = {}
            for p in proyectos:
                por_tribunal.setdefault(p.court, []).append(p)

            combinados = {}
            for trib, grupo in por_tribunal.items():
                docx_out = tmp_path / f'{caso["nombre"]}_{trib}.docx'
                generate_projects(work, grupo, docx_out)
                insp = _extraer_secuencia_docx(docx_out)
                indice_esperado = ['Índice'] + [
                    f'{i + 1}. {p.rit} — {kind_label(p.kind)}' for i, p in enumerate(grupo)
                ]
                combinados[trib] = {
                    'cantidad': len(grupo),
                    'cursivas': insp['cursivas'],
                    'relaciones_validas': insp['relaciones_validas'],
                    'medios': insp['medios'],
                    'partes': insp['partes'],
                    'indice': indice_esperado,
                    'parrafos': insp['parrafos'],
                }

            salida_casos.append({
                'nombre': caso['nombre'],
                'modo': caso['modo'],
                'encabezados': caso['encabezados'],
                'filas': caso['filas'],
                'res': caso.get('res', []),
                'editadas': caso.get('editadas', []),
                'plantillas_propias': caso.get('plantillas_propias', {}),
                'por_tribunal': combinados,
            })
    return {'corte': CORTE.isoformat(), 'hoy': HOY.isoformat(), 'casos': salida_casos}


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'word_tribunal.json'
    datos = construir()
    destino.parent.mkdir(parents=True, exist_ok=True)
    destino.write_bytes((json.dumps(datos, ensure_ascii=False, indent=2) + '\n').encode('utf-8'))
    total_archivos = sum(len(c['por_tribunal']) for c in datos['casos'])
    print(f'{len(datos["casos"])} casos ({total_archivos} archivos por tribunal) en {destino}')


if __name__ == '__main__':
    main()
