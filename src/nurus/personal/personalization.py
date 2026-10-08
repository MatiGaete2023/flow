"""Small, reversible configuration edits with collision checks."""
from copy import deepcopy
from uuid import uuid4

from nurus.rus.columns import normalize
from .config import emails, validate


def save_contact(config, name, address, aliases=(), *, previous=None):
    result=deepcopy(config)
    name=name.strip()
    if not name:raise ValueError('Indica un nombre para el contacto.')
    address='; '.join(emails(address))
    if name.startswith('Tribunal: '):
        key=name.split(': ',1)[1]
        if key not in result['correos']['tribunales']:raise ValueError('Tribunal no reconocido.')
        result['correos']['tribunales'][key]['para']=emails(address)
        return result
    if previous and previous.startswith('Tribunal: '):
        raise ValueError('La clave del tribunal se conserva; selecciona Nuevo para crear un programa.')
    if any(normalize(alias)==normalize(name) and target!=previous
           for alias,target in result['aliases'].items()):
        raise ValueError('El nombre ya es alias de otro contacto: '+name)
    others={normalize(key) for key in result['contactos'] if key!=previous}
    if normalize(name) in others and name!=previous:
        raise ValueError('Ya existe otro contacto con ese nombre. No se reemplazó.')
    clean=list(dict.fromkeys(alias.strip() for alias in aliases if alias.strip()))
    owned={alias for alias,target in result['aliases'].items() if target==previous}
    for alias in clean:
        if normalize(alias) in others:
            raise ValueError('El alias coincide con otro contacto: '+alias)
        if any(normalize(key)==normalize(alias) and key not in owned and normalize(target)!=normalize(name)
               for key,target in result['aliases'].items()):
            raise ValueError('El alias ya pertenece a otro contacto: '+alias)
    if previous:result['contactos'].pop(previous,None)
    result['aliases']={key:target for key,target in result['aliases'].items() if key not in owned}
    result['contactos'][name]=address
    for alias in clean:result['aliases'][alias]=name
    validate(result)
    return result


def duplicate_template(config, key, name):
    from .mail_category import category, CATEGORIES
    result=deepcopy(config)
    target='particular_'+uuid4().hex[:10]
    template=deepcopy(result['correos']['plantillas'][key])
    operational=category(template,key)
    template.update(nombre=name.strip() or template['nombre']+' · copia',archivada=False,
                    categoria=operational if operational in CATEGORIES else '')
    result['correos']['plantillas'][target]=template
    validate(result)
    return result,target


def archive_template(config,key,archived=True):
    result=deepcopy(config)
    result['correos']['plantillas'][key]['archivada']=bool(archived)
    return result


def restore_template(config,key):
    from .config import defaults
    result=deepcopy(config)
    previous=result.get('template_backups',{}).get(key) or defaults()['correos']['plantillas'].get(key)
    if previous is None:raise ValueError('Esta plantilla todavía no tiene una versión anterior guardada.')
    result['correos']['plantillas'][key]=deepcopy(previous)
    result['correos']['plantillas'][key]['archivada']=False
    validate(result)
    return result
