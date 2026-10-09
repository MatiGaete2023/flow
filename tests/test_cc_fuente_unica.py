"""El CC institucional tiene una sola fuente: nurus.services.policy.MANDATORY_CC."""
from pathlib import Path

from nurus.personal import config
from nurus.services import policy

ROOT = Path(__file__).resolve().parents[1]
DIRECCION = policy.MANDATORY_CC
PERMITIDOS = {ROOT / 'src' / 'nurus' / 'services' / 'policy.py'}
CARPETAS = ('src', 'descargador', 'tools', 'installer', 'contracts')
EXTENSIONES = {'.py', '.json', '.js', '.iss', '.ps1', '.bat', '.cmd', '.html', '.txt'}


def test_direccion_literal_solo_en_policy():
    hallazgos = []
    for carpeta in CARPETAS:
        for ruta in (ROOT / carpeta).rglob('*'):
            if ruta.is_file() and ruta.suffix.lower() in EXTENSIONES and ruta not in PERMITIDOS and '__pycache__' not in ruta.parts:
                if DIRECCION.casefold() in ruta.read_text(encoding='utf-8', errors='ignore').casefold():
                    hallazgos.append(str(ruta.relative_to(ROOT)))
    for ruta in ROOT.glob('*'):
        if ruta.is_file() and ruta.suffix.lower() in EXTENSIONES:
            if DIRECCION.casefold() in ruta.read_text(encoding='utf-8', errors='ignore').casefold():
                hallazgos.append(ruta.name)
    # verificar_wp.py comprueba el literal en policy.py. correos_base.json conserva cc_obligatorio (sin lector)
    # hasta que WP-12 regenere los golden: quitarlo antes rompe la paridad (control 4). Ver docs/implementacion/wp/WP-30.md.
    allowed_tools = {'tools/verificar_wp.py', 'src/nurus/personal/correos_base.json'}
    assert [h for h in hallazgos if h.replace('\\', '/') not in allowed_tools] == []


def test_config_reexporta_la_constante():
    assert config.CC is policy.MANDATORY_CC
