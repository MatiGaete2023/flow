"""Adaptador del Descargador a la presentación Excel compartida; sin cambios de formato."""

from csmp_shared import parser

parser()  # Resuelve el paquete común también en la distribución fuente independiente.

from nurus.personal.excel_presentation import (  # noqa: E402
    BODY_FONT,
    BORDER,
    EDGE,
    HEADER_FILL,
    HEADER_FONT,
    WRAP,
    column_width,
    configure_sheet,
    format_cell,
    format_sheet,
)

__all__ = [
    'BODY_FONT',
    'BORDER',
    'EDGE',
    'HEADER_FILL',
    'HEADER_FONT',
    'WRAP',
    'column_width',
    'configure_sheet',
    'format_cell',
    'format_sheet',
]
