"""Dependencies of prepared products, independent of their editable text."""
from hashlib import sha256
from pathlib import Path
import json

from nurus.rus.columns import normalize


def _fingerprint(work, record_ids, kind, recipient_type=''):
    ids = set(record_ids)
    rows = getattr(work, 'rows', [])
    mapping = getattr(work, 'mapping', {})
    cfg = getattr(work, 'config', {})
    templates=cfg.get('correos',{}).get('plantillas',{})
    is_mail=kind in templates or recipient_type in {'programas','tribunales'}
    from .mail_category import category
    from .courts import court_key
    operational=category(templates.get(kind,{}),kind)
    to_program=recipient_type=='programas' or (not recipient_type and operational.startswith('programa_'))

    def group(row):
        fields = ('tribunal','programa') if is_mail and to_program else ('tribunal',) if is_mail else ('tribunal','rit')
        return tuple((court_key if key=='tribunal' else normalize)(row.overrides.get(key, row.values.get(mapping.get(key, ''), ''))) for key in fields)

    context=_SCOPE.get()
    if context is not None and context['work'] is work:
        group_key=(is_mail,to_program)
        if group_key not in context['groups']:
            buckets={}
            for row in rows:buckets.setdefault(group(row),[]).append(row)
            context['groups'][group_key]=buckets
        by_id=context['by_id'];groups={group(by_id[rid]) for rid in ids if rid in by_id}
        candidates=[row for g in groups for row in context['groups'][group_key].get(g,[])]
    else:
        groups = {group(row) for row in rows if row.id in ids}
        candidates=rows
    records=[]
    for row in candidates:
        if row.id not in ids and group(row) not in groups:continue
        record={key:getattr(row,key) for key in
                ('id','review','observation','actions','excluded','decisions','overrides','word_overrides')}
        record['values']={key:row.values.get(column,'') for key,column in mapping.items()}
        if is_mail:
            from .outputs import due_value
            record['mail_metric']=due_value(work,row,operational)
        records.append(record)
    records.sort(key=lambda row: row['id'])
    configuration = {key: cfg.get(key) for key in ('correos', 'contactos', 'aliases', 'firma')}
    payload = {'records': records, 'config': configuration, 'kind': kind, 'ids': sorted(ids), 'recipient_type':recipient_type}
    return sha256(json.dumps(payload, ensure_ascii=False, sort_keys=True, default=str).encode()).hexdigest()


def stamp(work, product):
    product.dependency_hash = fingerprint(work, product.record_ids, product.kind,getattr(product,'recipient_type',''))
    if hasattr(product, 'template_hash'):
        product.template_hash = template_digest(product.template)


def stale(work, product):
    expected = getattr(product, 'dependency_hash', '')
    if expected and expected != fingerprint(work, product.record_ids, product.kind,getattr(product,'recipient_type','')):
        return True
    if getattr(product, 'template_hash', ''):
        path = Path(product.template)
        return not path.exists() or template_digest(path) != product.template_hash
    return False


def require_fresh(work, product):
    from .sync import require_current_copy
    require_current_copy(work)
    if stale(work, product):
        raise ValueError('Este producto necesita actualizarse porque cambiaron sus registros o plantilla. '
                         'Prepara nuevamente y revisa los cambios antes de guardar.')


from contextvars import ContextVar
from contextlib import contextmanager
_SCOPE=ContextVar('csmp_dependency_scope',default=None)


@contextmanager
def dependency_scope(work):
    """Only for a synchronous read-only batch; never persists across edits."""
    current=_SCOPE.get()
    if current is not None and current['work'] is work:
        yield
        return
    token=_SCOPE.set({'work':work,'by_id':{r.id:r for r in getattr(work,'rows',[])},'groups':{},'fingerprints':{},'templates':{}})
    try:yield
    finally:_SCOPE.reset(token)


def fingerprint(work,record_ids,kind,recipient_type=''):
    ids=tuple(sorted(set(record_ids)));context=_SCOPE.get();key=(ids,kind,recipient_type)
    if context is not None and context['work'] is work:
        cache=context['fingerprints']
        if key not in cache:cache[key]=_fingerprint(work,ids,kind,recipient_type)
        return cache[key]
    return _fingerprint(work,ids,kind,recipient_type)


def template_digest(path):
    path=Path(path);context=_SCOPE.get()
    if context is not None:
        cache=context['templates']
        if path not in cache:cache[path]=sha256(path.read_bytes()).hexdigest()
        return cache[path]
    return sha256(path.read_bytes()).hexdigest()
