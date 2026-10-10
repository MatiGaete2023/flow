"""Cada versión se escribe en un solo lugar; las demás ocurrencias derivan de él."""
import importlib.util
import json
import re
from pathlib import Path

import nurus

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('write_build_meta', ROOT / 'tools' / 'write_build_meta.py')
meta = importlib.util.module_from_spec(spec)
spec.loader.exec_module(meta)


def _texto(relativa):
    return (ROOT / relativa).read_text(encoding='utf-8')


def test_fuentes_unicas():
    assert meta.assistant_version() == nurus.__version__
    assert meta.downloader_version() == re.search(r"^VERSION='([^']+)'", _texto('descargador/contrato_integral.py'), re.M).group(1)
    pyproject = _texto('pyproject.toml')
    assert 'dynamic = ["version"]' in pyproject and 'version = {attr = "nurus.__version__"}' in pyproject
    assert not re.search(r'^version\s*=\s*"', pyproject, re.M)


def test_extension_coincide_con_el_descargador():
    version = meta.downloader_version()
    manifest = json.loads(_texto('descargador/extension/manifest.json'))
    assert manifest['version'] == version
    assert manifest['version_name'] == meta.VERSION_NAME.format(version=version)
    assert meta.PAGE_VERSION.search(_texto('descargador/extension/pagina.js')).group(0).endswith(f"version:'{version}'")


def test_sin_versiones_escritas_a_mano():
    asistente, descargador = meta.assistant_version(), meta.downloader_version()
    archivos = ['installer/CSMP.iss', 'descargador/descargador.py', 'tools/build_integral.ps1',
                '.github/workflows/tests.yml', '.github/workflows/descargador-integral.yml',
                '.github/workflows/verify-delivered-installer.yml']
    for relativa in archivos:
        texto = _texto(relativa)
        for literal in (asistente, asistente.replace('.dev', '-dev'), descargador, 'dev4', '2.7.4', '2.7.5', 'SITFA-27'):
            assert literal not in texto, (relativa, literal)


def test_sincronizar_no_cambia_archivos_ya_coherentes(tmp_path):
    destino = tmp_path / 'descargador'
    (destino / 'extension').mkdir(parents=True)
    for nombre in ('contrato_integral.py', 'extension/manifest.json', 'extension/pagina.js'):
        (destino / nombre).write_bytes((ROOT / 'descargador' / nombre).read_bytes())
    meta.sync_extension(destino)
    for nombre in ('extension/manifest.json', 'extension/pagina.js'):
        assert (destino / nombre).read_bytes() == (ROOT / 'descargador' / nombre).read_bytes()
