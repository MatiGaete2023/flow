"""Progress and cooperative cancellation, isolated from Tk and Office calls."""
from contextvars import ContextVar
from contextlib import contextmanager
from dataclasses import dataclass, field
from threading import Event
from time import perf_counter

_CURRENT = ContextVar('csmp_operation', default=None)


class OperationCancelled(Exception):
    """The next safe unit was not started. Confirmed results remain available."""


@dataclass
class Operation:
    emit: object
    cancelled: Event = field(default_factory=Event)

    def update(self, label, completed=None, total=None):
        self.emit(('progress', str(label), completed, total))

    def checkpoint(self):
        if self.cancelled.is_set():
            raise OperationCancelled('Operación detenida entre productos. Los resultados confirmados se conservan.')


@contextmanager
def operation_scope(operation):
    token = _CURRENT.set(operation)
    try:
        yield operation
    finally:
        _CURRENT.reset(token)


def progress(label, completed=None, total=None):
    active = _CURRENT.get()
    if active:
        active.update(label, completed, total)


def checkpoint():
    active = _CURRENT.get()
    if active:
        active.checkpoint()


@contextmanager
def phase(label):
    progress(label)
    started = perf_counter()
    try:
        yield
    finally:
        active = _CURRENT.get()
        if active:
            active.emit(('timing', str(label), perf_counter() - started, None))
