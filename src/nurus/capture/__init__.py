"""Contrato local compartido por Descargador y Asistente."""

from .contract import Capture, CaptureError, TABLES, VERSION, read_capture, write_capture

__all__ = ['Capture', 'CaptureError', 'TABLES', 'VERSION', 'read_capture', 'write_capture']
