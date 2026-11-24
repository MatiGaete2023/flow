"""E-A1 · Oráculo de «Actualizar desde Excel» de Flow Python (`nurus.personal.sync.refresh`).

Arma con Python un trabajo sintético de tres filas (Espera), lo exporta, aplica ediciones locales y ediciones en la
copia de Excel, y graba el resultado de `Work.refresh`: los cinco campos de revisión por fila, los conflictos y los
rechazos con su mensaje. La prueba Rust repite las mismas ediciones sobre su propio Trabajo y compara.

Uso (desde la raíz; la CI lo regenera en Windows y exige el mismo resultado): python app/qa/generar_sync.py [salida.json]   (por omisión app/tests/datos/sync.json)
"""

import json
import sys
import tempfile
from datetime import date
from pathlib import Path

sys.stdout.reconfigure(encoding='utf-8')
sys.stderr.reconfigure(encoding='utf-8')
RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'src')]

from openpyxl import Workbook, load_workbook  # noqa: E402
from nurus.personal.config import defaults  # noqa: E402
from nurus.personal.record_edits import apply  # noqa: E402
from nurus.personal.sync import SyncConflict  # noqa: E402
from nurus.personal.work import Work  # noqa: E402

CORTE = date(2026, 9, 13)
CAMPOS = ('OBSERVACION', 'FECHA_OBS', 'TT', 'CC', 'RES')
FILAS = [
    ('X-1-2026', 'Jgdo. L. y G. de Laja', 'Persona Ficticia Uno', '11111111-1', 'AFT EJEMPLO UNO', 30),
    ('X-2-2026', 'Jgdo. L. y G. de Laja', 'Persona Ficticia Dos', '22222222-2', 'AFT EJEMPLO DOS', 45),
    ('X-3-2026', 'Jgdo. L. y G. de Mulchen', 'Persona Ficticia Tres', '33333333-3', 'AFT EJEMPLO UNO', 60),
]


def origen(carpeta):
    libro = Workbook()
    hoja = libro.active
    hoja.title = 'informe_1'
    hoja.append(['Reporte RUS'])
    hoja.append([])
    hoja.append(['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'T ESPERA'])
    for fila in FILAS:
        hoja.append(list(fila))
    ruta = carpeta / 'origen.xlsx'
    libro.save(ruta)
    return ruta


def texto(valor):
    """Cómo compara la prueba Rust un valor de revisión."""
    if valor is None:
        return ''
    if isinstance(valor, float) and valor.is_integer():
        valor = int(valor)
    return str(valor)


def revision(fila):
    return {campo: texto(fila.review.get(campo, '')) for campo in CAMPOS}


def trabajo(carpeta):
    obra = Work(defaults()).analyze(origen(carpeta), 'ESPERA', as_of=CORTE)
    obra.export(carpeta / 'salida.xlsx', backend='portable', reduced_fidelity=True)
    return obra


def editar_excel(obra, ediciones, quitar_id=False, quitar_filas=0, cambiar_identidad=None):
    libro = load_workbook(obra.output)
    hoja = libro[obra.sheet]
    columnas = {c.value: c.column for c in hoja[3]}
    for fila, campo, valor in ediciones:
        hoja.cell(4 + fila, columnas[campo]).value = valor
    if cambiar_identidad is not None:
        hoja.cell(4 + cambiar_identidad, columnas['NOMBRE']).value = 'Otra Persona Ficticia'
    if quitar_filas:
        hoja.delete_rows(4 + len(FILAS) - quitar_filas, quitar_filas)
    if quitar_id:
        hoja.delete_cols(columnas['NURUS_ID_REGISTRO'])
    libro.save(obra.output)


def caso(nombre, locales=(), remotas=(), resoluciones=None, **excel):
    with tempfile.TemporaryDirectory() as t:
        carpeta = Path(t)
        obra = trabajo(carpeta)
        indice = {fila.id: i for i, fila in enumerate(obra.rows)}
        for fila, campo, valor in locales:
            apply(obra, [obra.rows[fila].id], review={campo: valor})
        editar_excel(obra, remotas, **excel)
        claves = {(obra.rows[i].id, c): v for (i, c), v in (resoluciones or {}).items()}
        salida = {'escenario': nombre, 'locales': [list(x) for x in locales], 'remotas': [list(x) for x in remotas],
                  'excel': {k: v for k, v in excel.items()},
                  'resoluciones': [[i, c, v] for (i, c), v in (resoluciones or {}).items()]}
        try:
            cambio = obra.refresh(claves or None)
            salida['cambio'] = bool(cambio)
        except SyncConflict as e:
            salida['conflictos'] = sorted(
                [[indice[c.record_id], c.field, texto(c.base), texto(c.local), texto(c.incoming)] for c in e.conflicts]
            )
        except ValueError as e:
            salida['error'] = str(e)
        salida['revision'] = [revision(fila) for fila in obra.rows]
        return salida


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'sync.json'
    casos = [
        caso('sin_cambios'),
        caso('remota', remotas=[(0, 'OBSERVACION', 'Constancia editada en Excel'), (0, 'CC', 1), (1, 'TT', 1),
                                (2, 'FECHA_OBS', '2026-09-14')]),
        caso('solo_local', locales=[(1, 'OBSERVACION', 'Edición local')]),
        caso('local_y_remota_en_filas_distintas', locales=[(0, 'OBSERVACION', 'Edición local')],
             remotas=[(1, 'OBSERVACION', 'Edición en Excel'), (1, 'CC', 1)]),
        caso('conflicto', locales=[(0, 'OBSERVACION', 'Versión local')], remotas=[(0, 'OBSERVACION', 'Versión de Excel')]),
        caso('conflicto_resuelto_con_excel', locales=[(0, 'OBSERVACION', 'Versión local')],
             remotas=[(0, 'OBSERVACION', 'Versión de Excel')], resoluciones={(0, 'OBSERVACION'): 'Versión de Excel'}),
        caso('conflicto_resuelto_con_local', locales=[(0, 'OBSERVACION', 'Versión local')],
             remotas=[(0, 'OBSERVACION', 'Versión de Excel')], resoluciones={(0, 'OBSERVACION'): 'Versión local'}),
        caso('misma_edicion_en_ambos', locales=[(2, 'OBSERVACION', 'Igual en ambos')],
             remotas=[(2, 'OBSERVACION', 'Igual en ambos')]),
        caso('res_remota', remotas=[(0, 'RES', 'NOMENCL')]),
        caso('sin_columna_tecnica', remotas=[(0, 'OBSERVACION', 'Sin columna técnica'), (0, 'CC', 1)], quitar_id=True),
        caso('identidad_cambiada', cambiar_identidad=0, remotas=[(0, 'CC', 1)]),
        caso('faltan_registros', quitar_filas=1),
    ]
    # Bytes y no texto: en Windows `write_text` cambiaría los saltos de línea y la CI compara byte a byte.
    destino.write_bytes(json.dumps({
        'corte': CORTE.isoformat(), 'campos': list(CAMPOS),
        'encabezados': ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'T ESPERA'],
        'filas': [list(f) for f in FILAS], 'casos': casos,
    }, ensure_ascii=False, indent=1).encode('utf-8'))
    print(f'{len(casos)} casos en {destino}')


if __name__ == '__main__':
    main()
