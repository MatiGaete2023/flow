"""Selección de modalidades para texto y registros del borrador."""
import re
from nurus.rus.columns import normalize
MODALITIES={
    'RES':'Cuidado alternativo residencial',
    'AMB':'Intervención ambulatoria de reparación',
    'FAE':'Cuidado alternativo familia de acogida',
    'DCE':'Diagnóstico Clínico Especializado (DCE)',
}
def modality(program):
    name=normalize(program).upper()
    first=re.split(r'[\s-]+',name)[0]
    if first=='DCE':return 'DCE'
    if first in {'FAE','FAS'}:return 'FAE'
    if first.startswith(('REM','RLP','RMA','RFA','RDS','RSP','RPE')) or first in {'RES','RESIDENCIA','RESIDENCIAL'}:return 'RES'
    if first in {'AFT','PF','PRM','PPF','PIE','PDE','PEE','PAS','DAM','OPD','PDC'}:return 'AMB'
    return ''
def selected_row(work,row,keys):
    if keys is None or set(keys)==set(MODALITIES):return True
    explicit=next((str(v) for k,v in row.values.items() if normalize(k) in ('modalidad','tipo programa')), '')
    key=modality(explicit) or modality(row.values.get(work.mapping.get('programa',''),''))
    return key in keys
