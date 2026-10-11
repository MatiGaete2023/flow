from copy import deepcopy
import json
import os
from pathlib import Path
import subprocess
import sys

import pytest
from nurus.capture import CaptureError
from nurus.capture.acceptance import CASES, template, summary, validate_acta, write_acta, read_acta

ARTIFACT = {
    'source_commit': 'a' * 40,
    'assistant_version': '0.5.0.dev4',
    'downloader_version': '2.7.2',
    'extension_version': '2.7.2',
}


def executed(kind='INSTITUCIONAL'):
    data = template(ARTIFACT, cases=['INT-24'], gate='G6', evidence_kind=kind)
    data['records'][0].update(
        result='PASÓ',
        environment='WINDOWS_10_INSTITUCIONAL' if kind == 'INSTITUCIONAL' else 'WINDOWS_CI',
        executed_at='2026-10-08T15:00:00-03:00',
        evidence_retained_locally=True,
        evidence_sha256=['b' * 64],
        operator_reference='OP-AABBCCDDEEFF',
        counts={'observed': 1, 'failed': 0},
    )
    return data


def test_all_source_requirements_pending_and_no_implicit_acceptance():
    source = json.loads((Path(__file__).parents[1] / 'docs/evidencia_p0/requisitos.json').read_text(encoding='utf-8'))
    assert set(CASES) == {r['id'] for r in source} and len(CASES) == 115
    pending = summary(template(ARTIFACT))
    assert pending['reported_results']['INSTITUCIONAL'] == {'NO_EJECUTADO': 115}
    assert pending['gate_acceptance'] == 'PENDIENTE_REVISION_HUMANA'


def test_synthetic_windows_pass_is_separate_from_institutional_acceptance():
    data = executed('SINTETICA_WINDOWS')
    data['records'].extend(template(ARTIFACT, cases=['INT-24'], gate='G6')['records'])
    report = summary(data)
    assert report['reported_results']['SINTETICA_WINDOWS'] == {'PASÓ': 1}
    assert report['reported_results']['INSTITUCIONAL'] == {'NO_EJECUTADO': 1}
    assert report['gate_acceptance'] == 'PENDIENTE_REVISION_HUMANA'


@pytest.mark.parametrize(
    'field,value',
    [
        ('environment', 'WINDOWS_CI'),
        ('operator_reference', 'Nombre de funcionario'),
        ('executed_at', '2026-10-08T15:00:00'),
        ('evidence_sha256', []),
        ('evidence_retained_locally', False),
        ('case', 'CT-99'),
        ('gate', ''),
        ('counts', {'failed': 1}),
        ('counts', {'observed': True}),
        ('counts', {'rut': 12345}),
        ('limitations', ['Texto de bitácora institucional']),
        ('result', 'ACEPTADA'),
        ('evidence_kind', None),
        ('evidence_sha256', ['b' * 64, 'b' * 64]),
    ],
)
def test_inconsistent_or_identifying_record_rejected(field, value):
    data = executed()
    data['records'][0][field] = value
    with pytest.raises(CaptureError):
        validate_acta(data)


def test_unknown_personal_fields_duplicate_cases_and_false_no_execution_rejected():
    for change in ('name', 'duplicate', 'pending', 'not_applicable'):
        data = executed()
        if change == 'name':
            data['records'][0]['person_name'] = 'Dato prohibido'
        elif change == 'duplicate':
            data['records'].append(deepcopy(data['records'][0]))
        elif change == 'pending':
            data['records'][0]['result'] = 'NO_EJECUTADO'
        else:
            data['records'][0]['result'] = 'NO_APLICA'
        with pytest.raises(CaptureError):
            validate_acta(data)


def test_acta_no_overwrite_and_invalid_input_preserves_original(tmp_path):
    path = write_acta(tmp_path / 'acta.json', template(ARTIFACT))
    before = path.read_bytes()
    with pytest.raises(ValueError):
        write_acta(path, executed())
    bad = executed()
    bad['artifact']['source_commit'] = 'placeholder'
    with pytest.raises(CaptureError):
        write_acta(tmp_path / 'incorrecta.json', bad)
    assert not (tmp_path / 'incorrecta.json').exists() and path.read_bytes() == before
    assert read_acta(path)['records'][0]['result'] == 'NO_EJECUTADO'
    assert not list(tmp_path.glob('nurus-producto-*'))


def test_cli_creates_pending_acta_and_never_echoes_invalid_sensitive_input(tmp_path):
    script = Path(__file__).parents[1] / 'tools/flow_acceptance.py'
    # Reproduce CP1252 incluso en Linux: el CLI debe publicar siempre UTF-8.
    env = {**os.environ, 'PYTHONIOENCODING': 'cp1252'}
    result = subprocess.run(
        [
            sys.executable,
            str(script),
            'init',
            str(tmp_path / 'acta.json'),
            '--commit',
            'a' * 40,
            '--assistant-version',
            '0.5.0.dev4',
            '--downloader-version',
            '2.7.2',
            '--extension-version',
            '2.7.2',
            '--case',
            'D-T10',
            '--gate',
            'G1',
        ],
        env=env,
        capture_output=True,
        text=True,
        encoding='utf-8',
        timeout=30,
    )
    assert result.returncode == 0, result.stderr
    assert json.loads(result.stdout)['reported_results']['INSTITUCIONAL'] == {'NO_EJECUTADO': 1}
    invalid = tmp_path / 'datos.json'
    invalid.write_text('{"secret":"NO_DEBE_IMPRIMIRSE"}', encoding='utf-8')
    rejected = subprocess.run(
        [sys.executable, str(script), 'validate', str(invalid)],
        env=env,
        capture_output=True,
        text=True,
        encoding='utf-8',
        timeout=30,
    )
    assert rejected.returncode == 2 and 'NO_DEBE_IMPRIMIRSE' not in rejected.stdout + rejected.stderr


def test_no_aplica_has_reason_but_is_not_execution():
    data = template(ARTIFACT, cases=['D-T13'], gate='G9')
    data['records'][0].update(result='NO_APLICA', limitations=['NO_APLICA_AL_ARTEFACTO'])
    assert summary(data)['reported_results']['INSTITUCIONAL'] == {'NO_APLICA': 1}


def test_duplicate_json_keys_and_oversize_input_rejected(tmp_path):
    p = tmp_path / 'acta.json'
    p.write_text('{"schema_id":"CSMP_ACCEPTANCE","schema_id":"otro"}')
    with pytest.raises(CaptureError):
        read_acta(p)
    p.write_bytes(b' ' * (1024 * 1024 + 1))
    with pytest.raises(CaptureError):
        read_acta(p)
