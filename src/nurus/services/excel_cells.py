"""Frontera común de texto Excel: validar antes de que openpyxl lo recorte."""

from collections.abc import Iterable
from openpyxl.cell.cell import Cell
from openpyxl.worksheet.worksheet import Worksheet

MAX_TEXT_UNITS = 32767


def validate_excel_text(value: object) -> None:
    if isinstance(value, str) and len(value.encode('utf-16-le')) // 2 > MAX_TEXT_UNITS:
        raise ValueError(
            'Campo excede el límite Excel de 32.767 unidades UTF-16; se conserva la fuente y el trabajo anterior.'
        )


def append_literal_row(sheet: Worksheet, values: Iterable[object]) -> None:
    """Agrega una fila al final con texto literal (nunca fórmula), validando antes de mutar la hoja."""
    values = list(values)
    for value in values:
        validate_excel_text(value)
    cells = []
    for column, value in enumerate(values, 1):
        # Worksheet.append fija la fila definitiva de cada celda.
        cell = Cell(sheet, row=1, column=column, value=value)
        if isinstance(value, str):
            cell.data_type = 's'
        cells.append(cell)
    sheet.append(cells)
