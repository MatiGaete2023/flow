"""Versiona el origen del ejecutable sin añadir información de la sesión."""
import argparse,json,subprocess
from pathlib import Path

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--descargador',type=Path);args=parser.parse_args()
    root=args.descargador.resolve() if args.descargador else Path(__file__).resolve().parents[1]
    commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
    if args.descargador:
        version=json.loads((root/'extension'/'manifest.json').read_text(encoding='utf-8'))['version']
        target=root/'_build_meta.py'
    else:
        import sys
        sys.path.insert(0,str(root/'src'))
        from nurus import __version__
        version=__version__;target=root/'src'/'nurus'/'personal'/'_build_meta.py'
    target.write_text(f'BUILD_COMMIT = {commit!r}\nBUILD_VERSION = {version!r}\n',encoding='utf-8')
    print(version,commit)

if __name__=='__main__':main()
