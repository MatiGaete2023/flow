from pathlib import Path
import subprocess
import tomllib
import nurus

ROOT=Path(__file__).parents[1]


def test_release_version_and_current_docs_are_aligned():
    project=tomllib.loads((ROOT/'pyproject.toml').read_text(encoding='utf-8'))
    assert project['project']['version']==nurus.__version__
    current_docs=[
        ROOT/'README.md', ROOT/'docs/IMPLEMENTACION.md',
        ROOT/'docs/VERIFICACION_PERSONAL.md', ROOT/'docs/ACEPTACION_CSMP_PERSONAL.md',
        ROOT/'docs/INDICE_DOCUMENTACION.md', ROOT/'docs/ESTADO_CONSOLIDADO_20260920.md',
    ]
    for path in current_docs:
        text=path.read_text(encoding='utf-8')
        assert nurus.__version__ in text[:1000], path
    readme=current_docs[0].read_text(encoding='utf-8')
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
    assert f'CSMP_Assistant_personal_{nurus.__version__.replace(".dev","-dev")}.zip' in workflow
    assert f'CSMP-Windows-{nurus.__version__.split(".")[-1]}' in workflow
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
