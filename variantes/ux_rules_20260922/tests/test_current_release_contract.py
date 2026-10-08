from pathlib import Path
import subprocess
import tomllib
import nurus

ROOT=Path(__file__).parents[1]


def test_release_version_and_current_docs_are_aligned():
    project=tomllib.loads((ROOT/'pyproject.toml').read_text(encoding='utf-8'))
    assert project['project']['version']==nurus.__version__=='0.4.0.dev12'
    current_docs=[ROOT/'README.md',ROOT/'docs/EVOLUCION_UX_REGLAS_20260922.md']
    for path in current_docs:
        text=path.read_text(encoding='utf-8')
        assert '0.4.0.dev12' in text, path
    frozen=(ROOT/'docs/CONGELAMIENTO_CANDIDATA_20260922.md').read_text(encoding='utf-8')
    assert '0.4.0.dev11' in frozen
    historical=(ROOT/'docs/CAMBIOS_USO_20260914.md').read_text(encoding='utf-8')
    assert '0.4.0.dev5' in historical
    assert '0.4.0.dev11' not in historical
    index=(ROOT/'docs/INDICE_DOCUMENTACION.md').read_text(encoding='utf-8')
    assert 'CAMBIOS_USO_20260916.md' in index
    assert 'ESTADO_CONSOLIDADO_20260920.md' in index
    assert 'CAMBIOS_USO_20260914.md' in index and 'histórico' in index.lower()
    readme=current_docs[0].read_text(encoding='utf-8')
    assert 'Windows/Linux' not in readme
    assert '22 de septiembre de 2026' in readme
    personal=(ROOT/'docs/IMPLEMENTACION_PERSONAL.md').read_text(encoding='utf-8')
    assert 'excepción documentada' not in personal.lower()
    assert 'Pendientes externos: PC_INFO de Laja' not in personal


def test_repository_has_no_transitional_runtime_patch_layer():
    personal=ROOT/'src/nurus/personal'
    assert not list(personal.glob('runtime_fixes_*.py'))
    init=(personal/'__init__.py').read_text(encoding='utf-8')
    assert 'runtime_fixes_' not in init


def test_workflow_and_distribution_match_current_release():
    workflow=(ROOT/'.github/workflows/tests.yml').read_text(encoding='utf-8')
    assert 'actions/checkout@v7' in workflow
    assert 'actions/setup-python@v7' in workflow
    assert 'CSMP_Assistant_personal_0.4.0-dev12.zip' in workflow
    assert 'CSMP-Windows-dev12' in workflow


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
