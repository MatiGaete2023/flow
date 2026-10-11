"""Versiona el origen del ejecutable sin añadir información de la sesión.

Fuentes únicas de versión:
  Asistente:   src/nurus/__init__.py (__version__)
  Descargador: descargador/contrato_integral.py (VERSION)

Uso:
  python tools/write_build_meta.py                    # _build_meta.py del Asistente
  python tools/write_build_meta.py --descargador DIR  # _build_meta.py del Descargador y sincroniza la extensión
  python tools/write_build_meta.py --sincronizar-extension   # solo manifest.json y pagina.js
  python tools/write_build_meta.py --versiones [--descargador DIR]  # imprime JSON con ambas versiones
  python tools/write_build_meta.py --github-env ARCHIVO      # agrega ASSISTANT_VERSION y DOWNLOADER_VERSION
"""
import argparse,json,re,subprocess
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
VERSION_NAME='{version} integral · bitácoras y evidencia'
PAGE_VERSION=re.compile(r"(return \{tipo:'CSMP_RUS_CAPACIDADES',version:')[^']*(')")

def assistant_version(root=ROOT):
    text=(root/'src'/'nurus'/'__init__.py').read_text(encoding='utf-8')
    return re.search(r'^__version__\s*=\s*"([^"]+)"',text,re.M).group(1)

def downloader_version(root=ROOT/'descargador'):
    text=(root/'contrato_integral.py').read_text(encoding='utf-8')
    return re.search(r"^VERSION\s*=\s*'([^']+)'",text,re.M).group(1)

def sync_extension(root=ROOT/'descargador'):
    version=downloader_version(root)
    manifest_path=root/'extension'/'manifest.json'
    manifest=json.loads(manifest_path.read_text(encoding='utf-8'))
    manifest['version']=version;manifest['version_name']=VERSION_NAME.format(version=version)
    manifest_path.write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    page_path=root/'extension'/'pagina.js'
    page=page_path.read_text(encoding='utf-8')
    updated,count=PAGE_VERSION.subn(lambda m:m.group(1)+version+m.group(2),page)
    if count!=1:raise SystemExit('pagina.js: no se encontró la constante de versión de capabilities')
    page_path.write_text(updated,encoding='utf-8')
    return version

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--descargador',type=Path)
    parser.add_argument('--sincronizar-extension',action='store_true');parser.add_argument('--versiones',action='store_true')
    parser.add_argument('--github-env',type=Path);args=parser.parse_args()
    downloader_root=args.descargador.resolve() if args.descargador else ROOT/'descargador'
    versions={'asistente':assistant_version(),'descargador':downloader_version(downloader_root)}
    if args.versiones:print(json.dumps(versions));return
    if args.github_env:
        with args.github_env.open('a',encoding='utf-8') as stream:
            stream.write(f"ASSISTANT_VERSION={versions['asistente']}\nDOWNLOADER_VERSION={versions['descargador']}\n")
        print(versions);return
    if args.sincronizar_extension:print(sync_extension(downloader_root));return
    root=args.descargador.resolve() if args.descargador else ROOT
    commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
    if args.descargador:
        version=sync_extension(root);target=root/'_build_meta.py'
    else:
        version=assistant_version(root);target=root/'src'/'nurus'/'personal'/'_build_meta.py'
    target.write_text(f'BUILD_COMMIT = {commit!r}\nBUILD_VERSION = {version!r}\n',encoding='utf-8')
    print(version,commit)

if __name__=='__main__':main()
