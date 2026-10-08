"""Faithful preview through isolated desktop Word; no guessed DOCX layout."""
from pathlib import Path
import os
from nurus.services.file_output import write_new_file
from .word_quality import require_complete


def export_pdf(source,destination):
    source=Path(source).resolve();target=Path(destination).resolve()
    if not source.is_file():raise ValueError('Genera primero el documento Word.')
    require_complete(source)
    if os.name!='nt':raise ValueError('La vista previa necesita Word instalado en Windows.')
    import pythoncom
    import win32com.client
    pythoncom.CoInitialize();word=None;doc=None
    try:
        word=win32com.client.DispatchEx('Word.Application');word.Visible=False;word.DisplayAlerts=0
        word.AutomationSecurity=3
        doc=word.Documents.Open(str(source),ReadOnly=True,AddToRecentFiles=False)
        write_new_file(target,lambda p:doc.ExportAsFixedFormat(str(p),17))
    finally:
        try:
            if doc is not None:doc.Close(False)
        finally:
            try:
                if word is not None:word.Quit()
            finally:pythoncom.CoUninitialize()
    return str(target)


def preview(app):
    from uuid import uuid4
    from .product_checks import check_products
    checks=check_products(app.work,app.drafts,app.projects)
    selected=app.projects[app.project_index] if app.project_index is not None else None
    check=next((c for c in checks if selected and c.id=='word:'+selected.key and c.path and not c.issues),None)
    if not check:raise ValueError('Genera el Word revisado antes de solicitar su vista PDF.')
    target=app.cfg.directory/'vista_previa'/(uuid4().hex+'.pdf')
    app._run('Creando vista PDF con Word…',lambda:export_pdf(check.path,target),lambda p:app._open(p))
