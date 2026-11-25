"""E-P1 · Oráculo de proyectos Word por tribunal (`resolutions.prepare_projects` y `generate_projects`):
arma casos ficticios con Laja y Mulchén, genera con Python el `.docx` de cada tribunal
y graba la secuencia de párrafos (`texto`, `salto`) y si quedan cursivas activas.

Uso (desde la raíz): python app/qa/generar_word_tribunal.py [salida.json]
(por omisión app/tests/datos/word_tribunal.json)
"""

import base64
import io
import json
import sys
import tempfile
import xml.etree.ElementTree as ET
import zipfile
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


class _FijaMeta(type(date)):
    def __instancecheck__(cls, obj):
        return isinstance(obj, date)


class _Fija(date, metaclass=_FijaMeta):
    @classmethod
    def today(cls):
        return HOY


outputs.date = _Fija
PLANTILLAS = RAIZ / 'src' / 'nurus' / 'personal' / 'plantillas_word'


def _crear_docx_propio(*, con_cursivas: bool, etiqueta: str) -> str:
    """Construye un .docx determinista en memoria (ZipInfo.create_system = 3) para probar matriz propia y cursivas."""
    estilo_italic = '<w:i/>' if con_cursivas else ''
    rpr_italic = '<w:i/><w:i w:val="1"/><w:iCs/>' if con_cursivas else ''
    doc_xml = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" '
        'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
        '<w:body>'
        f'<w:p><w:pPr><w:pStyle w:val="EstiloPropio"/></w:pPr><w:r><w:rPr>{rpr_italic}</w:rPr>'
        f'<w:t xml:space="preserve">{etiqueta} · {{{{FECHA}}}}</w:t></w:r></w:p>'
        f'<w:p><w:r><w:rPr>{rpr_italic}</w:rPr>'
        '<w:t xml:space="preserve">Causa {{RIT}} de {{NOMBRE}}, programa {{PROGRAMA}}.</w:t></w:r></w:p>'
        '<w:sectPr><w:pgSz w:w="12240" w:h="15840"/>'
        '<w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr>'
        '</w:body></w:document>'
    )
    styles_xml = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        '<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>'
        f'<w:style w:type="paragraph" w:styleId="EstiloPropio"><w:name w:val="EstiloPropio"/><w:rPr>{estilo_italic}</w:rPr></w:style>'
        '</w:styles>'
    )
    ct_xml = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
        '<Default Extension="xml" ContentType="application/xml"/>'
        '<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>'
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
        '</Relationships>'
    )
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, 'w', compression=zipfile.ZIP_STORED) as z:
        for nombre, cuerpo in (
            ('[Content_Types].xml', ct_xml),
            ('_rels/.rels', rels_xml),
            ('word/document.xml', doc_xml),
            ('word/_rels/document.xml.rels', doc_rels_xml),
            ('word/styles.xml', styles_xml),
        ):
            zi = zipfile.ZipInfo(nombre, date_time=(2026, 11, 20, 0, 0, 0))
            zi.create_system = 3
            zi.external_attr = 0o644 << 16
            zi.compress_type = zipfile.ZIP_STORED
            z.writestr(zi, cuerpo.encode('utf-8'))
    return base64.b64encode(buf.getvalue()).decode('ascii')


PLANTILLA_PROPIA_LAJA = _crear_docx_propio(con_cursivas=False, etiqueta='Matriz propia Laja')
PLANTILLA_CURSIVAS_MULCHEN = _crear_docx_propio(con_cursivas=True, etiqueta='Matriz cursivas Mulchén')

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


def _extraer_secuencia_docx(docx_path: Path):
    with zipfile.ZipFile(docx_path) as z:
        doc_xml = z.read('word/document.xml')
        otros = [z.read(n) for n in z.namelist() if n.startswith('word/header') or n.startswith('word/footer')]
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
            parrafos.append({'texto': txt, 'salto': pendiente_salto})
            pendiente_salto = False
    cursivas = _tiene_cursivas_activas(doc_xml) or any(_tiene_cursivas_activas(x) for x in otros)
    return parrafos, cursivas


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
                parrafos, cursivas = _extraer_secuencia_docx(docx_out)
                indice_esperado = ['Índice'] + [
                    f'{i + 1}. {p.rit} — {kind_label(p.kind)}' for i, p in enumerate(grupo)
                ]
                combinados[trib] = {
                    'cantidad': len(grupo),
                    'cursivas': cursivas,
                    'indice': indice_esperado,
                    'parrafos': parrafos,
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
