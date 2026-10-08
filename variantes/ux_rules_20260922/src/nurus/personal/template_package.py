"""Importación y actualización local de matrices proporcionadas por el usuario."""
from pathlib import Path
from zipfile import ZipFile
from io import BytesIO
from docx import Document
import base64
import hashlib
import json
import re
import shutil
import zlib
from nurus.rus.columns import normalize

BASE=Path(__file__).parent
BUNDLED_REVISION='2026-09-14-matrices-v1'
PATCH_FILE=BASE/'matrix_document_patches.json'

def _matrix_patches():
    data=json.loads(PATCH_FILE.read_text(encoding='utf-8'))
    if data.get('revision')!=BUNDLED_REVISION:
        raise ValueError('La revisión de matrices empaquetada no coincide con la esperada.')
    result={}
    for relative,item in data['templates'].items():
        patch_path=BASE/item['file']
        try:
            encoded=patch_path.read_text(encoding='ascii').strip()
            xml=zlib.decompress(base64.b64decode(encoded,validate=True))
        except Exception as exc:
            raise ValueError('Parche de matriz no utilizable: '+relative) from exc
        digest=hashlib.sha256(xml).hexdigest()
        if digest!=item['sha256']:
            raise ValueError('Parche de matriz alterado: '+relative)
        result[Path(relative)]=(xml,digest)
    return result

def _document_hash(path):
    with ZipFile(path) as archive:
        return hashlib.sha256(archive.read('word/document.xml')).hexdigest()

def _replace_document_xml(path,xml,expected):
    path=Path(path);temp=path.with_name(path.name+'.tmp')
    try:
        with ZipFile(path) as source, ZipFile(temp,'w') as target:
            found=False
            for info in source.infolist():
                content=source.read(info.filename)
                if info.filename=='word/document.xml':content=xml;found=True
                target.writestr(info,content)
            if not found:raise ValueError('La matriz no contiene word/document.xml: '+str(path))
        Document(temp)
        if _document_hash(temp)!=expected:raise ValueError('No se pudo verificar la matriz reconstruida: '+str(path))
        temp.replace(path)
    finally:
        temp.unlink(missing_ok=True)

def _backup(target,revision):
    suffix=revision.replace('-','_')
    backup=target.with_name(target.stem+'.pre_'+suffix+'.bak.docx')
    if not backup.exists():shutil.copyfile(target,backup)
    return backup

def install_bundled_templates(source,directory,revision=BUNDLED_REVISION):
    """Instala una revisión una sola vez y respalda las matrices reemplazadas.

    La revisión 2026-09-14 sustituye únicamente el cuerpo XML de las cinco matrices
    entregadas por el usuario. Tras registrar la revisión, una edición manual
    posterior se conserva. Revisiones genéricas mantienen el comportamiento previo.
    """
    source=Path(source);root=Path(directory);root.mkdir(parents=True,exist_ok=True)
    marker=root/'.bundled_revision'
    previous=marker.read_text(encoding='utf-8').strip() if marker.exists() else ''
    migrating=previous!=revision
    patches=_matrix_patches() if revision==BUNDLED_REVISION else {}
    installed=[];backups=[];preserved=[];patched=[];missing=[];existed={}
    if source.exists():
        for path in sorted(source.rglob('*.docx')):
            relative=path.relative_to(source);target=root/relative;target.parent.mkdir(parents=True,exist_ok=True)
            existed[relative]=target.exists()
            if not target.exists():
                shutil.copyfile(path,target);installed.append(str(target));continue
            if revision!=BUNDLED_REVISION and migrating and target.read_bytes()!=path.read_bytes():
                backup=_backup(target,revision);backups.append(str(backup))
                shutil.copyfile(path,target);installed.append(str(target));continue
            preserved.append(str(target))
    for relative,(xml,digest) in patches.items():
        target=root/relative
        if not target.exists():
            baseline=source/relative
            if not baseline.exists():missing.append(str(relative));continue
            target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(baseline,target);installed.append(str(target));existed[relative]=False
        current=_document_hash(target)
        if current==digest:continue
        # Una edición manual se preserva cuando la misma revisión ya fue aplicada.
        # Si el archivo faltaba y acaba de reponerse desde la base, sí debe recibir
        # nuevamente el cuerpo de la revisión vigente.
        if not migrating and existed.get(relative,True):continue
        if existed.get(relative,True):
            backup=_backup(target,revision);backups.append(str(backup))
        _replace_document_xml(target,xml,digest);patched.append(str(target))
    if missing:raise ValueError('Faltan matrices base para aplicar la revisión: '+', '.join(missing))
    if migrating:
        temp=marker.with_suffix('.tmp');temp.write_text(revision,encoding='utf-8');temp.replace(marker)
    return {'revision':revision,'installed':installed,'backups':backups,'preserved':preserved,'patched':patched}

def import_templates(path,directory):
    root=Path(directory);pending={};unmatched=[]
    with ZipFile(path) as archive:
        for item in archive.infolist():
            if item.is_dir() or not item.filename.lower().endswith('.docx'):continue
            name=normalize(item.filename.replace('_',' ').replace('\\','/')).upper()
            courts=[c for c in ('LAJA','MULCHEN','TOME') if re.search(r'\b'+c+r'\b',name)]
            kinds=[k for k,pattern in [('PC_IE',r'\bPC IE\b'),('PC_INFO',r'\bPC INFO\b'),('NOMENCL',r'\bNOMENCL(?:ATURA)?\b')] if re.search(pattern,name)]
            if len(courts)!=1 or len(kinds)!=1:unmatched.append(item.filename);continue
            if item.file_size>20_000_000:raise ValueError('Matriz demasiado grande: '+item.filename)
            content=archive.read(item)
            Document(BytesIO(content))
            key=(courts[0],kinds[0])
            if key in pending:raise ValueError('Hay dos matrices para '+('/'.join(key))+'. Conserva una versión por tipo.')
            pending[key]=content
    imported=[]
    for (court,kind),content in pending.items():
        target=root/court/(kind+'.docx');target.parent.mkdir(parents=True,exist_ok=True)
        if target.exists():shutil.copyfile(target,target.with_suffix('.bak.docx'))
        temp=target.with_suffix('.tmp');temp.write_bytes(content);temp.replace(target)
        imported.append(str(target))
    return {'imported':imported,'unmatched':unmatched}
