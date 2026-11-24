"""Conjunto exacto a revisar y resumen categórico. Sin autorización ni escritura real."""
from collections import Counter
from dataclasses import dataclass

from .batch_store import selection_digest
from .contracts import ContractError

from .contracts import RegistroFlowV1
from .journal import SimulationJournal, JournalError
from .readback_store import STATES


@dataclass(frozen=True)
class BatchSelection:
    operations: tuple[RegistroFlowV1, ...]

    def __post_init__(self):
        try:
            selection_digest(self.operations)
        except ContractError as exc:
            raise JournalError(str(exc)) from None

    @property
    def digest(self):
        return selection_digest(self.operations)

    def require_unchanged(self, current):
        if type(current) is not BatchSelection or current != self:
            raise JournalError('LOTE_CAMBIADO_REVISAR')

    def summary(self, journal):
        """Solo conteos cerrados, sin operación, ingreso, texto, autor ni hash."""
        if type(journal) is not SimulationJournal:
            raise JournalError('SOLO_SIMULACION')
        counts = Counter(); destinations = Counter(); attempts = 0
        with journal._tx():
            for op in self.operations:
                row = journal.db.execute('SELECT state FROM operations WHERE operation_id=?', (op.operation_id,)).fetchone()
                if row is None:
                    state = 'SIN_PREPARAR'
                else:
                    journal._require_current(op.operation_id, op)
                    state = row['state']
                    if state not in STATES:
                        raise JournalError('ESTADO_INVALIDO')
                    attempts += int(journal.db.execute('SELECT 1 FROM write_attempts WHERE operation_id=?', (op.operation_id,)).fetchone() is not None)
                counts[state] += 1
                destinations[op.destination] += 1
        return dict(simulation_only=True, total=len(self.operations), states=dict(counts), destinations=dict(destinations), attempts=attempts)


@dataclass(frozen=True)
class SimulationBatchReview:
    """Evidencia de confirmación ficticia, no identidad humana autenticada ni permiso de envío."""
    selection: BatchSelection
    confirmed_digest: str

    @classmethod
    def confirm(cls, selection, *, displayed_digest, confirmed):
        if type(selection) is not BatchSelection or confirmed is not True or displayed_digest != selection.digest:
            raise JournalError('CONFIRMACION_EXACTA_REQUERIDA')
        return cls(selection, displayed_digest)

    def check(self, current):
        self.selection.require_unchanged(current)
        if self.confirmed_digest != self.selection.digest:
            raise JournalError('CONFIRMACION_INVALIDA')
