"""Oráculo del paquete de la integración 18-nov-2026: ZIP 1.2.0 ficticio con Ingreso Histórico Protección, Escritos de la
causa y asociación explícita de informes con su proyección. Lo importa el Asistente Python y guarda lo que propone,
para que Rust lo compare.

Uso (desde la raíz del repo):
    python3 rust/oraculo/generar_paquete_integracion.py

Escribe rust/crates/flow-trabajo/tests/datos/paquete_integracion.zip y paquete_integracion.json.
"""

import json
import sys
from datetime import date
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(RAIZ / 'src'))
sys.path.insert(0, str(RAIZ / 'tests'))

from historia_fixtures import captura_con_historia  # noqa: E402
from nurus.capture import write_capture  # noqa: E402
from nurus.capture.contract import COMMON, TABLES  # noqa: E402
from nurus.personal.capture_input import import_capture  # noqa: E402
from nurus.personal.config import defaults  # noqa: E402

CORTE = date(2026, 10, 8)
AMB = 'Inf. Cump Programa Ambulatorio'
ING = 'Ingreso Informe de Cumplimiento X'


def consulta(base, qid, pantalla, filtros):
    q = {k: '' for k in TABLES['consultas']}
    q.update(query_id=qid, source_system='RUS', screen=pantalla, tab=pantalla.upper(), court_code=base['court_code'],
             modality_code=base['modality_code'], court_name=base['court_name'], coverage='COMPLETA',
             captured_at=base['captured_at'], filters_json=json.dumps(filtros), expected_pages='1', saved_pages='1',
             time_zone='America/Santiago', history_complete='false', institutional_verified='false')
    return q


def comunes(base, qid, n):
    c = {k: base[k] for k in COMMON if k in base}
    c.update(query_id=qid, coverage='COMPLETA', original_row=str(n))
    return c


def main():
    # Causa 1 = registros 2 y 3 (ambos INFORMES de la misma causa: la historia no se atribuye sin vínculo explícito).
    tramites = [(AMB, '2026-09-01', 30), (ING, '2026-09-10', 31), (AMB, '2026-09-20', 32)]
    metadata, tablas = captura_con_historia(tramites, causa='causa-ficticia-1', registros=4)
    for fila, doc in zip(tablas['historia_tramites'], ('D-ESC', 'D-ESC', 'D-OTRO')):
        fila['reference'] = doc
        fila['raw_json'] = json.dumps({'Trámite': 'Escrito' if fila['description'] == AMB else 'Resolución'}, ensure_ascii=False)
    vencimientos = {2: '2026-08-20', 3: '2026-08-25'}
    for fila in tablas['registros']:
        numero = int(fila['record_id'][1:])
        if numero in vencimientos:
            crudo = {'RIT': fila['rit_original'], 'TRIBUNAL': 'Laja', 'NOMBRE': f'Persona Ficticia {numero}', 'RUT': f'ID-{numero}',
                     'NOMBRE CENTRO': 'PRM Laja', 'FECHA VENCIMIENTO': vencimientos[numero]}
            fila['record_type'] = 'INFORMES'
            fila['raw_json'] = fila['normalized_json'] = json.dumps(crudo, ensure_ascii=False)
        else:
            # Registros de Espera con programa propio para que «otro programa» se distinga por nombre completo.
            crudo = json.loads(fila['normalized_json'])
            crudo['DERIVACION'] = 'PRM Laja'
            fila['raw_json'] = fila['normalized_json'] = json.dumps(crudo, ensure_ascii=False)
    base = tablas['registros'][0]
    q_h = consulta(base, 'hp-1', 'historico_proteccion', {'historico_persona': {'source_system': 'RUS', 'court_code': base['court_code'], 'person_id': base['person_id']}})
    q_e = consulta(base, 'es-1', 'escritos', {'escritos_causa': {'source_system': 'RUS', 'court_code': base['court_code'], 'cause_id': 'causa-ficticia-1'}})
    tablas['consultas'] += [q_h, q_e]
    filas_h = [('PRM Laja', '--', 'SIN INFORMACION'), ('FAE Biobío', '--', 'SIN INFORMACION'), ('PDE Mulchén', '01/05/2026', 'EGRESO')]
    tablas['historico_proteccion'] = [
        {**comunes(base, 'hp-1', n), 'historico_id': f'hp-{n}', 'program_name': p, 'egreso_date': e, 'egreso_cause': c,
         'ownership': 'VERIFICADA', 'raw_json': '{}'} for n, (p, e, c) in enumerate(filas_h, 1)]
    r2 = next(r for r in tablas['registros'] if r['record_id'] == 'r2')
    r3 = next(r for r in tablas['registros'] if r['record_id'] == 'r3')
    escritos = [
        # D-PEND: escrito pendiente asociado al ingreso 2 y a su proyección (20-ago): no se pide cuenta de él.
        ('D-PEND', AMB, 'Pendiente', r2['ingreso_id'], '2026-08-20'),
        # D-ESC: escrito y resolución (folios 30 y 31) de OTRA proyección del ingreso 2 (20-jul): sus folios no se atribuyen
        # ni al 20-ago de r2 ni a r3 por la vía genérica (E3-10).
        ('D-ESC', AMB, 'Resuelto', r2['ingreso_id'], '2026-07-20'),
        # D-OTRO: escrito ya incorporado a Historia (folio 32), asociado al ingreso 3 (25-ago): proyecto de nomenclatura.
        ('D-OTRO', AMB, 'Resuelto', r3['ingreso_id'], '2026-08-25'),
    ]
    tablas['escritos'] = [
        {**comunes(base, 'es-1', n), 'cause_id': 'causa-ficticia-1', 'ingreso_id': '', 'person_id': '', 'escrito_id': f'es-{n}',
         'scope': 'CAUSA', 'doc': d, 'escrito_type': t, 'requester': 'Programa', 'escrito_status': s, 'reference': '',
         'linked_ingreso_id': i, 'linked_projection': p, 'identity_status': 'CAUSA', 'raw_json': '{}'}
        for n, (d, t, s, i, p) in enumerate(escritos, 1)]
    metadata['table_states']['historico_proteccion'] = 'COMPLETA'
    metadata['table_states']['escritos'] = 'COMPLETA'
    metadata['queries'] = tablas['consultas']
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'rust/crates/flow-trabajo/tests/datos'
    destino.mkdir(parents=True, exist_ok=True)
    paquete = destino / 'paquete_integracion.zip'
    paquete.unlink(missing_ok=True)
    write_capture(paquete, metadata, tablas)
    work = import_capture(paquete, defaults(), as_of=CORTE)
    filas = [
        {'id': r.id, 'observacion': r.observation, 'reglas': list(r.rules), 'acciones': list(r.actions),
         'marcas': [{'nivel': n, 'codigo': c, 'texto': t} for n, c, t in r.avisos],
         'otros_programas': list(r.otros_programas)}
        for r in work.rows
    ]
    (destino / 'paquete_integracion.json').write_text(
        json.dumps({'corte': CORTE.isoformat(), 'filas': filas}, ensure_ascii=False, indent=1), encoding='utf-8', newline='\n')
    for f in filas:
        print(f['id'], '|', f['observacion'][:170], '|', f['acciones'])


if __name__ == '__main__':
    main()
