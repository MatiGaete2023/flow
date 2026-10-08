"""Controles y redacción de la pestaña Correos, inspirados en el creador CSMP v2.1."""
import re

from nurus.rus.rules import tribunal
from .modalities import MODALITIES
from .outputs import value


def lista_espanol(items):
    items=[str(item).strip() for item in items if str(item).strip()]
    if not items:return ''
    if len(items)==1:return items[0]
    if len(items)==2:
        conj='e' if re.match(r'(?i)^(i|hi)',items[1]) else 'y'
        return items[0]+' '+conj+' '+items[1]
    last=items[-1]
    conj='e' if re.match(r'(?i)^(i|hi)',last) else 'y'
    return ', '.join(items[:-1])+' '+conj+' '+last


def alcance_modalidades(keys):
    """Redacción semántica estable para una, varias o todas las modalidades."""
    selected=[label for key,label in MODALITIES.items() if key in set(keys or [])]
    if len(selected)==len(MODALITIES):return 'todas las modalidades'
    if len(selected)==1:return 'la modalidad '+selected[0]
    if selected:return 'las modalidades '+lista_espanol(selected)
    return ''


def selected_court_record_ids(work,court_keys):
    """Filtra el trabajo por los tribunales seleccionados en la interfaz."""
    allowed={str(key).strip().upper() for key in court_keys or []}
    if not allowed:return []
    result=[]
    for row in work.rows:
        raw=value(work,row,'tribunal')
        court=(tribunal(raw) or raw).strip().upper()
        if court in allowed:result.append(row.id)
    return result
