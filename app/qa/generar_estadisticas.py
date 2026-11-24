"""E-T3 · Oráculo de las estadísticas (`statistics.summarize` y `export_summary`) y de las métricas locales opcionales
(`review_metrics`) de Flow Python.

Arma trabajos sintéticos mínimos (filas con revisión, acciones y recibos) y graba los conteos, la hoja de constancias y el
documento de métricas. Las marcas de tiempo no se comparan: solo si están o no.

Uso (desde la raíz): python app/qa/generar_estadisticas.py [salida.json]   (por omisión app/tests/datos/estadisticas.json)
"""

import json
import re
import sys
import tempfile
from pathlib import Path
from types import SimpleNamespace as NS
from unittest.mock import MagicMock

sys.stdout.reconfigure(encoding='utf-8')
RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'src')]
for _m in ('tkinter', 'tkinter.ttk', 'tkinter.filedialog', 'tkinter.messagebox', 'tkinter.simpledialog'):
    sys.modules.setdefault(_m, MagicMock())

from openpyxl import load_workbook  # noqa: E402
from nurus.personal import review_metrics  # noqa: E402
from nurus.personal.statistics import export_summary, summarize  # noqa: E402


def fila(i, excluido=False, observacion='', fecha='', tt='', cc='', res='', acciones=(), editada=False):
    review = {}
    for clave, valor in (('OBSERVACION', observacion), ('FECHA_OBS', fecha), ('TT', tt), ('CC', cc), ('RES', res)):
        if valor != '':
            review[clave] = valor
    return NS(id=f'r{i}', excluded=excluido, review=review, actions=list(acciones), overrides={},
              values={'T': f'Tribunal {i % 2}', 'R': f'X-{i}-2099'}, rules=[], decisions={'resolution': 'auto'},
              word_overrides={}, editada=editada)


def trabajo(filas, recibos=()):
    return NS(rows=filas, mapping={'tribunal': 'T', 'rit': 'R'}, output='', receipts={f'k{i}': dict(r) for i, r in enumerate(recibos)})


def datos_fila(f):
    return {'excluido': f.excluded, 'review': f.review, 'acciones': f.actions}


def estadistica(nombre, filas, recibos=()):
    obra = trabajo(filas, recibos)
    caso = {'nombre': nombre, 'filas': [datos_fila(f) for f in filas], 'recibos': [dict(r) for r in recibos],
            'resumen': summarize(obra)}
    with tempfile.TemporaryDirectory() as t:
        destino = Path(t) / 'resumen.xlsx'
        export_summary(obra, destino)
        libro = load_workbook(destino)
        caso['hojas'] = [ws.title for ws in libro]
        caso['resumen_filas'] = [list(r) for r in libro['Resumen'].iter_rows(values_only=True)]
        caso['constancias'] = [[str(c) if c is not None else '' for c in r] for r in libro['Constancias'].iter_rows(values_only=True)]
        caso['constancias_numeros'] = [[type(c).__name__ for c in r] for r in libro['Constancias'].iter_rows(values_only=True)][1:]
    return caso


def normalizar_metricas(doc):
    """Quita las marcas de tiempo: solo importa si existen."""
    ciclos = []
    for c in doc['ciclos']:
        ciclos.append({k: (bool(v) if k in ('importado', 'exportado', 'primer_borrador', 'ultimo_borrador') else v) for k, v in c.items()})
    return ciclos


def metricas(nombre, pasos, activa=True, cambios=None, filas=(), previo=None):
    """`pasos`: lista de 'importar', 'exportar' y 'borrador'."""
    caso = {'nombre': nombre, 'pasos': list(pasos), 'activa': activa, 'cambios': cambios,
            'filas': [datos_fila(f) | {'editada': f.editada, 'id': f.id} for f in filas], 'previo': previo}
    with tempfile.TemporaryDirectory() as t:
        t = Path(t)
        if previo is not None:
            (t / 'metricas.json').write_text(previo if isinstance(previo, str) else json.dumps(previo), encoding='utf-8')
        obra = trabajo(list(filas))
        obra.config = {'metricas_locales': activa}
        obra.storage_directory = str(t)
        obra.capture_changes = cambios
        # `_edited` de la conciliación mira lo que la persona decidió: aquí, la marca `editada` de la fila sintética.
        original = review_metrics._edited
        review_metrics._edited = lambda row: bool(row.editada)
        try:
            for paso in pasos:
                {'importar': review_metrics.record_import, 'exportar': review_metrics.record_export,
                 'borrador': review_metrics.record_draft}[paso](obra)
        finally:
            review_metrics._edited = original
        archivo = t / 'metricas.json'
        caso['archivo'] = normalizar_metricas(json.loads(archivo.read_text(encoding='utf-8'))) if archivo.exists() else None
        caso['resumen'] = normalizar_metricas(review_metrics.summary(archivo)) if archivo.exists() else None
    return caso


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'estadisticas.json'
    completa = dict(observacion='Obs', fecha='2026-11-14', tt='1', cc='1')
    casos = [
        estadistica('vacio', []),
        estadistica('sin_revision', [fila(1), fila(2, excluido=True)]),
        estadistica('constancia_con_carga', [fila(1, **completa)]),
        estadistica('constancia_sin_carga', [fila(1, observacion='Obs', fecha='14/11/2026', tt='1', cc='0')]),
        estadistica('mezcla', [
            fila(1, **completa),
            fila(2, observacion='Obs', fecha='2026-11-15', tt='1', cc='0'),
            fila(3, observacion='Obs', fecha='2026-11-16', tt='2', cc='1'),
            fila(4, observacion='', fecha='2026-11-16', tt='1', cc='1'),
            fila(5, observacion='Obs', fecha='no es fecha', tt='1', cc='1'),
            fila(6, observacion='Obs', fecha='2026-11-16', tt='1', cc='3'),
            fila(7, excluido=True, **completa),
            fila(8, res='NOMENCL', acciones=['PC_IE']),
            fila(9, res='PC_IE', acciones=['PC_IE']),
            fila(10, res='pc info', acciones=['PC_IE']),
            fila(11, cc='1'),
        ]),
        estadistica('valores_numericos_y_vacios', [
            fila(1, observacion='Obs', fecha='2026-11-14', tt='1.0', cc='0,0'),
            fila(2, observacion='Obs', fecha='  2026-11-14 ', tt=' 1 ', cc='1'),
            fila(3, observacion='Obs', fecha='---', tt='1', cc='1'),
            fila(4, observacion='Obs', fecha='2026-11-14', tt='-', cc='1'),
        ]),
        estadistica('recibos', [fila(1, **completa)], [
            {'kind': 'word', 'state': 'created', 'path': 'a.docx'},
            {'kind': 'word', 'state': 'created', 'path': 'a.docx'},
            {'kind': 'word', 'state': 'created', 'path': 'b.docx'},
            {'kind': 'draft', 'state': 'created'},
            {'kind': 'draft', 'state': 'uncertain'},
            {'kind': 'excel', 'state': 'generated', 'path': 'x.xlsx'},
        ]),
    ]
    cambios = {'version': 1, 'base_digest': 'x', 'filas': {
        'r1': {'tipo': 'SIN_CAMBIO'}, 'r2': {'tipo': 'NUEVO'}, 'r3': {'tipo': 'CAMBIO_FUENTE'}, 'r4': {'tipo': 'CAMBIO_REGLAS'},
    }, 'egresados': [{'id_anterior': 'a'}, {'id_anterior': 'b'}]}
    filas = [fila(1, editada=True), fila(2, editada=True), fila(3), fila(4, editada=True)]
    ciclo = {'importado': '2026-01-01', 'exportado': '', 'primer_borrador': '', 'ultimo_borrador': '', 'filas': 1, 'nuevas': 0,
             'cambio_fuente': 0, 'cambio_reglas': 0, 'egresadas': 0, 'revisadas': 0, 'revisadas_sin_cambio': 0,
             'ediciones_despues_de_exportar': 0}
    mets = [
        metricas('desactivada', ['importar', 'exportar', 'borrador'], activa=False, filas=filas, cambios=cambios),
        metricas('importar_con_cambios', ['importar'], filas=filas, cambios=cambios),
        metricas('importar_sin_comparacion', ['importar'], filas=filas),
        metricas('ciclo_completo', ['importar', 'exportar', 'borrador', 'borrador'], filas=filas, cambios=cambios),
        metricas('exportar_sin_importar', ['exportar', 'borrador']),
        metricas('limite_de_60_ciclos', ['importar'], filas=filas, cambios=cambios, previo={'version': 1, 'ciclos': [ciclo] * 60}),
        metricas('archivo_danado', ['importar'], filas=filas, cambios=cambios, previo='{no es json'),
    ]
    destino.write_bytes((json.dumps({'estadisticas': casos, 'metricas': mets}, ensure_ascii=False, indent=1) + '\n').encode('utf-8'))
    print(f'{len(casos)} estadísticas y {len(mets)} métricas en {destino}')


if __name__ == '__main__':
    main()
