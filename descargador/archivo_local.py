"""Historial de referencias locales: consulta y exportación sin sesión Chrome."""
from pathlib import Path
from resultados import read_json,load_lot
import motor as m


def kind(folder):
    folder=Path(folder)
    for name,label in [('bitacoras.json','Bitácoras'),('flujo.json','Conjunto'),('resumen.json','Descarga')]:
        if (folder/name).is_file():return name,label
    raise m.PocError('La carpeta debe conservar resumen.json, flujo.json o bitacoras.json y sus originales.')


def remember(prefs,folder):
    folder=Path(folder).resolve();name,label=kind(folder)
    key='historial_bitacoras' if label=='Bitácoras' else 'historial_lotes'
    value=str(folder/name) if label=='Bitácoras' else str(folder)
    paths=prefs.data.setdefault(key,[])
    if value not in paths:paths.append(value);prefs.save()
    return folder


def discover(base,prefs,last=None):
    folders={Path(p).resolve() for p in prefs.data.get('historial_lotes',[]) if isinstance(p,str)}
    folders.update(Path(p).resolve().parent for p in prefs.data.get('historial_bitacoras',[]) if isinstance(p,str))
    base=Path(base)
    if base.is_dir():
        folders.add(base.resolve());folders.update(p.resolve() for p in base.iterdir() if p.is_dir())
    if last:folders.add(Path(last).resolve())
    result=[]
    for folder in sorted(folders,reverse=True):
        try:
            name,label=kind(folder);data=read_json(folder/name)
            count=len(data.get('registros',[])) if label=='Bitácoras' else data.get('consultas_completadas',len(data.get('fases',{})))
            result.append((folder,label,data.get('estado','DESCONOCIDO'),count))
        except (m.PocError,OSError,ValueError):continue
    return result


def load_sources(folders,partial=False):
    """Abre descargas anteriores y fases de conjuntos con sus hashes originales."""
    result=[];seen=set()
    for folder in folders:
        folder=Path(folder).resolve();_,label=kind(folder)
        if label=='Bitácoras':continue
        if label=='Conjunto':
            from flujo_csmp import source_folder
            data=read_json(folder/'flujo.json')
            if data.get('estado') not in ('VALIDADA','COMPLETA') and not partial:
                raise m.PocError('El conjunto está incompleto. Permite resultados incompletos para conservar su alcance explícito.')
            paths=[source_folder(folder,p['carpeta']) for p in data.get('fases',{}).values() if p.get('carpeta')]
        else:paths=[folder]
        for path in paths:
            if path in seen:continue
            seen.add(path);result.append(load_lot(path,partial))
    if not result:raise m.PocError('Añade al menos una descarga de hojas RUS para crear el paquete del Asistente.')
    return result
