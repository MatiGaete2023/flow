"""Contrato de carga capturado: lectura completa, sin inferir cobertura de firmas."""

from collections import Counter
from datetime import date, datetime
import re

HEADERS = (
    'RIT',
    'TRIBUNAL',
    'NOMBRE FUNCIONARIO',
    'NOMBRE JUEZ',
    'TRÁMITE',
    'INVALIDADO',
    'DOC. RES',
    'DOC. ESC',
    'DÍAS',
    'CARGO',
    'PRIMER PREGRABADO FECHA',
    'PRIMER PREGRABADO HORA',
    'ÚLTIMO PREGRABADO FECHA',
    'ÚLTIMO PREGRABADO HORA',
    'DESPACHO FECHA',
    'DESPACHO HORA',
    'DEVOLUCIÓN FECHA',
    'DEVOLUCIÓN HORA',
    'NUEVO DESPACHO FECHA',
    'NUEVO DESPACHO HORA',
    'CANTIDAD ESCRITOS RESOLUCIÓN',
    'FECHA FIRMA',
    'HORA FIRMA',
)


def parse_day(value):
    if isinstance(value, datetime):
        return value.date()
    if isinstance(value, date):
        return value
    try:
        return datetime.strptime(str(value).strip(), '%d/%m/%Y').date()
    except ValueError:
        return None


def canonical(value):
    import motor as m

    if isinstance(value, datetime):
        value = value.strftime('%d/%m/%Y')
    if isinstance(value, float) and value.is_integer():
        value = int(value)
    return m.normalized(value)


def record_rows(grid):
    """Valida encabezado, orden y cantidad antes de leer filas posicionales."""
    _validate_excel_schema(grid)
    return _record_rows_unchecked(grid)


def _record_rows_unchecked(grid):
    """Lee filas posicionales de una tabla web o un reporte ya validado."""
    rows = []
    for row in grid:
        if row and re.fullmatch(r'[A-Z]+-\d+-\d{4}', canonical(row[0])):
            if len(row) != 23:
                import motor as m

                raise m.PocError('Cambió el esquema de carga: se esperaban 23 columnas.')
            rows.append(tuple(canonical(v) for v in row))
    return rows


def _validate_excel_schema(grid):
    """Exige el encabezado completo y ordenado del XLS antes de leer sus filas."""
    expected = tuple(canonical(value) for value in HEADERS)
    known = set(expected)
    headers = []
    for index, row in enumerate(grid):
        normalized = tuple(canonical(value) for value in row)
        # El umbral reconoce un encabezado aunque una columna haya sido renombrada,
        # sin confundirlo con filas de título como el texto fijo «Concepción».
        looks_like_header = (normalized and normalized[0] == expected[0]) or (
            len(normalized) == len(expected) and len(known.intersection(normalized)) >= 12
        )
        if looks_like_header:
            headers.append((index, row, normalized))
    if len(headers) != 1:
        import motor as m

        if not headers:
            received = next(
                (tuple(row) for row in grid if row and re.fullmatch(r'[A-Z]+-\d+-\d{4}', canonical(row[0]))), ()
            )
            raise m.PocError(
                'No se encontró el encabezado de carga esperado: '
                + repr(HEADERS)
                + '; recibido: '
                + repr(received)
                + '.'
            )
        raise m.PocError('El informe contiene más de un encabezado de carga.')
    header_index, row, normalized = headers[0]
    if normalized != expected:
        import motor as m

        raise m.PocError(
            'Encabezado de carga inesperado; esperado: ' + repr(HEADERS) + '; recibido: ' + repr(tuple(row)) + '.'
        )
    record_indices = [
        index for index, row in enumerate(grid) if row and re.fullmatch(r'[A-Z]+-\d+-\d{4}', canonical(row[0]))
    ]
    if any(index < header_index for index in record_indices):
        import motor as m

        raise m.PocError('Hay registros antes del encabezado de carga esperado.')


def records_in_grid(grid):
    _validate_excel_schema(grid)
    return Counter(_record_rows_unchecked(grid))


def result_records(form, profile):
    import motor as m

    candidates = [_record_rows_unchecked(m.table_grid(t)) for t in form.xpath('.//table')]
    candidates = [rows for rows in candidates if rows]
    if len(candidates) > 1:
        raise m.PocError('Más de un listado de carga; revisar la estructura.')
    return Counter(candidates[0]) if candidates else Counter()
