"""Oráculo de estructura (escenarios 3, 5 y 10 del plan 18-nov-2026): 16 vencimientos de 10 ingresos, uno con cinco fechas
(dos con evidencia asociada, una incorporada sin nomenclatura y dos exigibles), dos NNA en una causa y dos ingresos sucesivos
de la misma persona al mismo programa. Datos ficticios. Python importa el ZIP y guarda su resultado; Rust lo compara.

Uso (desde la raíz del repo): python3 rust/oraculo/generar_paquete_estructura.py
Escribe rust/crates/flow-trabajo/tests/datos/paquete_estructura.{zip,json}.
"""

import json
import sys
from datetime import date
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(RAIZ / 'src'))
sys.path.insert(0, str(RAIZ / 'tests'))
sys.path.insert(0, str(Path(__file__).resolve().parent))

from generar_paquete_integracion import AMB, ING, comunes, consulta  # noqa: E402
from nurus.capture import write_capture  # noqa: E402
from nurus.capture.contract import TABLES  # noqa: E402
from nurus.capture.fixtures import fixture  # noqa: E402
from nurus.personal.capture_input import import_capture  # noqa: E402
from nurus.personal.config import defaults  # noqa: E402

CORTE = date(2026, 10, 8)
# (ingreso, persona, causa, fechas de vencimiento)
INGRESOS = [
    (0, 'p0', 'c0', ['2026-05-20', '2026-06-20', '2026-07-20', '2026-08-20', '2026-09-20']),
    (1, 'p1', 'c1', ['2026-08-01', '2026-09-01']),
    (2, 'p2', 'c1', ['2026-08-05', '2026-09-05']),  # dos NNA en la misma causa
    (3, 'p3', 'c2', ['2026-08-10']),
    (4, 'p3', 'c2', ['2026-09-10']),  # dos ingresos sucesivos de la misma persona al mismo programa
    (5, 'p5', 'c3', ['2026-08-15']),
    (6, 'p6', 'c4', ['2026-08-16']),
    (7, 'p7', 'c5', ['2026-08-17']),
    (8, 'p8', 'c6', ['2026-08-18']),
    (9, 'p9', 'c7', ['2026-08-19']),
]


def historia(base, qid, causa, tramites):
    filas = []
    for n, (desc, fecha, folio, ref) in enumerate(tramites, 1):
        fila = {k: '' for k in TABLES['historia_tramites']}
        fila.update(comunes(base, qid, n))
        fila.update(cause_id=causa, ingreso_id='', person_id='', program_id='', event_id=f'{qid}-{n}', event_at=fecha,
                    time_precision='DIA', time_zone='DESCONOCIDA', scope='CAUSA', event_type='tramite', description=desc,
                    reference=ref, status='CAPTURADO', identity_status='CAUSA', folio=str(folio), stage='Cumplimiento',
                    annex_visible='false',
                    raw_json=json.dumps({'Trámite': 'Escrito' if desc == AMB else 'Resolución'}, ensure_ascii=False))
        filas.append(fila)
    return filas


def main():
    metadata, tablas = fixture(16)
    base = tablas['registros'][0]
    columnas = ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'NOMBRE CENTRO', 'FECHA VENCIMIENTO']  # noqa: F841
    registros = tablas['registros']
    numero = 0
    mapa = {}
    for ingreso, persona, causa, fechas in INGRESOS:
        for fecha in fechas:
            fila = registros[numero]
            fila.update(cause_id=f'causa-{causa}', ingreso_id=f'ingreso-{ingreso}', person_id=f'persona-{persona}',
                        rit_original=f'FICT-{causa}-2026', record_type='INFORMES')
            crudo = {'RIT': fila['rit_original'], 'TRIBUNAL': 'Laja', 'NOMBRE': f'Persona Ficticia {persona}',
                     'RUT': f'ID-{persona}', 'NOMBRE CENTRO': 'PRM Laja', 'FECHA VENCIMIENTO': fecha}
            fila['raw_json'] = fila['normalized_json'] = json.dumps(crudo, ensure_ascii=False)
            mapa[(ingreso, fecha)] = fila
            numero += 1
    assert numero == 16
    tablas['bitacoras'] = []
    metadata['table_states']['bitacoras'] = 'VACIA_COMPROBADA'
    # Historia por causa. Causa c0: D1 escrito+ingreso (fecha 1), D3 escrito incorporado sin ingreso posterior (fecha 3).
    plan = {
        'c0': [(AMB, '2026-06-01', 10, 'D1'), (ING, '2026-06-05', 11, 'D1'), (AMB, '2026-07-25', 12, 'D3')],
        'c1': [(AMB, '2026-08-20', 20, 'D4')],
        'c2': [(ING, '2026-09-20', 30, 'D5')],
    }
    for i, (causa, tramites) in enumerate(plan.items(), 1):
        qid = f'hist-{i}'
        tablas['consultas'].append(consulta(base, qid, 'historia_tramites', {
            'historia_causa': {'source_system': 'RUS', 'court_code': base['court_code'], 'cause_id': f'causa-{causa}'},
            'ventana': {'criterio': 'ventana_4_meses', 'desde': '2026-05-01', 'hasta': '2026-10-08'}}))
        tablas.setdefault('historia_tramites', []).extend(historia(base, qid, f'causa-{causa}', tramites))
    metadata['table_states']['historia_tramites'] = 'COMPLETA'
    # Escritos de la causa c0 con asociación explícita a ingreso y fecha proyectada.
    tablas['consultas'].append(consulta(base, 'es-1', 'escritos', {'escritos_causa': {'source_system': 'RUS', 'court_code': base['court_code'], 'cause_id': 'causa-c0'}}))
    escritos = [('D1', 'Resuelto', '2026-05-20'), ('D2', 'Pendiente', '2026-06-20'), ('D3', 'Resuelto', '2026-07-20')]
    tablas['escritos'] = [
        {**comunes(base, 'es-1', n), 'cause_id': 'causa-c0', 'ingreso_id': '', 'person_id': '', 'escrito_id': f'es-{n}',
         'scope': 'CAUSA', 'doc': d, 'escrito_type': AMB, 'requester': 'Programa', 'escrito_status': e, 'reference': '',
         'linked_ingreso_id': 'ingreso-0', 'linked_projection': f, 'identity_status': 'CAUSA', 'raw_json': '{}'}
        for n, (d, e, f) in enumerate(escritos, 1)]
    metadata['table_states']['escritos'] = 'COMPLETA'
    metadata['queries'] = tablas['consultas']
    destino = RAIZ / 'rust/crates/flow-trabajo/tests/datos'
    paquete = destino / 'paquete_estructura.zip'
    paquete.unlink(missing_ok=True)
    write_capture(paquete, metadata, tablas)
    trabajo = import_capture(paquete, defaults(), as_of=CORTE)
    filas = [{'id': r.id, 'observacion': r.observation, 'reglas': list(r.rules), 'acciones': list(r.actions),
              'marcas': [{'nivel': n, 'codigo': c, 'texto': t} for n, c, t in r.avisos],
              'otros_programas': list(r.otros_programas)} for r in trabajo.rows]
    (destino / 'paquete_estructura.json').write_text(
        json.dumps({'corte': CORTE.isoformat(), 'filas': filas}, ensure_ascii=False, indent=1), encoding='utf-8')
    for (ingreso, fecha), reg in mapa.items():
        f = next(x for x in trabajo.rows if x.id == reg['record_id'] or x.source_row == int(reg['original_row']))
        print(ingreso, fecha, f.actions, [r.split('.')[-1] for r in f.rules])


if __name__ == '__main__':
    main()
