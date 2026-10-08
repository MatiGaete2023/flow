"""Pausa en límites de página; cancelar libera siempre al trabajador."""
import threading


class PauseGate:
    def __init__(self):
        self._condition=threading.Condition();self._cancelled=False;self._paused=False
    @property
    def paused(self):
        with self._condition:return self._paused
    def pause(self):
        with self._condition:self._paused=True
    def resume(self):
        with self._condition:self._paused=False;self._condition.notify_all()
    def set(self):
        with self._condition:self._cancelled=True;self._paused=False;self._condition.notify_all()
    def clear(self):
        with self._condition:self._cancelled=False;self._paused=False;self._condition.notify_all()
    def is_set(self):
        with self._condition:
            while self._paused and not self._cancelled:self._condition.wait()
            return self._cancelled

    def wait(self,timeout=None):
        if self.is_set():return True
        with self._condition:
            self._condition.wait(timeout)
        return self.is_set()
