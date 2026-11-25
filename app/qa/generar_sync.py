"""E-A1 · Oráculo de «Actualizar desde Excel» de Flow Python (`nurus.personal.sync.refresh`).

Arma con Python un trabajo sintético de tres filas (Espera), lo exporta, aplica ediciones locales y ediciones en la
copia de Excel, y graba el resultado de `Work.refresh`: los cinco campos de revisión por fila, los conflictos y los
rechazos con su mensaje. La prueba Rust repite las mismas ediciones sobre su propio Trabajo y compara.

Uso (desde la raíz; la CI lo regenera en Windows y exige el mismo resultado (sin contar CRLF)): python app/qa/generar_sync.py [salida.json]   (por omisión app/tests/datos/sync.json)
"""

import json
import sys
import tempfile
from copy import deepcopy
from datetime import date
from pathlib import Path

sys.stdout.reconfigure(encoding='utf-8')
sys.stderr.reconfigure(encoding='utf-8')
RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'src')]

from openpyxl import Workbook, load_workbook  # noqa: E402
from nurus.personal.config import defaults  # noqa: E402
from nurus.personal.record_edits import apply, encode  # noqa: E402
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


def tecnico(obra, fila, mail=None, overrides=None, resolution=None):
    """La celda `NURUS_DECISIONES` que dejaría Flow Python con esas gestiones, resoluciones o anulaciones en la fila."""
    copia = deepcopy(obra.rows[fila])
    if mail is not None:
        copia.decisions = {**copia.decisions, 'mail': dict(mail), 'structured': True}
    if overrides is not None:
        copia.overrides = dict(overrides)
    if resolution is not None:
        copia.decisions = {**copia.decisions, 'resolution': resolution, 'structured': True}
    return encode(copia)


def caso(
    nombre,
    locales=(),
    remotas=(),
    resoluciones=None,
    correos_locales=(),
    correos_remotos=(),
    anulaciones_locales=(),
    anulaciones_remotas=(),
    resoluciones_locales_decisiones=(),
    resoluciones_remotas_decisiones=(),
    celda_tecnica=None,
    sesion_antigua=False,
    resolver_decisiones=None,
    **excel,
):
    with tempfile.TemporaryDirectory() as t:
        carpeta = Path(t)
        obra = trabajo(carpeta)
        indice = {fila.id: i for i, fila in enumerate(obra.rows)}
        for fila, campo, valor in locales:
            apply(obra, [obra.rows[fila].id], review={campo: valor})
        for fila, mail in correos_locales:
            apply(obra, [obra.rows[fila].id], decisions={'mail': dict(mail)})
        for fila, overrides in anulaciones_locales:
            apply(obra, [obra.rows[fila].id], overrides=dict(overrides))
        for fila, res in resoluciones_locales_decisiones:
            apply(obra, [obra.rows[fila].id], decisions={'resolution': res})
        if sesion_antigua:
            for fila in obra.rows:
                fila.sync_state = None
                fila.sync_base = None
                fila.sync_local = None
        por_fila = {}
        for fila, mail in correos_remotos:
            por_fila.setdefault(fila, {})['mail'] = mail
        for fila, overrides in anulaciones_remotas:
            por_fila.setdefault(fila, {})['overrides'] = overrides
        for fila, res in resoluciones_remotas_decisiones:
            por_fila.setdefault(fila, {})['resolution'] = res
        remotas = list(remotas) + [
            (fila, 'NURUS_DECISIONES', tecnico(obra, fila, **datos)) for fila, datos in sorted(por_fila.items())
        ]
        if celda_tecnica is not None:
            remotas.append((0, 'NURUS_DECISIONES', celda_tecnica))
        editar_excel(obra, remotas, **excel)
        res_map = dict(resoluciones or {})
        if resolver_decisiones == 'excel':
            for f, c, v in remotas:
                if c == 'NURUS_DECISIONES':
                    res_map[(f, 'NURUS_DECISIONES')] = v
        elif resolver_decisiones == 'local':
            for f in sorted({f for f, c, _ in remotas if c == 'NURUS_DECISIONES'}):
                res_map[(f, 'NURUS_DECISIONES')] = encode(obra.rows[f])
        claves = {(obra.rows[i].id, c): v for (i, c), v in res_map.items()}
        salida = {'escenario': nombre, 'locales': [list(x) for x in locales], 'remotas': [list(x) for x in remotas],
                  'correos_locales': [[f, m] for f, m in correos_locales],
                  'anulaciones_locales': [[f, o] for f, o in anulaciones_locales],
                  'excel': {k: v for k, v in excel.items()},
                  'resoluciones': [[i, c, v] for (i, c), v in res_map.items()]}
        if sesion_antigua:
            salida['sesion_antigua'] = True
        try:
            cambio = obra.refresh(claves or None)
            salida['cambio'] = bool(cambio)
        except SyncConflict as e:
            # La celda técnica trae todo el estado de la fila: solo se compara qué fila y qué campo chocan.
            salida['conflictos'] = sorted(
                [[indice[c.record_id], c.field] + ([''] * 3 if c.field == 'NURUS_DECISIONES' else [texto(c.base), texto(c.local), texto(c.incoming)])
                 for c in e.conflicts]
            )
        except ValueError as e:
            salida['error'] = str(e)
        salida['revision'] = [revision(fila) for fila in obra.rows]
        # Lo que recalcula Python al cambiar una columna de origen en la copia: la propuesta y las reglas de cada fila.
        salida['propuestas'] = [fila.observation for fila in obra.rows]
        salida['reglas'] = [list(fila.rules) for fila in obra.rows]
        salida['correo'] = [dict(fila.decisions.get('mail', {})) for fila in obra.rows]
        salida['anulaciones'] = [dict(fila.overrides) for fila in obra.rows]
        salida['resolucion'] = [fila.decisions.get('resolution', 'auto') for fila in obra.rows]
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
        # E-A7 (F10): columnas de origen editadas en la copia; Python recalcula la propuesta y las reglas de esas filas.
        caso('origen_editado', remotas=[(0, 'T ESPERA', 150), (1, 'T ESPERA', 5)]),
        caso('origen_editado_y_revision_remota', remotas=[(2, 'T ESPERA', 100), (2, 'CC', 1), (2, 'OBSERVACION', 'Texto de Excel')]),
        caso('origen_editado_y_observacion_local', locales=[(0, 'OBSERVACION', 'Versión local')], remotas=[(0, 'T ESPERA', 150)]),
        caso('origen_editado_con_conflicto', locales=[(1, 'OBSERVACION', 'Versión local')],
             remotas=[(1, 'T ESPERA', 120), (1, 'OBSERVACION', 'Versión de Excel')]),
        caso('origen_con_el_mismo_valor', remotas=[(0, 'T ESPERA', 30)]),
        # E-A8 (F08): gestiones de correo por fila en `NURUS_DECISIONES`.
        caso('correo_remoto', correos_remotos=[(0, {'medidas': 'omit'})]),
        caso('correo_local', correos_locales=[(1, {'programa_espera': 'include'})]),
        caso('correo_remoto_y_revision', correos_remotos=[(2, {'programa_espera': 'omit', 'medidas': 'include'})],
             remotas=[(2, 'CC', 1)]),
        caso('correo_conflicto', correos_locales=[(0, {'medidas': 'omit'})], correos_remotos=[(0, {'medidas': 'include'})]),
        caso('correo_igual_en_ambos', correos_locales=[(0, {'medidas': 'omit'})], correos_remotos=[(0, {'medidas': 'omit'})]),
        caso('correo_local_y_remoto_en_filas_distintas', correos_locales=[(0, {'medidas': 'omit'})],
             correos_remotos=[(1, {'programa_espera': 'include'})]),
        caso('celda_tecnica_invalida', celda_tecnica='esto no es json'),
        caso('celda_tecnica_con_gestion_invalida',
             celda_tecnica=json.dumps({'version': 1, 'decisions': {'mail': {'medidas': 'siempre'}}, 'overrides': {}, 'word_overrides': {}, 'excluded': False})),
        # E-A10 (F08): anulaciones de columnas de origen en `NURUS_DECISIONES` y combinación con columnas editadas.
        caso('anulacion_local_y_revision_remota', anulaciones_locales=[(0, {'espera': '150'})], remotas=[(0, 'CC', 1)]),
        caso('anulacion_local_y_origen_editado_en_excel', anulaciones_locales=[(0, {'espera': '150'})], remotas=[(0, 'T ESPERA', 5)]),
        caso('anulacion_remota_y_origen_editado_en_excel', anulaciones_remotas=[(0, {'espera': '150'})], remotas=[(0, 'T ESPERA', 5)]),
        caso('anulacion_conflicto', anulaciones_locales=[(0, {'espera': '150'})], anulaciones_remotas=[(0, {'espera': '5'})]),
        caso('anulacion_igual_en_ambos', anulaciones_locales=[(0, {'espera': '150'})], anulaciones_remotas=[(0, {'espera': '150'})]),
        caso('anulacion_y_correo_conflicto', correos_locales=[(0, {'medidas': 'omit'})], anulaciones_remotas=[(0, {'espera': '150'})]),
        # E-A11 (F10): NURUS_DECISIONES completo: resolución remota, conflictos resueltos, chapa completa, RES review y sesion antigua.
        caso('resolucion_remota', resoluciones_remotas_decisiones=[(0, 'NOMENCL')]),
        caso('decisiones_conflicto_resuelto_con_excel', correos_locales=[(0, {'medidas': 'omit'})],
             anulaciones_remotas=[(0, {'espera': '5'})], resolver_decisiones='excel'),
        caso('decisiones_conflicto_resuelto_con_local', correos_locales=[(0, {'medidas': 'omit'})],
             anulaciones_remotas=[(0, {'espera': '5'})], resolver_decisiones='local'),
        caso('mismas_gestiones_distinto_resto_choca', correos_locales=[(0, {'medidas': 'omit'})],
             correos_remotos=[(0, {'medidas': 'omit'})], anulaciones_remotas=[(0, {'espera': '5'})]),
        caso('res_cambiado_en_excel_marca_review', remotas=[(0, 'RES', 'NOMENCL')]),
        caso('celda_tecnica_invalida_mensaje', celda_tecnica=json.dumps({'version': 2})),
        caso('sesion_antigua_sin_linea_base', sesion_antigua=True, anulaciones_remotas=[(0, {'espera': '5'})]),
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
