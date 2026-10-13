"""Synthetic regressions for the audit of verification and identity gates."""

import importlib.util
import lzma
import subprocess
import zipfile
import zlib
from pathlib import Path

import pytest


def tool(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).resolve().parents[1] / 'tools' / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


scanner = tool('check_real_ids')
verifier = tool('verificar_wp')


def test_indice_lee_blob_por_oid_y_rechaza_gitlink(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(scanner, 'ROOT', tmp_path)
    subprocess.run(['git', 'init', '-q', str(tmp_path)], check=True)
    image = tmp_path / 'imagen.png'
    image.write_bytes(b'contenido binario ficticio')
    subprocess.run(['git', 'add', 'imagen.png'], cwd=tmp_path, check=True)
    original = scanner.subprocess.run
    commands = []

    def tracked(command, **kwargs):
        commands.append(command)
        return original(command, **kwargs)

    monkeypatch.setattr(scanner.subprocess, 'run', tracked)
    assert scanner.scan([image], set(), stage=True) == []
    assert any('cat-file' in command for command in commands)
    assert not any('show' in command for command in commands)
    original(
        ['git', 'update-index', '--add', '--cacheinfo', '160000,' + 'a' * 40 + ',submodulo'],
        cwd=tmp_path, check=True,
    )
    assert scanner.main(['--stage']) == 2
    assert 'Traceback' not in capsys.readouterr().err


def test_indice_lee_blob_indexado_con_sufijo_desconocido(tmp_path, monkeypatch, identity):
    monkeypatch.setattr(scanner, 'ROOT', tmp_path)
    subprocess.run(['git', 'init', '-q', str(tmp_path)], check=True)
    sample = tmp_path / 'sesion.bin'
    sample.write_text(identity, encoding='utf-8')
    subprocess.run(['git', 'add', 'sesion.bin'], cwd=tmp_path, check=True)
    sample.write_text('árbol sin el identificador', encoding='utf-8')
    assert len(scanner.scan([sample], set(), stage=True)) == 1
    image = tmp_path / (identity + '.png')
    image.write_bytes(b'imagen ficticia')
    subprocess.run(['git', 'add', image.name], cwd=tmp_path, check=True)
    assert len(scanner.scan([image], set(), stage=True)) == 1
    renamed = tmp_path / 'renombrado.png'
    renamed.write_text(identity, encoding='utf-8')
    subprocess.run(['git', 'add', renamed.name], cwd=tmp_path, check=True)
    assert len(scanner.scan([renamed], set(), stage=True)) == 1


@pytest.fixture
def identity():
    # Generated only in the temporary fixture; never stored as a real identifier.
    body = str(12_345_678)
    return body + '-' + scanner.check_digit(body)


@pytest.mark.parametrize('separator', ['-', '\u2013', '\u2212'])
def test_identity_spaces_and_unicode_hyphens(tmp_path, identity, separator):
    sample = tmp_path / 'synthetic.txt'
    sample.write_text(identity[:-2] + ' ' + separator + ' ' + identity[-1], encoding="utf-8")
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


@pytest.mark.parametrize('error', [zlib.error, lzma.LZMAError])
def test_corrupt_compressed_member_fails_without_traceback(monkeypatch, tmp_path, capsys, error):
    sample = tmp_path / 'fixture.zip'
    with zipfile.ZipFile(sample, 'w') as archive:
        archive.writestr('fixture.txt', 'contenido ficticio')

    def corrupt(*args, **kwargs):
        raise error('private-fictitious-marker')

    monkeypatch.setattr(zipfile.ZipFile, 'read', corrupt)
    assert scanner.main([str(sample)]) == 2
    output = capsys.readouterr()
    assert 'no se certifica' in output.err
    assert 'private-fictitious-marker' not in output.out + output.err


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
    manifest.write_text('tests/test_fake.py::test_deleted\n', encoding="utf-8")
    monkeypatch.setattr(verifier, 'run', lambda *_: (2, 'collection failed'))
    counts, problems, nodes = verifier.removed_from_lists([manifest])
    assert counts['asistente'] == 1
    assert any('No se pudo recolectar' in issue for issue in problems)


def test_send_with_space_is_restricted(monkeypatch, tmp_path):
    source = tmp_path / 'src'
    source.mkdir()
    (source / 'fake.py').write_text('mail.Send ()\n', encoding="utf-8")
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
    (sources / 'global.json').write_text('{}', encoding="utf-8")
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
    (sources / 'global.json').write_text('{ private synthetic invalid', encoding="utf-8")
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
    }
    found = set()
    for workflow in ['tests.yml', 'descargador-integral.yml']:
        source = (root / '.github' / 'workflows' / workflow).read_text(encoding="utf-8")
        for block in source.split('- name: ')[1:]:
            name = block.splitlines()[0].strip()
            if name not in names:
                continue
            found.add(name)
            condition = next(
                line.strip().removeprefix('if: ') for line in block.splitlines() if line.strip().startswith('if: ')
            )
            for event, expected in [('pull_request', False), ('push', False), ('workflow_dispatch', True)]:
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
    manifest.write_text(node, encoding="utf-8")
    monkeypatch.setattr(verifier, 'collected_nodeids', lambda *_: set())
    inventory = {'asistente': ['tests/test_x.py::test_exists[synthetic]'], 'descargador': []}
    assert verifier.removed_from_lists([manifest], 'commit', inventory)[1]


def test_exact_baseline_collection_supports_parameterized_removals(monkeypatch, tmp_path):
    node = 'tests/test_x.py::test_exists[synthetic]'
    manifest = tmp_path / 'removed.txt'
    manifest.write_text(node, encoding="utf-8")
    monkeypatch.setattr(verifier, 'collected_nodeids', lambda *_: set())
    inventory = {'asistente': [node], 'descargador': []}
    assert verifier.removed_from_lists([manifest], 'commit', inventory) == (
        {'asistente': 1, 'descargador': 0},
        [],
        {'asistente': {node}, 'descargador': set()},
    )
    assert verifier.removed_from_lists([manifest], 'commit', {})[1]


def test_absence_of_exact_baseline_collection_cannot_certify_removal(monkeypatch, tmp_path):
    manifest = tmp_path / 'removed.txt'
    manifest.write_text('tests/test_x.py::test_exists', encoding="utf-8")
    monkeypatch.setattr(verifier, 'collected_nodeids', lambda *_: set())
    assert verifier.removed_from_lists([manifest], 'commit')[1]


def zip_bytes(name, data):
    import io

    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr(name, data)
    return buffer.getvalue()


def test_zip_capture_checks_consolidated_xlsx(tmp_path, identity, capsys):
    sample = tmp_path / 'synthetic_capture.zip'
    sample.write_bytes(
        zip_bytes('consolidated.xlsx', zip_bytes('xl/sharedStrings.xml', f'<sst><si><t>{identity}</t></si></sst>'))
    )
    assert scanner.main([str(sample)]) == 1
    output = capsys.readouterr().out
    assert identity not in output
    assert 'consolidated.xlsx' in output


def test_corrupt_nested_ooxml_cannot_certify_clean(tmp_path):
    sample = tmp_path / 'synthetic_capture.zip'
    sample.write_bytes(zip_bytes('consolidated.xlsx', b'not zip'))
    assert scanner.main([str(sample)]) == 2


@pytest.mark.parametrize('limit', ['depth', 'expansion', 'members'])
def test_archive_resource_limits_fail_closed(tmp_path, monkeypatch, limit):
    sample = tmp_path / 'synthetic_capture.zip'
    if limit == 'depth':
        data = b'fictitious'
        for _ in range(scanner.MAX_ARCHIVE_DEPTH + 1):
            data = zip_bytes('nested.zip', data)
    elif limit == 'expansion':
        monkeypatch.setattr(scanner, 'MAX_ARCHIVE_BYTES', 5)
        data = zip_bytes('synthetic.txt', 'fictitious')
    else:
        monkeypatch.setattr(scanner, 'MAX_ARCHIVE_MEMBERS', 0)
        data = zip_bytes('synthetic.txt', 'fictitious')
    sample.write_bytes(data)
    assert scanner.main([str(sample)]) == 2


def test_identifier_in_filename_is_redacted_in_diagnostics(tmp_path, identity, capsys):
    sample = tmp_path / (identity + '.txt')
    sample.write_text(identity, encoding="utf-8")
    assert scanner.main([str(sample)]) == 1
    assert identity not in capsys.readouterr().out


@pytest.mark.parametrize('suffix', ['.txt', '.png'])
def test_identity_in_filename_without_sensitive_contents_is_detected(tmp_path, identity, suffix, capsys):
    sample = tmp_path / (identity + suffix)
    sample.write_bytes(b'innocuous fixture')
    assert scanner.main([str(sample)]) == 1
    assert identity not in capsys.readouterr().out


def test_identity_in_tracked_directory_name_is_detected(monkeypatch, tmp_path, identity, capsys):
    folder = tmp_path / identity
    folder.mkdir()
    sample = folder / 'fixture.png'
    sample.write_bytes(b'innocuous fixture')
    monkeypatch.setattr(scanner, 'ROOT', tmp_path)
    monkeypatch.setattr(scanner, 'ALLOWLIST', tmp_path / 'allowlist.txt')
    monkeypatch.setattr(scanner, 'tracked_files', lambda: [sample])
    assert scanner.main([]) == 1
    assert identity not in capsys.readouterr().out


@pytest.mark.parametrize('member', ['{identity}.txt', '{identity}.png', '{identity}/innocuous.txt'])
def test_identity_in_zip_member_name_is_detected(tmp_path, identity, member, capsys):
    sample = tmp_path / 'fixture.zip'
    with zipfile.ZipFile(sample, 'w') as archive:
        archive.writestr(member.format(identity=identity), b'innocuous fixture')
    assert scanner.main([str(sample)]) == 1
    assert identity not in capsys.readouterr().out


@pytest.mark.parametrize('encoding', ['utf-8', 'utf-8-sig', 'cp1252', 'utf-16', 'utf-16-be'])
def test_identity_detection_preserves_explicit_text_encodings(tmp_path, identity, encoding):
    text = identity.replace('-', '\u2013')
    data = text.encode(encoding)
    if encoding == 'utf-16-be':
        import codecs

        data = codecs.BOM_UTF16_BE + data
    sample = tmp_path / 'synthetic.txt'
    sample.write_bytes(data)
    assert scanner.scan([sample], set())
    nested = tmp_path / 'synthetic.zip'
    nested.write_bytes(zip_bytes('synthetic.txt', data))
    assert scanner.scan([nested], set())


@pytest.mark.parametrize('data', [b'\x81', b'\xef\xbb\xbf\x81', b'\xff\xfe\x41', b'a\x00b'])
def test_invalid_or_ambiguous_text_encoding_fails_closed(tmp_path, data, capsys):
    sample = tmp_path / 'synthetic.txt'
    sample.write_bytes(data)
    assert scanner.main([str(sample)]) == 2
    nested = tmp_path / 'synthetic.zip'
    nested.write_bytes(zip_bytes('synthetic.txt', data))
    assert scanner.main([str(nested)]) == 2
    assert 'no se certifica' in capsys.readouterr().err


@pytest.mark.parametrize('encoding', ['utf-8', 'utf-16', 'windows-1252'])
def test_ooxml_uses_declared_encoding_without_losing_identity(tmp_path, identity, encoding):
    text = identity.replace('-', '\u2013')
    xml = f'<?xml version="1.0" encoding="{encoding}"?><sst><si><r><t>{text[:4]}</t></r><r><t>{text[4:]}</t></r></si></sst>'
    data = xml.encode(encoding)
    sample = tmp_path / 'synthetic.xlsx'
    sample.write_bytes(zip_bytes('xl/sharedStrings.xml', data))
    assert scanner.scan([sample], set())


def test_unknown_xml_encoding_fails_with_sanitized_diagnostic(tmp_path, identity, capsys):
    marker = 'private-fixture-unknown'
    xml = f'<?xml version="1.0" encoding="{marker}"?><sst><si><t>{identity}</t></si></sst>'
    sample = tmp_path / 'synthetic.xlsx'
    sample.write_bytes(zip_bytes('xl/sharedStrings.xml', xml.encode('utf-8')))
    assert scanner.main([str(sample)]) == 2
    output = capsys.readouterr()
    assert marker not in output.out + output.err
    assert identity not in output.out + output.err
    assert 'Traceback' not in output.out + output.err


def test_ci_budget_has_no_automatic_windows_build_or_duplicate_exe():
    root = Path(__file__).resolve().parents[1]
    for name in ['tests.yml', 'descargador-integral.yml']:
        source = (root / '.github/workflows' / name).read_text()
        events = source.split('permissions:')[0].split('concurrency:')[0]
        assert '  push:' not in events
        assert "branches: ['feat/**']" in events and '    paths:' in events
        assert 'timeout-minutes:' in source
        assert 'python -m PyInstaller' not in source
    assistant = (root / '.github/workflows/tests.yml').read_text()
    assert assistant.count('./tools/build_integral.ps1') == 1
    assert 'path: dist/integral/CSMP_Integral/' in assistant
    assert 'path: dist/integral/SITFA_Descargador/' in assistant
    assert 'retention-days: 90' not in assistant
    linux = (root / '.github/workflows/ci-rapida.yml').read_text()
    assert 'requirements/linux-py312.lock' in linux
    assert linux.index('python tools/check_real_ids.py') < linux.index('python -m pip install')
    assert 'python tools/estado.py validar' in linux
    assert 'python tools/check_links.py' in linux
    assert 'git diff --name-only "$BASE_SHA" HEAD' in linux
    assert 'xvfb-run -a python -m pytest -q tests' in linux
    assert 'xvfb-run -a python -m pytest -q descargador' in linux
