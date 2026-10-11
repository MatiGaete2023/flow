"""Datos sintéticos compartidos por herramientas y verificaciones congeladas."""

from hashlib import sha256
import json
from .contract import COMMON, TABLES


def fixture(count=4, partial=False):
    captured = '2026-10-08T15:00:00-03:00'
    capture_id = f'ficticia-20261008-{count}-{int(partial)}'
    q = {k: '' for k in TABLES['consultas']}
    q.update(
        query_id='q1',
        source_system='RUS',
        screen='seguimiento',
        tab='Espera',
        court_code='FICTICIO',
        court_name='Laja',
        modality_code='1',
        coverage='COMPLETA',
        captured_at=captured,
        filters_json='{}',
        expected_pages='1',
        saved_pages='1',
        time_zone='America/Santiago',
        history_complete='false',
        institutional_verified='false',
    )
    tables = {'consultas': [q], 'registros': [], 'incidencias': []}
    states = {k: 'NO_SOLICITADA' for k in TABLES}
    states.update(
        consultas='COMPLETA',
        registros='COMPLETA',
        incidencias='VACIA_COMPROBADA',
        bitacoras='NO_CONSULTADA' if partial else 'COMPLETA',
    )
    if not partial:
        tables['bitacoras'] = []
    for number in range(count):
        # Los cuatro códigos de modalidad tienen consultas independientes.
        code = str(number % 4 + 1)
        qid = 'q' + code
        if not any(query['query_id'] == qid for query in tables['consultas']):
            tables['consultas'].append({**q, 'query_id': qid, 'modality_code': code})
        common = {k: '' for k in COMMON}
        common.update(
            capture_id=capture_id,
            query_id=qid,
            source_system='RUS',
            court_code='FICTICIO',
            court_name='Laja',
            rit_original=f'FICT-{number}-2026',
            cause_id=f'causa-ficticia-{number // 2}',
            ingreso_id=f'ingreso-ficticio-{number}',
            person_id=f'persona-ficticia-{number}',
            program_id='programa-ficticio',
            modality_code=code,
            modality_raw=code,
            tab='Espera',
            captured_at=captured,
            original_file='fuente-ficticia.xls',
            original_sheet='Hoja1',
            original_row=str(number + 2),
            source_sha256=sha256(b'fuente exclusivamente ficticia').hexdigest(),
            coverage='COMPLETA',
        )
        raw = {
            'RIT': common['rit_original'],
            'RUT': f'ID-FICTICIO-{number}',
            'NOMBRE': f'Persona Ficticia {number}',
            'DERIVACION': 'PRM FICTICIO',
            'TRIBUNAL': 'Laja',
            'T ESPERA': '40',
            'FEC. RESOLUCIÓN': '2026-08-01',
            'MODALIDAD': {'1': 'Residencial', '2': 'Ambulatorio', '3': 'FAE', '4': 'DCE'}[code],
        }
        tables['registros'].append(
            {
                **common,
                'record_id': f'r{number}',
                'record_type': 'ESPERA',
                'identity_status': 'VERIFICADA',
                'raw_json': json.dumps(raw, ensure_ascii=False),
                'normalized_json': json.dumps(raw, ensure_ascii=False),
            }
        )
        if not partial:
            tables['bitacoras'].append(
                {
                    **common,
                    'event_id': f'e{number}',
                    'event_at': '2026-09-10',
                    'time_precision': 'DIA',
                    'time_zone': 'DESCONOCIDA',
                    'origin': 'CENTRO',
                    'origin_verified': 'true',
                    'author': 'Usuario ficticio',
                    'event_type': 'Observación',
                    'text': 'Texto de prueba sintético, sin datos institucionales.',
                    'response': '',
                    'response_at': '',
                    'identity_status': 'VERIFICADA',
                }
            )
    metadata = {
        'capture_id': capture_id,
        'captured_at': captured,
        'downloader': {'version': '2.7.2', 'commit': 'FIXTURE_SINTETICA'},
        'contains_sensitive_data': False,
        'originals_authorized': False,
        'queries': tables['consultas'],
        'table_states': states,
    }
    return metadata, tables
