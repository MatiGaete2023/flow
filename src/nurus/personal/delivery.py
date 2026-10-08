"""Current delivery with readable index and verifiable file manifest."""
from pathlib import Path
from hashlib import sha256
from zipfile import ZipFile, ZIP_DEFLATED
import json
from nurus.services.file_output import write_new_file
from .operations import progress,checkpoint


def digest(path):
    result=sha256()
    with Path(path).open('rb') as source:
        for block in iter(lambda:source.read(1024*1024),b''):result.update(block)
    return result.hexdigest()


def validate_product(path):
    path=Path(path)
    if not path.is_file() or path.stat().st_size==0:raise ValueError('Producto no disponible o vacío: '+path.name)
    if path.suffix.lower() in ('.xlsx','.docx','.zip'):
        with ZipFile(path) as archive:
            broken=archive.testzip()
            if broken:raise ValueError('Producto incompleto: '+path.name)
            required={'[Content_Types].xml'} if path.suffix.lower() in ('.xlsx','.docx') else set()
            if not required<=set(archive.namelist()):raise ValueError('Formato de producto inválido: '+path.name)
    if path.suffix.lower()=='.docx':
        from .word_quality import require_complete
        require_complete(path)
    if path.suffix.lower()=='.pdf':
        with path.open('rb') as f:
            if not f.read(5)==b'%PDF-':raise ValueError('PDF inválido: '+path.name)


def selected_zip(paths,destination,metadata=None):
    sources=list(dict.fromkeys(Path(p).resolve() for p in paths))
    target=Path(destination).resolve()
    if not sources:raise ValueError('Selecciona productos del trabajo actual.')
    if target in sources:raise ValueError('El ZIP no puede reemplazar un producto seleccionado.')
    entries=[]
    for index,path in enumerate(sources,1):
        checkpoint();validate_product(path)
        folder={'.xlsx':'Nominas','.docx':'Resoluciones','.pdf':'PDF','.csv':'Tablas','.ics':'Calendario'}.get(path.suffix.lower(),'Otros')
        info=(metadata or {}).get(str(path),{})
        entries.append({'archivo':f'{folder}/{index:03d}_{path.name}','nombre':path.name,'tipo':folder,'bytes':path.stat().st_size,'sha256':digest(path),'estado':'VERIFICADO',**{k:info[k] for k in ('registros','producto') if k in info}})
    manifest={'version':1,'alcance':'Entrega del trabajo actual','estado':'VERIFICADO','archivos':entries}
    index_text=['ENTREGA DEL TRABAJO ACTUAL','Archivos incluidos: '+str(len(entries)),
                'Cada archivo fue comprobado antes de incluirlo.','Los borradores permanecen en Outlook; este ZIP no acredita envío ni registro RUS.','']
    for item in entries:index_text.append(f"{item['archivo']} | {item['bytes']} bytes | VERIFICADO"+(f" | Registros: {item['registros']}" if 'registros' in item else ''))
    def write(temporary):
        with ZipFile(temporary,'w',ZIP_DEFLATED) as archive:
            for index,(path,item) in enumerate(zip(sources,entries),1):
                checkpoint();progress('Preparando entrega',index-1,len(entries))
                archive.write(path,item['archivo'])
            archive.writestr('INDICE.txt','\n'.join(index_text))
            archive.writestr('MANIFIESTO.json',json.dumps(manifest,ensure_ascii=False,indent=2))
        # Reopen after closing: Windows ZipInfo normalizes arcname separators only
        # in the serialized directory, so reading while writing is not portable.
        with ZipFile(temporary,'r') as archive:
            for item in entries:
                h=sha256()
                with archive.open(item['archivo']) as f:
                    for block in iter(lambda:f.read(1024*1024),b''):h.update(block)
                if h.hexdigest()!=item['sha256']:raise ValueError('El archivo cambió al preparar la entrega: '+item['nombre'])
    write_new_file(target,write)
    progress('Entrega preparada',len(entries),len(entries))
    return str(target)
