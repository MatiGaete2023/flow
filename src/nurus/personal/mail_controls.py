"""Controles y redacción de la pestaña Correos, inspirados en el creador CSMP v2.1."""
import re

from .courts import court_key
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
    allowed={court_key(key) for key in court_keys or []}
    if not allowed:return []
    result=[]
    for row in work.rows:
        raw=value(work,row,'tribunal')
        court=court_key(raw)
        if court in allowed:result.append(row.id)
    return result


def empty_preparation_message(work,selected,modality_keys,kind=''):
    from .modalities import selected_row
    ids=set(selected) if selected is not None else {row.id for row in work.rows}
    chosen=[row for row in work.rows if row.id in ids]
    active=[row for row in chosen if not row.excluded]
    included=[row for row in active if selected_row(work,row,modality_keys)]
    if not chosen:return 'No hay filas del tribunal o de la selección elegida. Revisa Filtros.'
    if not active:return 'Todas las filas elegidas están excluidas del lote. Revisa su inclusión en Trabajo.'
    if not included:
        return f'Las {len(active)} filas elegidas quedan fuera de las modalidades seleccionadas. Revisa Filtros o corrige el programa en Trabajo.'
    template=work.config['correos']['plantillas'].get(kind,{})
    name=template.get('nombre','el conjunto de correos')
    return f'No hay gestiones habilitadas para {name}. Revisa el tipo de correo, las reglas y las decisiones incluir/omitir en Trabajo.'
