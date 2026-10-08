"""Identidad común de tribunales para filtros y destinatarios de correo."""
from nurus.rus.columns import normalize
from nurus.rus.rules import tribunal


def court_key(value):
    return tribunal(value) or normalize(value).upper()


def court_contact(config, value):
    """Resuelve claves históricas sin modificar los destinatarios personalizados.

    Las claves estables tienen prioridad. Si solo hay alias contradictorios,
    se requiere corregir la configuración antes de elegir destinatarios.
    """
    courts = config['correos']['tribunales']
    key = court_key(value)
    if key in courts:
        return courts[key]
    matches = [contact for name, contact in courts.items()
               if court_key(name) == key or court_key(contact.get('nombre', '')) == key]
    if matches:
        if any(contact != matches[0] for contact in matches[1:]):
            raise ValueError('Hay destinatarios contradictorios para ' + str(value)
                             + '. Revisa los tribunales en Configuración.')
        return matches[0]
    return {'nombre': str(value), 'para': []}
