"""Generate fictitious current products and capture the real Resultados view."""
import os,json
from pathlib import Path
from tempfile import TemporaryDirectory
from hashlib import sha256
from nurus.personal.config import Configuration
from nurus.personal.app import App
from nurus.personal.work import Work
from nurus.personal.outputs import Draft,draft_fingerprint
from nurus.personal.resolutions import Project
from nurus.personal.product_checks import project_hash
from nurus.personal.activity import receipt
from nurus.personal.reports import _book
from nurus.personal.manual import generate_word
from nurus.personal.delivery import selected_zip


def showcase(destination):
    destination=Path(destination).resolve();destination.mkdir(parents=True,exist_ok=True)
    roster=destination/'Nomina_ficticia.xlsx'
    _book([('Nómina',['RIT','TRIBUNAL','RUT','NOMBRE','PROGRAMA','OBSERVACION'],[
        ['X-1-2026','Tribunal ficticio','00111111-1','Persona ficticia uno','Programa de ejemplo','Observación extensa de demostración. '*5],
        ['X-2-2026','Tribunal ficticio','00222222-2','Persona ficticia dos','Programa de ejemplo','=Texto literal que debe permanecer como texto']])],roster)
    source=destination/'Trabajo_ficticio.xlsx'
    _book([('ESPERA',['RIT','TRIBUNAL','RUT','NOMBRE','DERIVACION','T ESPERA'],[['X-1-2026','Tribunal ficticio','00111111-1','Persona ficticia uno','Programa de ejemplo',40]])],source)
    word=destination/'Documento_ficticio.docx'
    generate_word(word,'Proyecto ficticio de revisión','Este documento contiene datos ficticios.\n\nTexto de demostración para comprobar el producto Word y su paginación.')
    selected_zip([roster,word],destination/'Entrega_ficticia.zip')
    with TemporaryDirectory(prefix='csmp-showcase-') as folder:
        app=App(Configuration(Path(folder)));app.title('CSMP · demostración con datos ficticios')
        try:
            app.work=Work.external(source,app.cfg.data,mode='ESPERA');app._show_work()
            draft=Draft('Comunicación · Programa de ejemplo','Buen día:\n\nSe adjunta nómina ficticia.',to='prueba@example.test',program='Programa de ejemplo',attachments=[str(roster)])
            draft.roster=[{'program':'Programa de ejemplo','generated':str(roster),'rows':[],'headers':[]}]
            second=Draft('Comunicación · Por completar','Buen día:\n\nSegundo borrador ficticio.',program='Programa por completar')
            app._display_prepared_drafts([draft,second],'{count} productos ficticios')
            app.work.receipts[draft_fingerprint(draft)]=receipt('draft','created',subject=draft.subject,entry_id='ficticio',store_id='ficticio')
            project=Project('ejemplo','FICTICIO','X-1-2026','PC_IE',[app.work.rows[0].id],str(word),{},'Texto de demostración','Texto de demostración')
            app.projects=[project];app.work.receipts['example-word']=receipt('word','generated',record_ids=project.record_ids,path=str(word),product_id=project.key,product_hash=project_hash(project),type=project.kind)
            for theme in ('Claro','Oscuro'):
                app.theme_choice.set(theme);app.apply_appearance();app.geometry('1120x740');app.tabs.select(app.pages['Resultados']);app.update()
                app.results_tree.selection_set('draft:'+second.product_id);app.update()
                if os.name=='nt':
                    from window_capture import capture_window
                    capture_window(app,'resultados-'+theme.lower()+'-ficticio')
                else:
                    from PIL import ImageGrab
                    bbox=(app.winfo_rootx(),app.winfo_rooty(),app.winfo_rootx()+app.winfo_width(),app.winfo_rooty()+app.winfo_height())
                    ImageGrab.grab(bbox=bbox,xdisplay=os.environ.get('DISPLAY')).save(destination/('Resultados_'+theme+'.png'))
        finally:app.destroy()
    return str(destination)


if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser();p.add_argument('--output',type=Path,default=Path('artifacts/productos-ficticios'));args=p.parse_args();print(showcase(args.output))
