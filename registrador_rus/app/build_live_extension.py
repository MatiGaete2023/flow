"""Genera extensión de validación local en carpeta NUEVA; no instala ni accede a RUS."""
from pathlib import Path
import argparse
import shutil

ROOT=Path(__file__).resolve().parents[1]
def build(destination):
    target=Path(destination)
    target.mkdir(parents=True,exist_ok=False)
    for name in ('manifest.json','background.js','verify.html','verify.js'):
        shutil.copyfile(ROOT/'live_extension'/name,target/name)
    for name in ('prepare.js','observed_form.js','live_adapter.js'):
        shutil.copyfile(ROOT/'extension'/name,target/name)
    return target

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('destination')
    args=parser.parse_args()
    build(args.destination)
