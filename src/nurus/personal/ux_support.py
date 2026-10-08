"""Ayudas UX puras para el CSMP Assistant personal.

No contiene reglas de dominio: solo presenta estado que el trabajo ya conoce.
"""
from __future__ import annotations

from pathlib import Path

from nurus.rus.columns import normalize
from .outputs import value, draft_fingerprint
from .resolutions import KINDS, resolution_kind


RES_HELP = {
    'PC_IE': 'Pide cuenta por ingreso efectivo',
    'PC_INFO': 'Pide cuenta por informe',
    'NOMENCL': 'Proyecto de nomenclatura',
}


def technical_value(row, key):
    review=getattr(row,'review',{}) or {}
    if key in review:
        raw=review.get(key,'')
    else:
        wanted=normalize(key)
        raw=next((v for k,v in getattr(row,'values',{}).items() if normalize(k)==wanted),'')
    return '' if raw is None else str(raw)


def incident_ids(work):
    return [row.id for row in getattr(work,'rows',[]) if getattr(row,'warnings',None)]


def _word_done(work,row_id):
    for receipt in getattr(work,'receipts',{}).values():
        if receipt.get('kind')!='word':
            continue
        ids=receipt.get('record_ids') or [receipt.get('record_id')]
        if row_id in ids:
            return True
    return False


def _mail_state(work,row_id,drafts):
    prepared=False
    created=False
    receipts=getattr(work,'receipts',{})
    for draft in drafts or []:
        if row_id not in getattr(draft,'record_ids',[]):
            continue
        prepared=True
        key=getattr(draft,'key','') or draft_fingerprint(draft)
        if receipts.get(key,{}).get('state')=='created':
            created=True
    return 'created' if created else 'prepared' if prepared else ''


def _resolution_codes(row):
    codes=[]
    explicit=resolution_kind(technical_value(row,'RES'))
    if explicit:
        codes.append(explicit)
    for action in getattr(row,'actions',[]) or []:
        if action in KINDS and action not in codes:
            codes.append(action)
    return codes


def resume_available(work):
    output=getattr(work,'output','')
    return bool(output and Path(output).is_file())


def product_indicators(work,row,drafts=None):
    items=['Excel ✓' if resume_available(work) else 'Excel pendiente']
    mail=_mail_state(work,row.id,drafts)
    if mail=='created':
        items.append('Correo ✓')
    elif mail=='prepared':
        items.append('Correo preparado')
    codes=_resolution_codes(row)
    if codes:
        items.extend('RES '+code for code in codes)
        items.append('Word ✓' if _word_done(work,row.id) else 'Word pendiente')
    elif _word_done(work,row.id):
        items.append('Word ✓')
    return items


def case_detail_lines(work,row,drafts=None):
    fields=[
        ('RIT',value(work,row,'rit')),
        ('NNA',value(work,row,'nombre')),
        ('Tribunal',value(work,row,'tribunal')),
        ('Programa',value(work,row,'programa')),
        ('Modalidad',value(work,row,'modalidad')),
        ('Observación final',(getattr(row,'review',{}) or {}).get('OBSERVACION',getattr(row,'observation',''))),
        ('TT',technical_value(row,'TT')),
        ('CC',technical_value(row,'CC')),
        ('RES',technical_value(row,'RES')),
    ]
    state='Excluido' if getattr(row,'excluded',False) else 'Revisar aviso' if getattr(row,'warnings',None) else 'Propuesta'
    fields.append(('Estado',state))
    fields.append(('Productos',' · '.join(product_indicators(work,row,drafts))))
    if getattr(row,'warnings',None):
        fields.append(('Incidencias','; '.join(dict.fromkeys(str(x) for x in row.warnings))))
    return [(label,str(data)) for label,data in fields if str(data or '').strip()]


def case_detail_text(work,row,drafts=None):
    return '\n'.join(f'{label}: {data}' for label,data in case_detail_lines(work,row,drafts))


def compact_case_detail_text(work,row,drafts=None):
    """Detalle breve para Trabajo; evita repetir la observación editable."""
    data=dict(case_detail_lines(work,row,drafts))
    groups=[
        ('RIT','NNA','Tribunal'),
        ('Programa','Modalidad'),
        ('TT','CC','RES','Estado','Productos'),
        ('Incidencias',),
    ]
    lines=[]
    for group in groups:
        parts=[f'{label}: {data[label]}' for label in group if data.get(label)]
        if parts:
            lines.append(' · '.join(parts))
    return '\n'.join(lines)


def grouped_draft_detail(work,draft,drafts=None):
    ids=list(dict.fromkeys(getattr(draft,'record_ids',[]) or []))
    by_id={row.id:row for row in getattr(work,'rows',[])}
    rows=[by_id[rid] for rid in ids if rid in by_id]
    if len(rows)==1:
        return case_detail_text(work,rows[0],drafts)
    if not rows:
        return 'Sin causa asociada disponible en el trabajo actual.'
    lines=[f'Registros relacionados: {len(rows)}']
    for row in rows[:5]:
        rit=value(work,row,'rit');name=value(work,row,'nombre')
        label=' · '.join(part for part in (rit,name) if part)
        if label:lines.append(label)
    if len(rows)>5:lines.append(f'… y {len(rows)-5} más')
    return '\n'.join(lines)


def resume_description(work, saved_path):
    source=Path(getattr(work,'path','') or '').name or 'sin archivo'
    mode=str(getattr(work,'mode','') or 'TRABAJO')
    try:
        stamp=Path(saved_path).stat().st_mtime
        from datetime import datetime
        when=datetime.fromtimestamp(stamp).strftime('%d/%m/%Y %H:%M')
    except OSError:
        when=''
    return f'{mode} · {source}' + (f' · {when}' if when else '')
