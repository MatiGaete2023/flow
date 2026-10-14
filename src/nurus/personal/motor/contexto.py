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
