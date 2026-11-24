"""Genera extensión de validación local en carpeta NUEVA; no instala ni accede a RUS."""
from pathlib import Path
import argparse
import shutil

ROOT=Path(__file__).resolve().parents[1]
def build(destination,*,pilot=False):
    if pilot:
        from .contracts import CAPABILITIES
        if CAPABILITIES['write_one'] is not True:
            raise RuntimeError('PILOTO_REAL_NO_HABILITADO')
    target=Path(destination)
    target.mkdir(parents=True,exist_ok=False)
    if pilot:
        for name in ('manifest.json','background.js','pilot.html','pilot.js'):
            shutil.copyfile(ROOT/'pilot_extension'/name,target/name)
        shutil.copyfile(ROOT/'live_extension'/'browser.js',target/'browser.js')
    else:
        for name in ('manifest.json','background.js','verify.html','verify.js','browser.js'):
            shutil.copyfile(ROOT/'live_extension'/name,target/name)
    for name in ('prepare.js','observed_form.js','live_adapter.js'):
        shutil.copyfile(ROOT/'extension'/name,target/name)
    return target

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('destination')
    parser.add_argument('--pilot',action='store_true',help='solo versión aceptada del piloto real')
    args=parser.parse_args()
    build(args.destination,pilot=args.pilot)
