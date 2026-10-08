"""Profile a fictitious Excel/model/report workflow, not user documents."""
import argparse,json,platform,time
from pathlib import Path
from tempfile import TemporaryDirectory
from openpyxl import Workbook
from nurus.rus.reader import read_workbook
from nurus.personal.work import Work
from nurus.personal.config import defaults
from nurus.personal.reports import _book


def profile(rows,output):
    from pyinstrument import Profiler
    output=Path(output);output.mkdir(parents=True,exist_ok=True)
    with TemporaryDirectory(prefix='csmp-profile-') as folder:
        source=Path(folder)/'fixture.xlsx';book=Workbook(write_only=True);s=book.create_sheet('ESPERA')
        headers=['RIT','TRIBUNAL','NOMBRE','RUT','DERIVACION','T ESPERA'];s.append(headers)
        for n in range(rows):s.append(['X-'+str(n)+'-2026','LAJA','Persona ficticia '+str(n),'00111111-1','PRM ficticio',40])
        book.save(source);book.close();durations={};profiler=Profiler();profiler.start()
        start=time.perf_counter();batch=read_workbook(source,'ESPERA');durations['leer_excel']=time.perf_counter()-start
        start=time.perf_counter();work=Work(defaults()).analyze(source,'ESPERA',batch=batch);durations['analizar_sin_relectura']=time.perf_counter()-start
        start=time.perf_counter();_book([('Nómina',['RIT','NOMBRE','OBSERVACION'],[[r.values['RIT'],r.values['NOMBRE'],r.observation] for r in work.rows])],Path(folder)/'report.xlsx');durations['crear_nomina']=time.perf_counter()-start
        profiler.stop();(output/'Perfil_trabajo.html').write_text(profiler.output_html(),encoding='utf-8')
        (output/'Perfil_trabajo.json').write_text(json.dumps({'filas':rows,'equipo':platform.platform(),'segundos':durations,'nota':'Una ejecución ficticia con muestreo; no mide Office ni equivale a un benchmark repetido.'},ensure_ascii=False,indent=2),encoding='utf-8')
    print(json.dumps(durations,ensure_ascii=False))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--rows',type=int,default=1000);p.add_argument('--output',type=Path,default=Path('artifacts/perfil'));args=p.parse_args()
    if not 1<=args.rows<=50000:p.error('Usa entre 1 y 50.000 filas.')
    profile(args.rows,args.output)
