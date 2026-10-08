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
