"""Comprobación del ejecutable instalado con configuración temporal aislada."""
from pathlib import Path
from tempfile import TemporaryDirectory
import json
from hashlib import sha256
import sys


def check(destination):
    from nurus import __version__
    from nurus.rus.catalog import load_catalog
    from .config import Configuration
    from .app import App
    from .template_package import _matrix_patches
    import nurus.personal as personal
    import xlrd,win32com.client,openpyxl
    destination=Path(destination).resolve();destination.parent.mkdir(parents=True,exist_ok=True)
    root=Path(personal.__file__).parent
    templates={p.relative_to(root/'plantillas_word').as_posix() for p in (root/'plantillas_word').rglob('*.docx')}
    expected={f'{court}/{kind}.docx' for court in ('LAJA','MULCHEN') for kind in ('NOMENCL','PC_IE','PC_INFO')}
    if templates!=expected:raise ValueError('Faltan matrices Word en el paquete: '+', '.join(sorted(expected-templates)))
    if len(_matrix_patches())!=5:raise ValueError('Faltan revisiones de matrices Word en el paquete.')
    if not load_catalog():raise ValueError('No se encuentra el catálogo de reglas del paquete.')
    with TemporaryDirectory(dir=destination.parent,prefix='csmp-check-') as folder:
        app=App(Configuration(Path(folder)))
        try:
            app.withdraw();app.update()
            for name in ('Trabajo','Correos','Resoluciones','Resultados','Configuración'):
                app.tabs.select(app.pages[name]);app.update()
            result={'version':__version__,'frozen':bool(getattr(sys,'frozen',False)),
                    'pages':list(app.pages),'templates':len(templates),'matrix_patches':5,'ok':True}
        finally:app.destroy()
    destination.write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding='utf-8')

def check_integral(destination,downloader):
    """Ejecuta ambos binarios y el retorno/exportación reales con un lote ficticio."""
    from datetime import date
    import subprocess,tempfile
    from types import SimpleNamespace
    import openpyxl
    from nurus import __version__
    from .integral_contract import validate,bundled_downloader,choose
    from .bitacora_link import finish,write
    from .bitacoras import earliest
    destination=Path(destination).resolve();destination.parent.mkdir(parents=True,exist_ok=True)
    downloader=Path(downloader or bundled_downloader()).resolve()
    if not getattr(sys,'frozen',False):raise ValueError('Esta prueba requiere los ejecutables Windows entregados.')
    with tempfile.TemporaryDirectory(prefix='integracion-ficticia-',dir=destination.parent) as temp:
        folder=Path(temp);lot=folder/'lotes';lot.mkdir()
        capabilities=validate([str(downloader)],folder)
        app=SimpleNamespace(cfg=SimpleNamespace(directory=folder,data={},save=lambda data:None))
        selected=choose(app)
        if Path(selected[0]).resolve()!=downloader:raise ValueError('CSMP no eligió el descargador del mismo paquete.')
        result=subprocess.run([str(downloader),'--generar-lote-prueba',str(lot)],timeout=60,
            creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
        if result.returncode:raise ValueError('No se generó el lote ficticio con el descargador entregado.')
        reply=json.loads((lot/'prueba.json').read_text(encoding='utf-8'))
        today=date.today();request={'id':'ficticio','folder':str(folder),'desde':earliest(today).isoformat(),
            'hasta':today.isoformat(),'corte':today.isoformat(),'destino':str(folder/'Bitacoras.xlsx'),
            'descargador':capabilities,'estado':'PENDIENTE'}
        write(folder/'respuesta.json',{**reply,'id':request['id']})
        app=SimpleNamespace(_run=lambda text,action,done:done(action()),status=SimpleNamespace(set=lambda text:None))
        finish(app,request)
        if request['estado']!='EXPORTADA':raise ValueError('CSMP no completó el retorno del lote.')
        book=openpyxl.load_workbook(request['archivo'])
        try:
            assert book['Copia íntegra']['M2'].value=='Texto completo'
            assert book['Copia íntegra']['K2'].value==1
            assert book['Copia íntegra']['M2'].border.left.style=='thin'
            assert book['Lecturas']['K2'].value=='LEIDA'
            assert book['Consultas']['D2'].value=='ENUMERADA'
            sheets=book.sheetnames
        finally:book.close()
        proof={'ok':True,'datos':'ficticios','csmp_version':__version__,'descargador':capabilities,
            'csmp_exe_sha256':sha256(Path(sys.executable).read_bytes()).hexdigest(),
            'descargador_exe_sha256':sha256(downloader.read_bytes()).hexdigest(),
            'seleccion_descargador_del_paquete':True,'retorno_lote_verificado':True,'excel_generado_por_csmp':True,'texto_completo':True,
            'cc_al_tribunal_1':True,'bordes':True,'hojas':sheets,'peticiones_reales_rus':0,'escrituras_rus':0}
    write(destination,proof)
