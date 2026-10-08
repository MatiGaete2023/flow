"""Reproduce la misma verificación de productos incluida en el EXE de CSMP."""
import argparse
import json
from pathlib import Path
import sys

sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'src'))
from nurus.personal.capture_selfcheck import check_pipeline


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output',type=Path,help='Directorio nuevo para productos exclusivamente ficticios.')
    args=parser.parse_args()
    result=check_pipeline(args.output)
    (args.output/'VERIFICACION_PRODUCTOS.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(result,ensure_ascii=False,indent=2))
