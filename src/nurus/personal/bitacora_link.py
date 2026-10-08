"""Intercambio independiente del trabajo de correos y propuestas."""
from datetime import date
from hashlib import sha256
import json
from pathlib import Path
import subprocess
import sys
from tkinter import filedialog
from uuid import uuid4
from .download_link import launch_command
from .bitacoras import validate_period
from .bitacora_lote import export_lote

def write(path,data):
    temp=path.with_suffix('.tmp');temp.write_text(json.dumps(data,ensure_ascii=False,indent=2),encoding='utf-8');temp.replace(path)

def verify_reply(request):
    reply=Path(request['folder'])/'respuesta.json'
    if reply.stat().st_size>64000:raise ValueError('Respuesta de lectura demasiado grande.')
    data=json.loads(reply.read_text(encoding='utf-8'))
    if data.get('id')!=request['id']:raise ValueError('La respuesta corresponde a otra solicitud.')
    manifest=Path(data['manifest']).resolve();allowed=(Path(request['folder'])/'lotes').resolve()
    if not manifest.is_relative_to(allowed) or manifest.name!='bitacoras.json' or manifest.stat().st_size>32*1024*1024:
        raise ValueError('El lote está fuera de la carpeta de esta solicitud.')
    if sha256(manifest.read_bytes()).hexdigest()!=data.get('sha256'):raise ValueError('El control del lote cambió después de la consulta.')
    expected=request.get('descargador')
    if expected:
        origin=json.loads(manifest.read_text(encoding='utf-8')).get('origen',{})
        if any(origin.get(k)!=expected.get(k) for k in ('version','commit')):
            raise ValueError('El lote proviene de otro descargador; recupera la solicitud correspondiente.')
    return manifest

def finish(app,request):
    manifest=verify_reply(request)
    def action():
        start,end=date.fromisoformat(request['desde']),date.fromisoformat(request['hasta'])
        # Mantener el período pedido al iniciar, aun si se recupera días después.
        return export_lote(manifest,request['destino'],start,end,today=date.fromisoformat(request['corte']))
    def done(path):
        request['estado']='EXPORTADA';request['archivo']=str(path)
        write(Path(request['folder'])/'solicitud.json',request)
        app.status.set('Excel de bitácoras creado: '+str(path)+'. Revisa las hojas Lecturas y Consultas para conocer las fallas.')
    app._run('Creando Excel con copias de bitácoras y control de lecturas…',action,done)

def start(app,start,end):
    if app.busy:raise ValueError('Espera a que termine la operación actual.')
    active=getattr(app,'_diary_process',None)
    if active is not None and active.poll() is None:raise ValueError('La consulta de bitácoras ya está abierta.')
    validate_period(start,end)
    destination=filedialog.asksaveasfilename(parent=app,title='Excel de bitácoras RUS',defaultextension='.xlsx',initialfile='Bitacoras_RUS.xlsx')
    if not destination:return
    from .integral_contract import choose
    chosen=choose(app)
    if not chosen:return
    executable,command,capabilities=chosen
    folder=app.cfg.directory/'intercambio_bitacoras'/uuid4().hex;folder.mkdir(parents=True)
    request={'version':1,'id':folder.name,'folder':str(folder.resolve()),'desde':start.isoformat(),'hasta':end.isoformat(),
             'corte':date.today().isoformat(),'destino':str(Path(destination).resolve()),'estado':'PENDIENTE'}
    request['descargador']={'ruta':str(Path(executable).resolve()),'version':capabilities['version'],'commit':capabilities['commit']}
    write(folder/'solicitud.json',request)
    command+=['--bitacoras-reply',str(folder/'respuesta.json'),'--bitacoras-id',request['id'],'--destino',str(folder/'lotes')]
    process=subprocess.Popen(command,cwd=str(Path(executable).resolve().parent),creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
    app._diary_process=process
    app.status.set('Conecta Chrome, carga Seguimiento, elige tribunales/modalidades/pestañas y pulsa Leer bitácoras del lote.')
    def poll():
        if not app.winfo_exists():return
        if (folder/'respuesta.json').is_file():
            if app.busy:app.after(700,poll)
            else:app._guard(lambda:finish(app,request))
            return
        if process.poll() is not None:
            app.status.set('La solicitud queda conservada. Recuperar Excel de bitácoras permite incorporar un resultado pendiente.')
            return
        app.after(700,poll)
    app.after(700,poll)

def recover(app):
    path=filedialog.askopenfilename(parent=app,title='Elegir solicitud.json de bitácoras',
        initialdir=str(app.cfg.directory/'intercambio_bitacoras'),filetypes=[('Solicitud','solicitud.json')])
    if not path:return
    if Path(path).stat().st_size>64000:raise ValueError('Solicitud demasiado grande.')
    request=json.loads(Path(path).read_text(encoding='utf-8'))
    if request.get('version')!=1 or Path(request.get('folder','')).resolve()!=Path(path).resolve().parent:
        raise ValueError('Solicitud fuera de su carpeta.')
    if request.get('estado')=='EXPORTADA':
        app.status.set('Esta solicitud ya se exportó: '+request['archivo']);return
    finish(app,request)
