"""Empaqueta una compilación verificada y sus fuentes, sin datos de usuario."""
import hashlib
import importlib.metadata
import json
from pathlib import Path
import platform
import shutil
import sys
import zipfile

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = '9726448af37b86cf73a87e25d087cea839fb641b'
SOURCE_FILES = [
    'tools/anonimizar_local.py',
    'src/nurus/__init__.py', 'src/nurus/personal/__init__.py',
    'src/nurus/personal/anonymize.py', 'src/nurus/personal/anonymize_package.py',
    'src/nurus/capture/__init__.py', 'src/nurus/capture/contract.py',
    'src/nurus/capture/coverage.py', 'src/nurus/capture/fixtures.py',
    'src/nurus/rus/__init__.py', 'src/nurus/rus/columns.py',
    'src/nurus/rus/models.py', 'src/nurus/rus/reader.py',
]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    out = Path(sys.argv[1]).resolve()
    out.mkdir(parents=True, exist_ok=True)
    bundle = out / 'AnonimizadorCSMP-portable'
    bundle.mkdir(exist_ok=False)
    binary = ROOT / 'dist/anonimizador/AnonimizadorCSMP.exe'
    report = ROOT / 'dist/anonimizador/AUTOPRUEBA.json'
    if json.loads(report.read_text())['resultado'] != 'PASA':
        raise RuntimeError('Autoprueba no aprobada')
    shutil.copy2(binary, bundle / binary.name)
    shutil.copy2(report, bundle / report.name)
    shutil.copy2(ROOT / 'portable/LEEME.md', bundle / 'LEEME.md')
    versions = {name: importlib.metadata.version(name) for name in ('openpyxl', 'xlrd', 'et-xmlfile', 'pyinstaller', 'pyinstaller-hooks-contrib')}
    verification = {'base_flow': UPSTREAM, 'python': platform.python_version(),
                    'sistema_compilacion': platform.platform(), 'arquitectura': platform.machine(),
                    'sha256_exe': digest(binary), 'dependencias': versions,
                    'autoprueba_exe': 'PASA', 'firma_authenticode': False,
                    'windows_10_validacion_manual': False}
    (bundle / 'VERIFICACION.json').write_text(json.dumps(verification, indent=2), encoding='utf-8')
    licenses = bundle / 'LICENCIAS'
    licenses.mkdir()
    shutil.copy2(Path(sys.base_prefix) / 'LICENSE.txt', licenses / 'Python.txt')
    for name in ('openpyxl', 'xlrd', 'et-xmlfile', 'pyinstaller'):
        distribution = importlib.metadata.distribution(name)
        for file in distribution.files or []:
            if 'licen' in file.name.lower() or file.name.upper().startswith(('COPYING', 'NOTICE')):
                source = Path(distribution.locate_file(file))
                if source.is_file():
                    shutil.copy2(source, licenses / (name + '-' + file.name))
    provenance = {'repo': 'https://github.com/MatiGaete2023/flow', 'commit': UPSTREAM,
                  'files': {p: digest(ROOT / p) for p in SOURCE_FILES}}
    source_zip = out / 'AnonimizadorCSMP-fuentes.zip'
    with zipfile.ZipFile(source_zip, 'x', zipfile.ZIP_DEFLATED) as archive:
        for name in SOURCE_FILES:
            archive.write(ROOT / name, name)
        for name in ('launcher.py', 'selftest.py', 'requirements-build.txt', 'build.ps1', 'package.py', 'LEEME.md'):
            archive.write(ROOT / 'portable' / name, 'portable/' + name)
        workflow = ROOT / '.github/workflows/anonimizador-portable.yml'
        if workflow.exists():
            archive.write(workflow, '.github/workflows/anonimizador-portable.yml')
        archive.writestr('PROCEDENCIA.json', json.dumps(provenance, indent=2))
    manifest = ''.join(f'{digest(p)}  {p.relative_to(bundle).as_posix()}\n' for p in sorted(bundle.rglob('*')) if p.is_file())
    (bundle / 'SHA256SUMS.txt').write_text(manifest, encoding='utf-8')
    with zipfile.ZipFile(out / 'AnonimizadorCSMP-Windows-x64.zip', 'x', zipfile.ZIP_DEFLATED) as archive:
        for file in sorted(bundle.rglob('*')):
            if file.is_file():
                archive.write(file, file.relative_to(out))
    (out / 'SHA256SUMS.txt').write_text(''.join(f'{digest(p)}  {p.name}\n' for p in sorted(out.glob('*.zip'))), encoding='utf-8')


if __name__ == '__main__':
    main()
