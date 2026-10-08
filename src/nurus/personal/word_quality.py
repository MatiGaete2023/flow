"""Completeness checks and conservative pagination for reviewed Word content."""
from pathlib import Path
import re
from zipfile import ZipFile
from lxml import etree

MARKERS=re.compile(r'\{\{[^{}]+\}\}|\[\s*COMPLETAR[^\]]*\]',re.I)


def unresolved_text(text):
    return sorted(set(MARKERS.findall(str(text))))


def pending_fields(document):
    found=[]
    # Validate every Word XML part, including headers, footers and text boxes.
    if isinstance(document,(str,Path)):
        with ZipFile(document) as archive:
            for name in archive.namelist():
                if name.startswith('word/') and name.endswith('.xml'):
                    root=etree.fromstring(archive.read(name),parser=etree.XMLParser(resolve_entities=False,no_network=True))
                    for p in root.iter('{http://schemas.openxmlformats.org/wordprocessingml/2006/main}p'):
                        found.extend(unresolved_text(''.join(n.text or '' for n in p.iter('{http://schemas.openxmlformats.org/wordprocessingml/2006/main}t'))))
    else:
        ns='{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
        for p in document.element.iter(ns+'p'):
            found.extend(unresolved_text(''.join(n.text or '' for n in p.iter(ns+'t'))))
    return sorted(set(found))


def require_complete(document,allow_review_markers=False):
    found=pending_fields(document)
    if allow_review_markers:found=[f for f in found if not f.upper().startswith('[COMPLETAR')]
    if found:raise ValueError('Completa los campos Word pendientes: '+', '.join(found))


def improve_pagination(document):
    from .resolutions import paragraphs
    for p in paragraphs(document):
        if not p.text.strip():continue
        fmt=p.paragraph_format
        if fmt.widow_control is None:fmt.widow_control=True
        if p.style and p.style.name.startswith('Heading') and fmt.keep_with_next is None:fmt.keep_with_next=True
