"""E-A9 · Oráculo de «Actualizar desde Excel» en un trabajo que viene de un paquete CSMP_CAPTURE
(`nurus.personal.sync.refresh` sobre `capture_input.import_capture`).

Parte del lote de listados de `app/tests/datos/paquete.json` (Cumplimiento, dos personas) y le agrega tres columnas de origen
(`DIAS DE CUMPLIMIENTO`, `DIAS PARA EGRESAR` y `FEC.EGRESO PROYECTADO`) con su huella, porque el SITFA ficticio no las
emite y sin ellas ninguna edición de origen cambia una regla. Python arma el paquete, lo abre como trabajo, exporta la copia,
se editan columnas de origen en ella y se graba qué recalcula `refresh`: la propuesta y las reglas de cada fila.

La prueba Rust reconstruye el mismo lote, arma su paquete, hace las mismas ediciones sobre su copia y compara.

Uso (desde la raíz): python app/qa/generar_sync_paquete.py [salida.json]   (por omisión app/tests/datos/sync_paquete.json)
"""

import base64
import hashlib
import json
import sys
import tempfile
from datetime import date
from pathlib import Path

sys.stdout.reconfigure(encoding='utf-8')
RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'app' / 'qa'), str(RAIZ / 'descargador'), str(RAIZ / 'src'), str(RAIZ / 'tests')]

from archivo_local import load_sources  # noqa: E402
from export_csmp_capture import export_capture_with_note  # noqa: E402
from nurus.personal.capture_input import import_capture  # noqa: E402
from nurus.personal.config import defaults  # noqa: E402
from nurus.personal.record_edits import apply  # noqa: E402
from nurus.personal.sync import SyncConflict  # noqa: E402
from openpyxl import load_workbook  # noqa: E402

CORTE = date(2026, 10, 8)
EXTRA = [('DIAS DE CUMPLIMIENTO', '300'), ('DIAS PARA EGRESAR', '25'), ('FEC.EGRESO PROYECTADO', '15/12/2026')]
CAMPOS = ('OBSERVACION', 'FECHA_OBS', 'TT', 'CC', 'RES')


def poner(ruta, contenido):
    ruta.write_bytes(contenido)


def lote_con_origen():
    """El lote de listados con las columnas de origen agregadas a cada página y su huella actualizada."""
    datos = json.loads((RAIZ / 'app' / 'tests' / 'datos' / 'paquete.json').read_text(encoding='utf-8'))
    lote = next(e for e in datos['escenarios'] if e['escenario'] == 'listados')['lote']
    archivos = {n: base64.b64decode(c) for n, c in lote['archivos'].items()}
    verificacion = json.loads(archivos['verificacion 001.json'])
    for item in verificacion['archivos']:
        texto = archivos[item['archivo']].decode('utf-8')
        texto = texto.replace('<th>Acciones</th>', '<th>Acciones</th>' + ''.join(f'<th>{h}</th>' for h, _ in EXTRA))
        texto = texto.replace('<td></td></tr>', '<td></td>' + ''.join(f'<td>{v}</td>' for _, v in EXTRA) + '</tr>')
        archivos[item['archivo']] = texto.encode('utf-8')
        item['sha256'] = hashlib.sha256(archivos[item['archivo']]).hexdigest()
        item['bytes'] = len(archivos[item['archivo']])
    archivos['verificacion 001.json'] = json.dumps(verificacion, indent=2, ensure_ascii=False).encode('utf-8')
    return {'carpeta': lote['carpeta'], 'archivos': {n: base64.b64encode(c).decode() for n, c in sorted(archivos.items())}}


def reconstruir(carpeta, archivos):
    for ruta, contenido in archivos.items():
        destino = carpeta / ruta
        destino.parent.mkdir(parents=True, exist_ok=True)
        poner(destino, base64.b64decode(contenido))


def abrir(base, lote):
    carpeta = base / lote['carpeta']
    reconstruir(carpeta, lote['archivos'])
    ruta, _ = export_capture_with_note(load_sources([carpeta], False), base / 'captura.zip', diaries=None,
                                       match_diaries=False, consolidated=False, meses=4)
    return import_capture(ruta, defaults(), as_of=CORTE)


def texto(valor):
    if valor is None:
        return ''
    if isinstance(valor, float) and valor.is_integer():
        valor = int(valor)
    return str(valor)


def caso(lote, nombre, remotas=(), locales=(), resoluciones=None):
    with tempfile.TemporaryDirectory() as t:
        base = Path(t)
        obra = abrir(base, lote)
        obra.export(base / 'salida.xlsx', backend='portable', reduced_fidelity=True)
        indice = {fila.id: i for i, fila in enumerate(obra.rows)}
        for fila, campo, valor in locales:
            apply(obra, [obra.rows[fila].id], review={campo: valor})
        libro = load_workbook(obra.output)
        hoja = libro[obra.sheet]
        columnas = {c.value: c.column for c in hoja[obra.header]}
        for fila, campo, valor in remotas:
            hoja.cell(obra.header + 1 + fila, columnas[campo]).value = valor
        libro.save(obra.output)
        claves = {(obra.rows[i].id, c): v for (i, c), v in (resoluciones or {}).items()}
        salida = {'escenario': nombre, 'locales': [list(x) for x in locales], 'remotas': [list(x) for x in remotas],
                  'resoluciones': [[i, c, v] for (i, c), v in (resoluciones or {}).items()]}
        try:
            salida['cambio'] = bool(obra.refresh(claves or None))
        except SyncConflict as e:
            salida['conflictos'] = sorted([[indice[c.record_id], c.field] for c in e.conflicts])
        except ValueError as e:
            salida['error'] = str(e)
        salida['revision'] = [{c: texto(f.review.get(c, '')) for c in CAMPOS} for f in obra.rows]
        salida['propuestas'] = [f.observation for f in obra.rows]
        salida['reglas'] = [list(f.rules) for f in obra.rows]
        return salida


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'sync_paquete.json'
    lote = lote_con_origen()
    casos = [
        caso(lote, 'sin_cambios'),
        # Egreso proyectado ya pasado y días negativos: la medida pasa de «por vencer» a vencida.
        caso(lote, 'origen_editado', remotas=[(0, 'FEC.EGRESO PROYECTADO', '01/09/2026'), (0, 'DIAS PARA EGRESAR', -10)]),
        caso(lote, 'origen_editado_en_las_dos_filas', remotas=[(0, 'DIAS PARA EGRESAR', 80), (1, 'DIAS PARA EGRESAR', 5)]),
        caso(lote, 'origen_editado_y_observacion_local', locales=[(1, 'OBSERVACION', 'Versión local')],
             remotas=[(1, 'FEC.EGRESO PROYECTADO', '01/09/2026'), (1, 'DIAS PARA EGRESAR', -10)]),
        caso(lote, 'origen_editado_y_revision_remota', remotas=[(0, 'DIAS PARA EGRESAR', 80), (0, 'CC', 1), (0, 'OBSERVACION', 'Texto de Excel')]),
        caso(lote, 'origen_con_el_mismo_valor', remotas=[(0, 'DIAS PARA EGRESAR', 25)]),
    ]
    poner(destino, (json.dumps({'corte': CORTE.isoformat(), 'lote': lote, 'casos': casos}, ensure_ascii=False, indent=1) + '\n').encode('utf-8'))
    for c in casos:
        print(c['escenario'], c.get('error') or c.get('conflictos') or c.get('cambio'), [r[:1] for r in c['reglas']])
    print(f'{len(casos)} casos en {destino}')


if __name__ == '__main__':
    main()
