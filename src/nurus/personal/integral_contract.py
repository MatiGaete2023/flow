"""Comprueba el descargador que realmente se ejecutará, sin iniciar una sesión RUS."""
import json,subprocess,sys,tempfile
from pathlib import Path

REQUIRED={'bitacoras_lectura':1,'descarga_conjunta':1,'pdf_seleccion':1}

def bundled_downloader():
    if not getattr(sys,'frozen',False):return None
    root=Path(sys.executable).resolve().parent.parent
    candidate=root/'SITFA_Descargador'/'SITFA_Descargador.exe'
    return candidate if candidate.is_file() else None

def validate(command,folder):
    folder=Path(folder);folder.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='compatibilidad-',dir=folder) as temp:
        target=Path(temp)/'capacidades.json'
        try:
            result=subprocess.run([*command,'--capacidades-integrales',str(target)],
                stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=15,
                creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
            if result.returncode!=0 or not target.is_file() or target.stat().st_size>8000:raise ValueError()
            data=json.loads(target.read_text(encoding='utf-8'))
            if (data.get('tipo')!='CSMP_RUS_CAPACIDADES' or data.get('protocolos')!=REQUIRED
                or data.get('escritura_rus') is not False):raise ValueError()
            if any(not isinstance(data.get(k),str) or not data[k] or len(data[k])>100 for k in ('version','commit')):raise ValueError()
            return data
        except (OSError,ValueError,subprocess.TimeoutExpired,AttributeError):
            raise ValueError('El descargador seleccionado no acredita el paquete integral de bitácoras, descarga CSMP y PDF. Selecciona SITFA_Descargador.exe incluido junto con este CSMP.') from None

def choose(app,*,preferred=''):
    from tkinter import filedialog
    from .download_link import launch_command
    bundled=bundled_downloader()
    candidate=str(bundled) if bundled else preferred or app.cfg.data.get('descargador_integral','')
    if candidate:
        try:
            command=launch_command(candidate);data=validate(command,app.cfg.directory/'compatibilidad')
            app.cfg.data['descargador_integral']=candidate;app.cfg.save(app.cfg.data)
            return candidate,command,data
        except ValueError:pass
    types=[('Descargador Windows','*.exe')]
    if not getattr(sys,'frozen',False):types.append(('Descargador Python','descargador.py'))
    executable=filedialog.askopenfilename(parent=app,title='Elegir descargador del paquete integral 2.7.1 o compatible',filetypes=types)
    if not executable:return None
    command=launch_command(executable);data=validate(command,app.cfg.directory/'compatibilidad')
    app.cfg.data['descargador_integral']=executable;app.cfg.save(app.cfg.data)
    return executable,command,data
