"""WP-30: el CC institucional tiene una sola fuente, nurus.services.policy.MANDATORY_CC."""

from pathlib import Path

from nurus.personal import config
from nurus.services import policy

ROOT = Path(__file__).resolve().parents[1]
DIRECCION = policy.MANDATORY_CC
CARPETAS = ('src', 'descargador', 'tools', 'installer', 'contracts')
EXTENSIONES = {'.py', '.json', '.js', '.iss', '.ps1', '.bat', '.cmd', '.html', '.txt', '.spec'}
# policy.py es la fuente; verificar_wp.py comprueba el literal en policy.py. correos_base.json conserva
# «cc_obligatorio» sin lector: quitarlo cambia la configuración serializada en los golden (control 4) y
# solo un WP autorizado a regenerar golden puede hacerlo. Ver docs/implementacion/wp/WP-30.md.
PERMITIDOS = {'src/nurus/services/policy.py', 'tools/verificar_wp.py', 'src/nurus/personal/correos_base.json'}


def _archivos():
    for carpeta in CARPETAS:
        yield from (ruta for ruta in (ROOT / carpeta).rglob('*') if ruta.is_file())
    yield from (ruta for ruta in ROOT.glob('*') if ruta.is_file())


def test_direccion_literal_solo_en_la_fuente():
    hallazgos = []
    for ruta in _archivos():
        relativa = ruta.relative_to(ROOT).as_posix()
        if ruta.suffix.lower() not in EXTENSIONES or '__pycache__' in ruta.parts or relativa in PERMITIDOS:
            continue
        if DIRECCION.casefold() in ruta.read_text(encoding='utf-8', errors='ignore').casefold():
            hallazgos.append(relativa)
    assert hallazgos == []


def test_config_reexporta_la_constante_y_nadie_lee_cc_obligatorio():
    assert config.CC is policy.MANDATORY_CC
    lectores = [
        ruta.relative_to(ROOT).as_posix()
        for ruta in (ROOT / 'src').rglob('*.py')
        if 'cc_obligatorio' in ruta.read_text(encoding='utf-8')
    ]
    assert lectores == []
