"""Medición sintética de importación; no consulta ni produce solicitudes masivas."""
import argparse
from datetime import date
import json
from pathlib import Path
import sys
from tempfile import TemporaryDirectory
from time import perf_counter
import tracemalloc

sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'src'))
from nurus.capture.fixtures import fixture
from nurus.capture.contract import write_capture
from nurus.personal.capture_input import import_capture
from nurus.personal.config import defaults


def measure(count):
    with TemporaryDirectory(prefix='flow-import-benchmark-') as temporary:
        metadata,tables=fixture(count)
        package=write_capture(Path(temporary)/'capture.zip',metadata,tables)
        del metadata,tables
        tracemalloc.start();start=perf_counter()
        work=import_capture(package,defaults(),as_of=date(2026,10,8))
        elapsed=perf_counter()-start;_,peak=tracemalloc.get_traced_memory();tracemalloc.stop()
        assert len(work.rows)==count
        return {'records':count,'seconds':round(elapsed,4),'peak_traced_bytes':peak,
                'data_kind':'SINTETICA','institutional_access':False}


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('--rows',type=int,nargs='+',default=[2000,10000,20000])
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    if any(n<1 or n>20000 for n in args.rows):parser.error('Filas entre 1 y 20.000.')
    results=[]
    for count in args.rows:
        results.append(measure(count)); print(f'{count} registros: {results[-1]["seconds"]} s',flush=True)
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(json.dumps(results,indent=2)+'\n',encoding='utf-8')
