"""Lectura de planillas RUS y compatibilidad diferida con el evaluador histórico.

CSMP Assistant importa lector/modelos directamente. El evaluador NuRus anterior
solo se carga si un consumidor histórico llama explícitamente evaluate_batch().
"""

from .models import EvaluationBatch, EvaluationStatus, Mode, SourceReference
from .reader import WorkbookReadError, read_workbook


def evaluate_batch(*args, **kwargs):
    """Compatibilidad diferida; no forma parte del flujo operativo del Asistente."""
    from .service import evaluate_batch as _evaluate_batch
    return _evaluate_batch(*args, **kwargs)


__all__ = [
    "EvaluationBatch",
    "EvaluationStatus",
    "Mode",
    "SourceReference",
    "WorkbookReadError",
    "evaluate_batch",
    "read_workbook",
]
