from pathlib import Path
import subprocess
import tomllib

ROOT=Path(__file__).parents[1]


def test_release_version_and_current_docs_are_aligned():
    # WP-31: la versión vive solo en nurus.__version__; pyproject la lee de ahí y los documentos no la repiten.
    project=tomllib.loads((ROOT/'pyproject.toml').read_text(encoding='utf-8'))
    assert 'version' not in project['project'] and 'version' in project['project']['dynamic']
    assert project['tool']['setuptools']['dynamic']['version']=={'attr':'nurus.__version__'}
    readme=(ROOT/'README.md').read_text(encoding='utf-8')
    assert 'Windows' in readme and 'Correo libre' in readme and 'Word manual' in readme
    for obsolete in ('nurus/adapters/sent_mail.py','nurus/personal/activity_view.py','nurus/personal/review_store.py'):
        assert not (ROOT/'src'/obsolete).exists()


def test_repository_has_no_transitional_runtime_patch_layer():
    personal=ROOT/'src/nurus/personal'
    assert not list(personal.glob('runtime_fixes_*.py'))
    init=(personal/'__init__.py').read_text(encoding='utf-8')
    assert 'runtime_fixes_' not in init


def test_workflow_and_distribution_match_current_release():
    workflow=(ROOT/'.github/workflows/tests.yml').read_text(encoding='utf-8')
    assert 'actions/checkout@v7' in workflow
    assert 'actions/setup-python@v7' in workflow
    assert 'CSMP_Assistant_personal_${{ env.ASSISTANT_VERSION }}.zip' in workflow
    assert 'CSMP-Windows-${{ env.ASSISTANT_VERSION }}' in workflow
    assert 'python tools/write_build_meta.py --github-env $env:GITHUB_ENV' in workflow
    assert "'codex/**'" in workflow


def test_gitignore_covers_local_csmp_environment_and_build_outputs():
    entries=set((ROOT/'.gitignore').read_text(encoding='utf-8').splitlines())
    assert {'.venv-csmp/','dist/','build/','*.egg-info/'} <= entries


def test_repository_has_no_common_generated_junk_tracked():
    """Controla el índice Git, no los cachés que pytest/compileall crean durante CI."""
    completed=subprocess.run(
        ['git','ls-files','-z'],cwd=ROOT,check=True,capture_output=True,
    )
    tracked=[Path(item.decode('utf-8')) for item in completed.stdout.split(b'\0') if item]
    forbidden_names={'.DS_Store','Thumbs.db'}
    for path in tracked:
        assert path.name not in forbidden_names
        assert '__pycache__' not in path.parts
        assert path.suffix.lower() not in {'.pyc','.pyo'}
