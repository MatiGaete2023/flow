"""Corte vertical reproducible y benchmark: usa exclusivamente fixtures sintéticas."""
import argparse
from datetime import date
from hashlib import sha256
import json
from pathlib import Path
import sys
from time import perf_counter
import tracemalloc
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'src'))
from flow_capture_fixture import fixture
from nurus.capture import write_capture,read_capture
from nurus.personal.config import defaults
from nurus.personal.capture_input import import_capture
from nurus.personal.capture_products import export_requests,export_evidence
from nurus.personal.outputs import prepare_required_drafts
from nurus.personal.resolutions import prepare_projects,generate_projects


def run(root,count,partial=True):
    root.mkdir(parents=True,exist_ok=False);m,t=fixture(count,partial=partial)
    stages={};tracemalloc.start()
    start=perf_counter();package=write_capture(root/'Captura_CSMP.zip',m,t);stages['export_zip']=perf_counter()-start
    start=perf_counter();capture=read_capture(package);stages['validate']=perf_counter()-start
    config=defaults();config['contactos']={'programas':{},'tribunales':{}}
    start=perf_counter();work=import_capture(package,config,as_of=date(2026,10,8));stages['import_normalize_analyze']=perf_counter()-start
    start=perf_counter();work.export(root/'Observaciones.xlsx',backend='portable',reduced_fidelity=True);stages['excel']=perf_counter()-start
    export_requests(work,root/'solicitud_antecedentes.json');export_evidence(work,root/'Cobertura.xlsx')
    start=perf_counter();drafts=prepare_required_drafts(work,modalities='Residencial, Ambulatorio, FAE y DCE',period='octubre 2026',defer_attachments=True)
    # Descriptores de borrador; jamás instancia COM/Outlook ni envía mensajes.
    for draft in drafts:draft.to='ficticio@example.test';draft.cc='copia@example.test'
    (root/'Borradores.json').write_text(json.dumps([{'kind':d.kind,'subject':d.subject,'to':d.to,'cc':d.cc,'body':d.body,'record_ids':d.record_ids,'attachments_pending':bool(d.roster),'requires_human_review':True} for d in drafts],ensure_ascii=False,indent=2,default=str),encoding='utf-8')
    stages['drafts']=perf_counter()-start
    # Los proyectos siguen siendo documentos revisables con marcadores explícitos.
    if count<=100:
        from nurus.personal.resolutions import automatic_project_selections
        from nurus.personal.config import BASE
        from nurus.personal.roster import generate
        projects,errors=prepare_projects(work,automatic_project_selections(work),BASE/'plantillas_word')
        for project in projects:project.review_required=False
        if projects:generate_projects(work,projects,root/'Proyectos_revisables.docx')
        for draft in drafts:
            if draft.roster:generate(draft,root)
        (root/'Proyectos_pendientes.json').write_text(json.dumps(errors,ensure_ascii=False),encoding='utf-8')
    work.save(root/'sesion');current,peak=tracemalloc.get_traced_memory();tracemalloc.stop()
    result={'rows':count,'partial':partial,'schema':capture.manifest['schema_version'],'stages_seconds':{k:round(v,4) for k,v in stages.items()},'peak_bytes':peak,'package_bytes':package.stat().st_size,'reads_institutional':0,'writes_institutional':0,'outlook_saved':0}
    (root/'benchmark.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8');return result

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('output',type=Path);parser.add_argument('--rows',type=int,nargs='+',default=[100,500,2000]);parser.add_argument('--complete',action='store_true');args=parser.parse_args()
    if any(n<1 or n>20000 for n in args.rows):parser.error('Filas entre 1 y 20.000.')
    results=[run(args.output/str(count),count,partial=not args.complete) for count in args.rows]
    print(json.dumps(results,indent=2))
