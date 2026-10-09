"""Cobertura por fuente e identidad; un agregado nunca sustituye el alcance."""
from collections import defaultdict
from collections.abc import Iterable, Mapping
import json
from typing import TypeAlias

Identity: TypeAlias = tuple[str, str, str, str]
CoverageIndex: TypeAlias = dict[tuple[str, Identity], str]
ADEQUATE = frozenset(('COMPLETA', 'VACIA_COMPROBADA'))
ABSENT = frozenset(('NO_SOLICITADA', 'NO_CONSULTADA', 'NO_DISPONIBLE', 'FALLIDA', 'CANCELADA'))
SOURCES = ('bitacoras', 'historia_tramites', 'calendarios', 'documentos')
SCREEN_TABLES = {'seguimiento': ('registros',), 'carga': ('registros',),
                 'calendario_informes': ('registros', 'calendarios'),
                 'bitacoras': ('bitacoras',), 'historia_tramites': ('historia_tramites',),
                 'documentos': ('documentos',)}


def remote_identity(row: Mapping[str, str]) -> Identity:
    return tuple(row.get(k, '') for k in ('source_system', 'court_code', 'cause_id', 'ingreso_id'))


def aggregate(states: Iterable[str]) -> str:
    values = list(states)
    if not values:
        return 'NO_SOLICITADA'
    if all(value == 'VACIA_COMPROBADA' for value in values):
        return 'VACIA_COMPROBADA'
    if all(value in ADEQUATE for value in values):
        return 'COMPLETA'
    if len(set(values)) == 1:
        return values[0]
    return 'PARCIAL'


def query_tables(query: Mapping[str, str]) -> tuple[str, ...]:
    filters = json.loads(query['filters_json'])
    declared = filters.get('capture_tables')
    if declared is not None:
        return tuple(declared)
    return SCREEN_TABLES.get(query['screen'], ())


def validate_coverage(manifest, tables, fail) -> None:
    queries = {q['query_id']: q for q in manifest['queries']}
    for q in queries.values():
        filters = json.loads(q['filters_json'])
        declared = filters.get('capture_tables')
        if declared is not None and (not isinstance(declared, list) or not declared or
                                    any(not isinstance(t, str) or t not in SOURCES + ('registros',) for t in declared) or
                                    len(set(declared)) != len(declared)):
            fail('Alcance de tablas de consulta inválido.')
        scope = filters.get('remote_identity')
        if scope is not None:
            if not isinstance(scope, dict) or set(scope) != {'source_system', 'court_code', 'cause_id', 'ingreso_id'}:
                fail('Identidad de alcance de consulta inválida.')
            if any(not isinstance(v, str) or not v for v in scope.values()):
                fail('Identidad de alcance incompleta.')
            if scope['source_system'] != q['source_system'] or scope['court_code'] != q['court_code']:
                fail('Identidad de alcance de otra consulta.')
    for table in ('registros', *SOURCES):
        rows = tables.get(table, ())
        qids = {row['query_id'] for row in rows}
        qids.update(q['query_id'] for q in queries.values() if table in query_tables(q))
        for row in rows:
            q = queries[row['query_id']]
            scope = json.loads(q['filters_json']).get('remote_identity')
            if scope and remote_identity(row) != remote_identity(scope):
                fail('Fila fuera de la identidad consultada.')
            if row['coverage'] == 'VACIA_COMPROBADA':
                fail('Fila presente declarada vacía.')
            if row['coverage'] in ADEQUATE and q['coverage'] not in ADEQUATE:
                fail('Fila declara mayor cobertura que su consulta.')
            if q['coverage'] == 'VACIA_COMPROBADA':
                fail('Consulta vacía con filas de datos.')
        declared = manifest['table_states'][table]
        states = [queries[qid]['coverage'] for qid in qids] + [row['coverage'] for row in rows]
        if declared in ADEQUATE and any(state not in ADEQUATE for state in states):
            fail('Estado de tabla declara mayor cobertura que sus consultas o filas.')
        if declared in ABSENT and rows:
            fail('Tabla no disponible declarada con filas.')


def build_index(manifest, tables) -> CoverageIndex:
    queries = {q['query_id']: q for q in manifest['queries']}
    states = defaultdict(list)
    for source in SOURCES:
        for row in tables.get(source, ()):
            key = remote_identity(row)
            verified = row.get('identity_status') == 'VERIFICADA' or row.get('association') == 'VERIFICADA'
            if all(key) and verified:
                states[source, key].extend((row['coverage'], queries[row['query_id']]['coverage']))
        for q in queries.values():
            scope = json.loads(q['filters_json']).get('remote_identity')
            if scope and source in query_tables(q):
                states[source, remote_identity(scope)].append(q['coverage'])
    return {key: aggregate(values) for key, values in states.items()}


def record_coverages(manifest, tables, records):
    index = build_index(manifest, tables)
    result = {}
    for rid, record in records.items():
        states = {}
        for source in SOURCES:
            fallback = manifest['table_states'][source]
            if fallback not in ABSENT:
                fallback = 'NO_CONSULTADA'
            states[source] = index.get((source, remote_identity(record)), fallback)
        result[rid] = states
    return result
