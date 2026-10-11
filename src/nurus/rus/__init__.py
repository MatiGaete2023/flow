"""Lectura de planillas RUS para el Asistente (lector, columnas y modelos).

El evaluador histórico NuRus (rus/service.py y rus/rules.py) se retiró en WP-21;
permanece recuperable en el historial Git (etiqueta pre-WP-21).
"""

from .models import EvaluationBatch, EvaluationStatus, Mode, SourceReference
from .reader import WorkbookReadError, read_workbook


__all__ = [
    "EvaluationBatch",
    "EvaluationStatus",
    "Mode",
    "SourceReference",
    "WorkbookReadError",
    "read_workbook",
]
