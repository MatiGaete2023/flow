"""Recorrido offline con el exportador productivo y NuRus experimental."""
import argparse
from datetime import date
import json
from pathlib import Path
import sys
import tempfile


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('nurus_repo',type=Path);args=parser.parse_args()
    sys.path.insert(0,str(args.nurus_repo.resolve()/'src'));sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
    from test_resultados import fixture
    from resultados import load_lot,export_book
    from nurus.personal.config import defaults
    from nurus.personal.work import Work
    from nurus.personal.sitfa import verified_source
    with tempfile.TemporaryDirectory() as tmp:
        root=Path(tmp);wait=load_lot(fixture(root/'espera'))
        path=export_book([wait],csmp=True);work=Work(defaults()).analyze(path,'ESPERA',as_of=date(2026,10,1))
        assert len(work.rows)==2 and all('programa_espera' in r.actions for r in work.rows)
        assert verified_source(work,work.rows[0])==wait.folder/'pagina.xls'
        output=work.export(root/'revisable.xlsx',backend='portable',reduced_fidelity=True)
        work.save(root/'sesion');recovered=Work.load(root/'sesion')
        assert verified_source(recovered,recovered.rows[0])==wait.folder/'pagina.xls'
        cmp=load_lot(fixture(root/'cumplimiento',mode='Cumplimiento'))
        reports=load_lot(fixture(root/'informes',mode='Informes'))
        path=export_book([cmp,reports],csmp=True)
        cross=Work(defaults()).analyze(path,'CUMPLIMIENTO',as_of=date(2026,10,1))
        assert not cross.cross_missing and all('CUMPLIMIENTO.C10_HOJA2' in row.rules for row in cross.rows)
        print(json.dumps({'resultado':'OK','datos':'ficticios','filas_espera':len(work.rows),'filas_cumplimiento':len(cross.rows),
                          'exportacion_revisable':True,'procedencia_recuperada':True,'cruce_c10':True,'office_ejecutado':False,'red':False}))


if __name__=='__main__':main()
