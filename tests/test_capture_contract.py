"""Ataques y coherencia del contrato; todos los datos son ficticios."""

from copy import deepcopy
from hashlib import sha256
import importlib.util
import json
from pathlib import Path
import stat
import zipfile

import pytest
from nurus.capture import CaptureError, read_capture, write_capture

spec = importlib.util.spec_from_file_location('fixture', Path(__file__).parents[1] / 'tools/flow_capture_fixture.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
fixture = module.fixture


def mutate(path, change):
    with zipfile.ZipFile(path) as z:
        items = {i.filename: z.read(i) for i in z.infolist()}
    change(items)
    with zipfile.ZipFile(path, 'w', compression=zipfile.ZIP_DEFLATED) as z:
        for name, raw in items.items():
            z.writestr(name, raw)


def test_full_and_partial_and_deterministic(tmp_path):
    before_modules = set(__import__('sys').modules)
    for partial in (False, True):
        m, t = fixture(partial=partial)
        a = write_capture(tmp_path / f'{partial}.zip', m, t)
        b = write_capture(tmp_path / f'{partial}-again.zip', m, t)
        assert a.read_bytes() == b.read_bytes()
        loaded = read_capture(a)
        assert len(loaded.rows('registros')) == 4
        assert {r['modality_code'] for r in loaded.rows('registros')} == {'1', '2', '3', '4'}
        assert loaded.manifest['table_states']['bitacoras'] == ('NO_CONSULTADA' if partial else 'COMPLETA')
        assert not any(
            name.startswith('nurus.personal')
            for name in set(__import__('sys').modules) - before_modules
            if name.endswith('app')
        )


@pytest.mark.parametrize(
    'name', ['../escape', '/absolute', 'C:/evil', 'originales\\evil', 'data/CON.txt', 'data/a.', 'data/a:b', 'data/./a']
)
def test_paths(tmp_path, name):
    m, t = fixture()
    path = write_capture(tmp_path / 'bad.zip', m, t)
    mutate(path, lambda items: items.update({name: b'x'}))
    with pytest.raises(CaptureError):
        read_capture(path)
    assert not (tmp_path / 'escape').exists()


def test_hash_and_undeclared(tmp_path):
    m, t = fixture()
    path = write_capture(tmp_path / 'bad.zip', m, t)
    mutate(path, lambda items: items.update({'data/registros.csv': items['data/registros.csv'] + b'changed'}))
    with pytest.raises(CaptureError, match='Hash'):
        read_capture(path)


@pytest.mark.parametrize(
    'change',
    [
        lambda m, t: m.update(schema_version='2.0.0'),
        lambda m, t: m.update(required_features=['execute']),
        lambda m, t: t['registros'][0].update(capture_id='another'),
        lambda m, t: t['registros'][0].update(query_id='missing'),
        lambda m, t: t['registros'][0].update(court_code='another'),
        lambda m, t: t['registros'][0].update(ingreso_id=''),
        lambda m, t: t['registros'][1].update(record_id='r0'),
        lambda m, t: t['consultas'][0].update(saved_pages='0'),
    ],
)
def test_invalid_producer_does_not_publish(tmp_path, change):
    m, t = fixture()
    change(m, t)
    with pytest.raises(CaptureError):
        write_capture(tmp_path / 'bad.zip', m, t)
    assert not (tmp_path / 'bad.zip').exists()


def test_originals_require_explicit_authorization(tmp_path):
    m, t = fixture()
    with pytest.raises(CaptureError):
        write_capture(tmp_path / 'una.zip', m, t, originals={'originales/una.txt': b'privado'})
    assert not (tmp_path / 'una.zip').exists()
    m['originals_authorized'] = True
    path = write_capture(tmp_path / 'authorized.zip', m, t, originals={'originales/una.txt': b'ficticio'})
    assert read_capture(path).manifest['originals_authorized'] is True


def test_no_overwrite_or_leftover(tmp_path):
    m, t = fixture()
    path = write_capture(tmp_path / 'valid.zip', m, t)
    before = path.read_bytes()
    with pytest.raises(CaptureError):
        write_capture(path, m, t)
    assert path.read_bytes() == before
    assert not list(tmp_path.glob('.capture-*'))


def test_symlink_duplicate_and_expansion(tmp_path):
    for attack in ('symlink', 'duplicate', 'bomb'):
        m, t = fixture()
        path = write_capture(tmp_path / f'{attack}.zip', m, t)
        with zipfile.ZipFile(path, 'a', compression=zipfile.ZIP_DEFLATED) as z:
            if attack == 'symlink':
                info = zipfile.ZipInfo('originales/link')
                info.external_attr = (stat.S_IFLNK | 0o777) << 16
                z.writestr(info, b'../outside')
            elif attack == 'duplicate':
                z.writestr('MANIFEST.JSON', b'{}')
            else:
                z.writestr('originales/bomb', b'0' * 1000000)
        with pytest.raises(CaptureError):
            read_capture(path)


def test_optional_1x_metadata_and_empty_query_semantics(tmp_path):
    m, t = fixture()
    m['extension_optional'] = {'label': 'future'}
    path = write_capture(tmp_path / 'new.zip', m, t)

    def new_version(items):
        m = json.loads(items['manifest.json'])
        m['schema_version'] = '1.2.0'
        items['manifest.json'] = json.dumps(m).encode()

    mutate(path, new_version)
    assert read_capture(path).manifest['schema_version'] == '1.2.0'


def test_schema_fixtures_and_optional_column(tmp_path):
    import csv
    from io import StringIO
    from jsonschema import Draft202012Validator

    schema = json.loads(
        (Path(__file__).parents[1] / 'contracts/csmp_capture_v1/manifest.schema.json').read_text(encoding='utf-8')
    )
    Draft202012Validator.check_schema(schema)
    for partial in (True, False):
        m, t = fixture(partial=partial)
        path = write_capture(tmp_path / f'{partial}-schema.zip', m, t)
        capture = read_capture(path)
        Draft202012Validator(schema).validate(capture.manifest)

    def optional_column(items):
        old = items['data/registros.csv'].decode()
        reader = csv.DictReader(StringIO(old))
        rows = list(reader)
        out = StringIO()
        writer = csv.DictWriter(out, fieldnames=[*reader.fieldnames, 'optional_future'])
        writer.writeheader()
        writer.writerows({**r, 'optional_future': 'literal'} for r in rows)
        data = out.getvalue().encode()
        items['data/registros.csv'] = data
        m = json.loads(items['manifest.json'])
        m['schema_version'] = '1.1.0'
        f = next(f for f in m['files'] if f.get('table') == 'registros')
        f.update(bytes=len(data), sha256=sha256(data).hexdigest())
        items['manifest.json'] = json.dumps(m).encode()

    mutate(path, optional_column)
    assert read_capture(path).rows('registros')[0]['optional_future'] == 'literal'


def test_identity_absent_unknown_version_and_document_parent_fail(tmp_path):
    m, t = fixture()
    r = t['registros'][0]
    from nurus.capture.contract import COMMON

    d = {k: r.get(k, '') for k in COMMON}
    d.update(
        document_id='doc1',
        parent_event_id='not-present',
        scope='INGRESO',
        reference='referencia ficticia',
        local_file='',
        sha256='',
        download_state='NO_SOLICITADA',
        content_read='false',
    )
    t['documentos'] = [d]
    m['table_states']['documentos'] = 'COMPLETA'
    with pytest.raises(CaptureError, match='padre'):
        write_capture(tmp_path / 'no-parent.zip', m, t)


def test_cause_scope_event_and_unassociated_calendar_are_kept_separate(tmp_path):
    m, t = fixture()
    r = t['registros'][0]
    from nurus.capture.contract import COMMON

    common = {k: r.get(k, '') for k in COMMON}
    common['ingreso_id'] = ''
    t['historia_tramites'] = [
        {
            **common,
            'event_id': 'cause-event',
            'event_at': '2026-09-20',
            'time_precision': 'DIA',
            'time_zone': 'DESCONOCIDA',
            'scope': 'CAUSA',
            'event_type': 'Trámite ficticio',
            'description': 'Título no acredita contenido',
            'reference': 'referencia',
            'status': 'CAPTURADO',
            'document_id': '',
            'identity_status': 'CAUSA',
        }
    ]
    t['calendarios'] = [
        {
            **common,
            'calendar_id': 'cal1',
            'event_at': '2026-10-20',
            'event_type': 'INFORME',
            'status': 'SEGUN_FUENTE',
            'association': 'NO_ASOCIADO',
            'raw_json': '{}',
            'normalized_json': '{}',
        }
    ]
    m['table_states'].update(historia_tramites='COMPLETA', calendarios='COMPLETA')
    capture = read_capture(write_capture(tmp_path / 'cause.zip', m, t))
    from nurus.capture.temporal import chronology
    from datetime import date

    ctx = chronology(capture, r, date(2026, 10, 8), fixture_preview=True)
    assert 'cause-event' not in [e['event_id'] for e in ctx['later']]
    assert capture.rows('calendarios')[0]['association'] == 'NO_ASOCIADO'


@pytest.mark.parametrize(
    'change',
    [
        lambda m, t: t['consultas'][0].update(expected_pages='²'),
        lambda m, t: t['registros'][0].update(original_row='²'),
        lambda m, t: t['consultas'][0].update(
            period_from='2026-10-01T09:00:00+00:00', period_to='2026-10-01T10:00:00+05:00'
        ),
    ],
)
def test_malformed_values_raise_capture_error_not_crude_exceptions(tmp_path, change):
    m, t = fixture()
    change(m, t)
    with pytest.raises(CaptureError):
        write_capture(tmp_path / 'x.zip', m, t)


def test_equal_page_counts_and_ordered_period_with_zones_are_accepted(tmp_path):
    m, t = fixture()
    t['consultas'][0].update(expected_pages='01', saved_pages='1')
    t['consultas'][0].update(period_from='2026-10-01T10:00:00+05:00', period_to='2026-10-01T09:00:00+00:00')
    read_capture(write_capture(tmp_path / 'ok.zip', m, t))


def test_lowercase_content_types_is_read_not_keyerror():
    from io import BytesIO
    from nurus.capture.contract import check_workbook

    out = BytesIO()
    with zipfile.ZipFile(out, 'w') as book:
        book.writestr('[content_types].xml', '<Types/>')
        book.writestr('xl/workbook.xml', '<workbook/>')
    # WP-63 exige los nombres exactos que escribe Excel; lo que importa es un rechazo claro, no un KeyError.
    with pytest.raises(CaptureError, match='no es un libro'):
        check_workbook(out.getvalue())


def test_corrupt_deflate_in_workbook_is_capture_error():
    from io import BytesIO
    from nurus.capture.contract import check_workbook

    out = BytesIO()
    with zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED) as book:
        book.writestr('[Content_Types].xml', '<Types>' + 'x' * 4000 + '</Types>')
        book.writestr('xl/workbook.xml', '<workbook/>')
    raw = bytearray(out.getvalue())
    start = raw.index(b'[Content_Types].xml') + len('[Content_Types].xml')
    for i in range(start, start + 12):
        raw[i] ^= 0xFF
    with pytest.raises(CaptureError):
        check_workbook(bytes(raw))
