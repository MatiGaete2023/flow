"""One grammar for editable templates, validation and preview."""
from string import Formatter


def variables(text, allowed):
    result = set()
    try:
        parts = list(Formatter().parse(text))
    except ValueError as exc:
        raise ValueError('Plantilla con llaves incompletas: ' + str(exc)) from exc
    for _, name, spec, conversion in parts:
        if name is None:
            continue
        if not name or name not in allowed or spec or conversion:
            raise ValueError('Variable de plantilla inválida: {' + str(name) + '}. '
                             'Usa solo los nombres ofrecidos, sin formatos ni conversiones.')
        result.add(name)
    return result


def render(text, values):
    variables(text, set(values))
    return text.format_map(values)
