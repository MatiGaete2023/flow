"""E-A2 · Oráculo de la conciliación entre capturas de Flow Python (`capture_reconciliation.reconcile`).

Arma con Python paquetes CSMP_CAPTURE desde los lotes sintéticos de `paquete.json` y `resultados.json`, abre uno, le
hace ediciones, abre otro con `previous=` y graba qué se conserva, qué conflictos aparecen y qué cambios muestra la
comparación. Las filas se nombran por su identidad remota (tribunal-causa-ingreso), igual que en la prueba Rust.

Uso (desde la raíz): python app/qa/generar_conciliacion.py [salida.json]   (por omisión app/tests/datos/conciliacion.json)
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
from nurus.personal.capture_reconciliation import ReconciliationRequired  # noqa: E402
from nurus.personal.config import defaults  # noqa: E402
from nurus.personal.record_edits import apply  # noqa: E402

CORTE = date(2026, 10, 8)
DATOS = RAIZ / 'app' / 'tests' / 'datos'


def reconstruir(carpeta, archivos):
    for ruta, contenido in archivos.items():
        destino = carpeta / ruta
        destino.parent.mkdir(parents=True, exist_ok=True)
        destino.write_bytes(base64.b64decode(contenido))


def programa_cambiado(lote):
    """Copia del lote de listados con otro programa para la persona de la página 2: cambia un dato fuente y el vínculo
    con el ingreso se conserva (la huella y el vínculo se actualizan en la verificación)."""
    archivos = {n: base64.b64decode(c) for n, c in lote['archivos'].items()}
    pagina = 'CUMP Ambulatorio Jdo Familia Tome 2.xls'
    archivos[pagina] = archivos[pagina].replace(b'PRM Centro Ficticio', b'PRM Centro Cambiado')
    verificacion = json.loads(archivos['verificacion 001.json'])
    for item in verificacion['archivos']:
        if item['pagina'] == 2:
            item['sha256'] = hashlib.sha256(archivos[pagina]).hexdigest()
            item['bytes'] = len(archivos[pagina])
            for vinculo in item['vinculos_ingreso']:
                vinculo['programa'] = 'PRM Centro Cambiado'
                vinculo['calendario'] = ['PRM Centro Cambiado' if x == 'PRM Centro Ficticio' else x for x in vinculo['calendario']]
    archivos['verificacion 001.json'] = json.dumps(verificacion, indent=2, ensure_ascii=False).encode('utf-8')
    return {'carpeta': lote['carpeta'], 'archivos': {n: base64.b64encode(c).decode() for n, c in sorted(archivos.items())}}


def etiqueta(trabajo, fila):
    """Identidad remota con su modalidad; sin identidad, el archivo y la fila físicos."""
    o = trabajo.capture_sources[fila.id]
    if o['identity_status'] == 'VERIFICADA' and all(o[k] for k in ('court_code', 'cause_id', 'ingreso_id')):
        return f"{o['court_code']}-{o['cause_id']}-{o['ingreso_id']}|{o['record_type']}|{o['modality_code']}"
    return f"fisica|{o['original_file']}#{o['original_row']}|{o['record_type']}"


def paquete(base, nombre, lotes):
    carpeta = base / nombre
    rutas = []
    for lote in lotes:
        c = carpeta / lote['carpeta']
        reconstruir(c, lote['archivos'])
        rutas.append(c)
    ruta, _ = export_capture_with_note(load_sources(rutas, False), carpeta / 'captura.zip', diaries=None,
                                       match_diaries=False, consolidated=False, meses=4)
    return ruta


def abrir(ruta, previo=None, respuestas=None):
    return import_capture(ruta, defaults(), as_of=CORTE, previous=previo, resolutions=respuestas)


def revision(trabajo):
    return {etiqueta(trabajo, f): {'OBSERVACION': str(f.review.get('OBSERVACION', '')), 'RES': str(f.review.get('RES', ''))}
            for f in trabajo.rows}


def escenario(nombre, rutas, previo, nuevo, ediciones=(), respuestas=None):
    anterior = abrir(rutas[previo])
    por_etiqueta = {etiqueta(anterior, f): f for f in anterior.rows}
    for etq, campo, valor in ediciones:
        fila = por_etiqueta[etq]
        if campo == 'OBSERVACION':
            apply(anterior, [fila.id], review={'OBSERVACION': valor})
        else:
            apply(anterior, [fila.id], decisions={'resolution': valor})
    por_id_previo = {f.id: etiqueta(anterior, f) for f in anterior.rows}
    candidato = {f.id: f for f in anterior.rows}
    _ = candidato
    nueva_ruta = rutas[nuevo]
    # Las respuestas se nombran por etiqueta; para Python hace falta el id de la fila nueva (o «missing:» + id anterior).
    sondeo = abrir(nueva_ruta)
    ids_nuevos = {etiqueta(sondeo, f): f.id for f in sondeo.rows}
    ids = {}
    for clave, respuesta in (respuestas or {}).items():
        if clave.startswith('missing:'):
            ids_previos = {v: k for k, v in por_id_previo.items()}
            ids['missing:' + ids_previos.get(clave[8:], clave[8:])] = respuesta
        else:
            ids[ids_nuevos.get(clave, clave)] = respuesta
    salida = {'escenario': nombre, 'previo': previo, 'nuevo': nuevo, 'ediciones': [list(e) for e in ediciones],
              'respuestas': dict(respuestas or {})}
    try:
        trabajo = abrir(nueva_ruta, anterior, ids)
    except ReconciliationRequired as error:
        etiquetas_nuevas = {v: k for k, v in ids_nuevos.items()}
        _ = etiquetas_nuevas
        salida['resultado'] = 'conflictos'
        salida['conflictos'] = sorted(
            [
                (('missing:' + por_id_previo[c.previous_ids[0]]) if c.record_id.startswith('missing:')
                 else next(k for k, v in ids_nuevos.items() if v == c.record_id)),
                c.reason, bool(c.can_keep), sorted(c.changed_fields),
            ]
            for c in error.conflicts
        )
        return salida
    except ValueError as error:
        salida['resultado'] = 'error'
        salida['error'] = str(error)
        return salida
    salida['resultado'] = 'ok'
    if previo == nuevo:
        # Mismos bytes: Python reutiliza el trabajo anterior con sus ediciones (sin pasar por `reconcile`).
        salida['continuada'] = True
        salida['emparejadas'] = sorted(etiqueta(trabajo, f) for f in trabajo.rows)
    else:
        reconciliacion = trabajo.capture_reconciliation
        salida['continuada'] = bool(reconciliacion['continued'])
        nuevas = set(reconciliacion['id_map'].values())  # id_map: fila anterior -> fila nueva
        salida['emparejadas'] = sorted(etiqueta(trabajo, f) for f in trabajo.rows if f.id in nuevas)
    salida['revision'] = revision(trabajo)
    cambios = trabajo.capture_changes or {}
    salida['cambios'] = {etiqueta(trabajo, f): cambios['filas'][f.id]['tipo'] for f in trabajo.rows if f.id in cambios.get('filas', {})}
    salida['campos'] = {etiqueta(trabajo, f): cambios['filas'][f.id]['campos'] for f in trabajo.rows
                        if cambios.get('filas', {}).get(f.id, {}).get('campos')}
    salida['egresados'] = sorted(por_id_previo[e['id_anterior']] for e in cambios.get('egresados', []))
    return salida


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else DATOS / 'conciliacion.json'
    paquetes = {e['escenario']: e for e in json.loads((DATOS / 'paquete.json').read_text(encoding='utf-8'))['escenarios']}
    modificado = json.loads((DATOS / 'resultados.json').read_text(encoding='utf-8'))['modificado']
    lotes = {
        'listados': paquetes['listados']['lote'],
        'listados_dos_pestanas': paquetes['listados_dos_pestanas']['lote'],
        'modificado': modificado,
        'programa_cambiado': programa_cambiado(paquetes['listados']['lote']),
    }
    with tempfile.TemporaryDirectory() as t:
        base = Path(t)
        rutas = {n: paquete(base, n, [lote]) for n, lote in lotes.items()}
        # Etiquetas de las personas del lote «listados»: se leen del trabajo, no se escriben a mano.
        trabajo = abrir(rutas['listados'])
        e1, e2 = sorted(etiqueta(trabajo, f) for f in trabajo.rows)
        dos = abrir(rutas['listados_dos_pestanas'])
        solo_dos = sorted(etiqueta(dos, f) for f in dos.rows if etiqueta(dos, f) not in {etiqueta(trabajo, f) for f in trabajo.rows}
                          and f.id and dos.capture_sources[f.id]['record_type'] == 'CUMPLIMIENTO')
        casos = [
            escenario('mismo_paquete_con_ediciones', rutas, 'listados', 'listados',
                      [(e1, 'OBSERVACION', 'Observación local uno'), (e2, 'RES', 'PC_IE')]),
            escenario('identidad_perdida_sin_ediciones', rutas, 'listados', 'modificado'),
            escenario('identidad_perdida_con_edicion', rutas, 'listados', 'modificado', [(e2, 'OBSERVACION', 'Observación local dos')]),
            escenario('identidad_perdida_reiniciada', rutas, 'listados', 'modificado', [(e2, 'OBSERVACION', 'Observación local dos')],
                      {'missing:' + e2: 'RESET'}),
            escenario('fuente_cambiada_sin_ediciones', rutas, 'listados', 'programa_cambiado'),
            escenario('edicion_en_fila_sin_cambios', rutas, 'listados', 'programa_cambiado', [(e1, 'OBSERVACION', 'Observación local uno')]),
            escenario('fuente_cambiada_con_edicion', rutas, 'listados', 'programa_cambiado', [(e2, 'OBSERVACION', 'Observación local dos')]),
            escenario('conflicto_conservar', rutas, 'listados', 'programa_cambiado', [(e2, 'OBSERVACION', 'Observación local dos')],
                      {e2: 'KEEP'}),
            escenario('conflicto_reiniciar', rutas, 'listados', 'programa_cambiado', [(e2, 'OBSERVACION', 'Observación local dos'), (e2, 'RES', 'PC_IE')],
                      {e2: 'RESET'}),
            escenario('respuesta_ajena', rutas, 'listados', 'listados', [(e1, 'OBSERVACION', 'Observación local uno')],
                      {e2: 'RESET'}),
            escenario('faltante_editado', rutas, 'listados_dos_pestanas', 'listados', [(solo_dos[0], 'OBSERVACION', 'Observación local')]),
            escenario('faltante_reiniciado', rutas, 'listados_dos_pestanas', 'listados', [(solo_dos[0], 'OBSERVACION', 'Observación local')],
                      {'missing:' + solo_dos[0]: 'RESET'}),
        ]
    texto = json.dumps({'corte': CORTE.isoformat(), 'lote_programa_cambiado': lotes['programa_cambiado'], 'casos': casos},
                       ensure_ascii=False, indent=1)
    destino.write_bytes((texto + '\n').encode('utf-8'))
    for c in casos:
        print(c['escenario'], c['resultado'], c.get('conflictos') or c.get('error') or '')
    print(f'{len(casos)} casos en {destino}')


if __name__ == '__main__':
    main()
