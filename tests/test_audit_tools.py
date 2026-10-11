"""Synthetic regressions for the audit of verification and identity gates."""

import importlib.util
import subprocess
import zipfile
from pathlib import Path

import pytest


def tool(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).resolve().parents[1] / 'tools' / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


scanner = tool('check_real_ids')
verifier = tool('verificar_wp')


@pytest.fixture
def identity():
    # Generated only in the temporary fixture; never stored as a real identifier.
    body = str(12_345_678)
    return body + '-' + scanner.check_digit(body)


@pytest.mark.parametrize('separator', ['-', '\u2013', '\u2212'])
def test_identity_spaces_and_unicode_hyphens(tmp_path, identity, separator):
    sample = tmp_path / 'synthetic.txt'
    sample.write_text(identity[:-2] + ' ' + separator + ' ' + identity[-1])
    assert scanner.scan([sample], set())


@pytest.mark.parametrize(
    'container, xml',
    [
        (
            'fixture.docx',
            '<w:document xmlns:w="word"><w:p><w:r><w:t>{first}</w:t></w:r><w:r><w:t>{last}</w:t></w:r></w:p></w:document>',
        ),
        ('fixture.xlsx', '<sst xmlns="excel"><si><r><t>{first}</t></r><r><t>{last}</t></r></si></sst>'),
    ],
)
def test_ooxml_split_identity_is_detected(tmp_path, identity, container, xml):
    sample = tmp_path / container
    with zipfile.ZipFile(sample, 'w') as archive:
        archive.writestr('content.xml', xml.format(first=identity[:4], last=identity[4:]))
    found = scanner.scan([sample], set())
    assert len(found) == 1
    assert identity not in repr(found)


@pytest.mark.parametrize('contents', [b'not zip', None])
def test_unreadable_container_cannot_certify_clean(tmp_path, contents, capsys):
    sample = tmp_path / 'synthetic.xlsx'
    if contents is None:
        with zipfile.ZipFile(sample, 'w') as archive:
            archive.writestr('content.xml', '<invalid')
    else:
        sample.write_bytes(contents)
    assert scanner.main([str(sample)]) == 2
    assert 'no se certifica' in capsys.readouterr().err


def test_absent_explicit_path_cannot_certify_clean(tmp_path):
    with pytest.raises(SystemExit) as result:
        scanner.main([str(tmp_path / 'absent.xlsx')])
    assert result.value.code == 2


def test_history_rejects_silent_exclusions(monkeypatch):
    monkeypatch.setattr(scanner, 'scan_history', lambda *_: pytest.fail('must validate before scanning'))
    with pytest.raises(SystemExit):
        scanner.main(['--historia', '--excluir', 'src'])


def test_collection_failure_does_not_prove_removed_node(monkeypatch, tmp_path):
    manifest = tmp_path / 'removed.txt'
    manifest.write_text('tests/test_fake.py::test_deleted\n')
    monkeypatch.setattr(verifier, 'run', lambda *_: (2, 'collection failed'))
    counts, problems = verifier.removed_from_lists([manifest])
    assert counts['asistente'] == 1
    assert any('No se pudo recolectar' in issue for issue in problems)


def test_send_with_space_is_restricted(monkeypatch, tmp_path):
    source = tmp_path / 'src'
    source.mkdir()
    (source / 'fake.py').write_text('mail.Send ()\n')
    monkeypatch.setattr(verifier, 'ROOT', tmp_path)
    assert any('Send()' in issue for issue in verifier.restrictions())


def test_cli_help_does_not_rewrite_documents(tmp_path):
    root = Path(__file__).resolve().parents[1]
    # CLI --help must be read only, including for the former implicit writers.
    paths = [root / 'tools' / f'{name}.py' for name in ['flow_plan_progress', 'flow_baseline_matrix']]
    before = subprocess.run(['git', 'diff', '--', 'docs'], cwd=root, check=True, capture_output=True).stdout
    for path in paths:
        result = subprocess.run([verifier.PY, str(path), '--help'], cwd=tmp_path, capture_output=True)
        assert result.returncode == 0
    after = subprocess.run(['git', 'diff', '--', 'docs'], cwd=root, check=True, capture_output=True).stdout
    assert before == after


state_tool = tool('estado')


def test_functional_cycle_is_invalid():
    state = {
        'paquetes': {
            'WP-01': {'estado': 'PENDIENTE', 'depende_de': ['WP-02']},
            'WP-02': {'estado': 'PENDIENTE', 'depende_de': ['WP-01']},
        }
    }
    assert any('ciclo' in issue for issue in state_tool.problems(state))
    state['paquetes']['WP-01']['depende_de'] = [{'wp': 'WP-02', 'tipo': 'archivo'}]
    assert not state_tool.problems(state)


def test_invalid_sources_do_not_overwrite_view(monkeypatch, tmp_path):
    sources = tmp_path / 'sources'
    sources.mkdir()
    (sources / 'global.json').write_text('{}')
    view = tmp_path / 'ESTADO.json'
    view.write_bytes(b'previous view')
    monkeypatch.setattr(state_tool, 'SOURCES', sources)
    monkeypatch.setattr(state_tool, 'VIEW', view)
    monkeypatch.setattr(state_tool, 'aggregate', lambda: {'paquetes': {'WP-00': {'estado': 'INVALID'}}})
    assert state_tool.main(['agregar']) == 1
    assert view.read_bytes() == b'previous view'


def test_invalid_json_reports_failure_without_traceback(monkeypatch, tmp_path, capsys):
    sources = tmp_path / 'sources'
    sources.mkdir()
    (sources / 'global.json').write_text('{ private synthetic invalid')
    monkeypatch.setattr(state_tool, 'SOURCES', sources)

    # A failed source read must not publish its content through a traceback.
    def broken_source():
        raise ValueError('private synthetic invalid')

    monkeypatch.setattr(state_tool, 'aggregate', broken_source)
    assert state_tool.main(['validar']) == 1
    assert 'private' not in capsys.readouterr().out


def test_ooxml_identity_is_not_double_counted(tmp_path, identity):
    sample = tmp_path / 'synthetic.docx'
    with zipfile.ZipFile(sample, 'w') as archive:
        archive.writestr('content.xml', f'<document><p><t>{identity}</t></p></document>')
    assert len(scanner.scan([sample], set())) == 1


def test_heavy_ci_steps_run_after_integration_and_skip_pr():
    root = Path(__file__).resolve().parents[1]
    names = {
        'Windows executable',
        'Build Windows distribution',
        'Build and verify Windows installer',
        'Verificar lector y empaquetado integral SITFA',
        'Extensión y servidor ficticios',
        'Ejecutable y contrato compartido',
    }
    found = set()
    for workflow in ['tests.yml', 'descargador-integral.yml']:
        source = (root / '.github' / 'workflows' / workflow).read_text()
        for block in source.split('- name: ')[1:]:
            name = block.splitlines()[0].strip()
            if name not in names:
                continue
            found.add(name)
            condition = next(
                line.strip().removeprefix('if: ') for line in block.splitlines() if line.strip().startswith('if: ')
            )
            for event, expected in [('pull_request', False), ('push', True), ('workflow_dispatch', True)]:
                expression = (
                    condition.replace('github.event_name', repr(event))
                    .replace('matrix.python', repr('3.12'))
                    .replace('&&', 'and')
                )
                assert eval(expression, {'__builtins__': {}}, {}) == expected
    assert found == names


@pytest.mark.parametrize(
    'node',
    [
        'tests/test_x.py::InventedClass::test_exists',
        'tests/test_x.py::test_exists[invented]',
        '../tests/test_x.py::test_exists',
    ],
)
def test_invented_removed_node_cannot_borrow_existing_function(monkeypatch, tmp_path, node):
    manifest = tmp_path / 'removed.txt'
    manifest.write_text(node)
    monkeypatch.setattr(verifier, 'collected_nodeids', lambda *_: set())
    inventory = {'asistente': ['tests/test_x.py::test_exists[synthetic]'], 'descargador': []}
    assert verifier.removed_from_lists([manifest], 'commit', inventory)[1]


def test_exact_baseline_collection_supports_parameterized_removals(monkeypatch, tmp_path):
    node = 'tests/test_x.py::test_exists[synthetic]'
    manifest = tmp_path / 'removed.txt'
    manifest.write_text(node)
    monkeypatch.setattr(verifier, 'collected_nodeids', lambda *_: set())
    inventory = {'asistente': [node], 'descargador': []}
    assert verifier.removed_from_lists([manifest], 'commit', inventory) == ({'asistente': 1, 'descargador': 0}, [])
    assert verifier.removed_from_lists([manifest], 'commit', {})[1]


def test_absence_of_exact_baseline_collection_cannot_certify_removal(monkeypatch, tmp_path):
    manifest = tmp_path / 'removed.txt'
    manifest.write_text('tests/test_x.py::test_exists')
    monkeypatch.setattr(verifier, 'collected_nodeids', lambda *_: set())
    assert verifier.removed_from_lists([manifest], 'commit')[1]
