"""Fecha, parámetros y resultados por ejecución; aislamiento entre hilos."""
from contextvars import ContextVar
from datetime import datetime as RealDatetime

current = ContextVar('csmp_run', default=None)

class datetime(RealDatetime):
    @classmethod
    def now(cls, tz=None):
        ctx = current.get()
        if ctx is None:
            return RealDatetime.now(tz)
        value = ctx['date']
        return RealDatetime(value.year, value.month, value.day, 12, tzinfo=tz)

def parametro(name):
    return current.get()['config']['umbrales'][name]

def historia():
    ctx = current.get()
    return ctx.get('historia') if ctx else None

def duplicidad():
    ctx = current.get()
    return ctx.get('duplicidad') if ctx else None

def bitacora():
    ctx = current.get()
    return ctx.get('bitacora', ()) if ctx else ()

def marcar(nivel, codigo, texto):
    ctx = current.get()
    if ctx is not None:
        ctx.setdefault('marcas', []).append((nivel, codigo, texto))

def emitidos():
    ctx = current.get()
    return ctx.setdefault('emitidos', set()) if ctx is not None else set()

def evento(clave):
    ctx = current.get()
    if ctx is not None:
        ctx['events'].append(clave)

def acumulado():
    ctx = current.get()
    return ctx.get('acumulado') if ctx else None


def _al_corte(fuente, ctx):
    """Una lectura posterior a la fecha de corte no existía al corte: no decide nada (E3-06)."""
    if fuente is None:
        return None
    leido = getattr(fuente, 'leido', None)
    corte = ctx.get('date')
    corte = corte.date() if hasattr(corte, 'date') and callable(corte.date) else corte
    return None if leido is not None and corte is not None and leido > corte else fuente

def escritos():
    ctx = current.get()
    return _al_corte(ctx.get('escritos'), ctx) if ctx else None

def historico():
    ctx = current.get()
    return _al_corte(ctx.get('historico'), ctx) if ctx else None

def consignados():
    ctx = current.get()
    return ctx.get('consignados', frozenset()) if ctx else frozenset()

def evidencia_informe():
    ctx = current.get()
    return ctx.get('evidencia_informe') if ctx else None

def bitacora_parcial():
    ctx = current.get()
    return bool(ctx.get('bitacora_parcial')) if ctx else False
