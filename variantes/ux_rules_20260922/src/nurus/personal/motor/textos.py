from .contexto import current

def render(modo, id_regla, **datos):
    ctx = current.get()
    key = modo + '.' + id_regla
    if key in ctx['config']['desactivadas'] or key in ctx.get('invalid',()):
        return ''
    ctx['events'].append(key)
    return ctx['config']['textos'][modo][id_regla]['texto'].format_map(datos)
