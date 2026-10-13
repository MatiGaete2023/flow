"""Resume la estructura de copias HTML de bitácora sin textos, nombres, RIT ni identificadores.

Uso (en el PC autorizado, desde la raíz del repositorio):
    python descargador/extraer_estructura_bitacora.py bitacora-a.html [bitacora-b.html ...] > estructura.json

Solo `estructura.json` puede compartirse. Los HTML originales no salen del equipo.
Entrega conteos, códigos numéricos de ventana/etapa/consulta y el resultado del lector del
Asistente (entradas leídas o mensaje de error), nunca valores de celdas ni campos ocultos.
"""

import json
import re
import sys
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'src'))

from nurus.bitacora_html import FULL_FIELDS, HEADERS, IDENTITY_FIELDS, decode_snapshot, normalize, read_snapshot  # noqa: E402

CODES = ('tipo_popUp', 'COD_Etapa', 'TIP_Consulta')
NUMERIC_IMG = re.compile(r'<img\b[^>]*\bid\s*=\s*["\']?\d+', re.I)


def _hidden(text, name):
    found = re.search(r'name\s*=\s*["\']' + name + r'["\'][^>]*value\s*=\s*["\']([^"\']*)', text, re.I)
    return found.group(1) if found else ''


def _raw(text):
    start = text.find('id="TablaInforme"')
    end = text.find('</table', start) if start >= 0 else -1
    inside = text[start:end] if start >= 0 and end >= 0 else ''
    centro = len(re.findall(r'HOBS_Centro_\d+', text))
    return {
        'tablas': len(re.findall(r'<table\b', text, re.I)),
        'tablas_tablainforme': text.count('id="TablaInforme"'),
        'tr': len(re.findall(r'<tr\b', text, re.I)),
        'form': len(re.findall(r'<form\b', text, re.I)),
        'cierre_html': bool(re.search(r'</html\s*>', text, re.I)),
        'hobs_centro_total': centro,
        'hobs_centro_antes_de_cerrar_tablainforme': len(re.findall(r'HOBS_Centro_\d+', inside)),
        'img_numerica_total': len(NUMERIC_IMG.findall(text)),
        'img_numerica_antes_de_cerrar_tablainforme': len(NUMERIC_IMG.findall(inside)),
        'tr_antes_de_cerrar_tablainforme': len(re.findall(r'<tr\b', inside, re.I)),
    }


def _lxml(text):
    from lxml import html

    document = html.fromstring(text)
    tables = document.xpath('//table[@id="TablaInforme"]')
    if len(tables) != 1:
        return {'tablainforme': len(tables)}
    table = tables[0]
    rows = table.xpath('.//tr')
    header = tuple(normalize(' '.join(c.text_content().split())) for c in rows[0].xpath('./td|./th')) if rows else ()
    return {
        'filas_en_tabla': len(rows),
        'celdas_por_fila': dict(Counter(len(r.xpath('./td')) for r in rows[1:])),
        'cabecera_esperada': header == tuple(map(normalize, HEADERS)),
        'columnas_cabecera': len(header),
        'ocultos_en_tabla': sum(1 for i in table.xpath('.//input') if i.get('name', '').startswith(FULL_FIELDS)),
        'ocultos_en_documento': sum(1 for i in document.iter('input') if i.get('name', '').startswith(FULL_FIELDS)),
        'img_numerica_en_tabla': sum(1 for i in table.xpath('.//img[@id]') if i.get('id', '').isdigit()),
    }


def describe(path):
    data = Path(path).read_bytes()
    text = decode_snapshot(data)
    result = {'archivo': f'archivo-{describe.count}', 'bytes': len(data)}
    describe.count += 1
    result.update({code: _hidden(text, code) for code in CODES if _hidden(text, code).isdigit()})
    result['bruto'] = _raw(text)
    try:
        result['lxml'] = _lxml(text)
    except Exception as exc:  # noqa: BLE001 - diagnóstico: informa el tipo, nunca el contenido
        result['lxml'] = {'error': type(exc).__name__}
    params = {key: _hidden(text, key) for key in IDENTITY_FIELDS}
    params['tipo_popUp'] = _hidden(text, 'tipo_popUp') or '12'
    try:
        result['lector_entradas'] = len(read_snapshot(data, params)['entries'])
    except ValueError as exc:
        result['lector_error'] = str(exc).split(':')[0]
    return result


describe.count = 1


def main(argv):
    if not argv:
        print(__doc__, file=sys.stderr)
        return 2
    print(
        json.dumps(
            {'tipo': 'ESTRUCTURA_BITACORA_ANONIMA', 'version': 1, 'copias': [describe(p) for p in argv]},
            ensure_ascii=False,
            indent=2,
        )
    )
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
