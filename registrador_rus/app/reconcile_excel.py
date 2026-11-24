"""P6c: devolución de recibos ficticios a una copia XLSX simple, sin RUS."""
from dataclasses import dataclass
from datetime import date, datetime
from hashlib import sha256
from io import BytesIO
import os
from pathlib import Path
import tempfile
from zipfile import ZipFile

from .contracts import RegistroFlowV1
from .journal import SimulationJournal

HEADERS = ('SITFA_TRIBUNAL_CODIGO', 'SITFA_CAUSA_RUS', 'SITFA_INGRESO_RUS', 'OBSERVACION', 'FECHA_OBS', 'CC')
UNSUPPORTED_PARTS = ('xl/drawings/', 'xl/charts/', 'xl/media/', 'xl/pivot', 'xl/externalLinks/', 'xl/vba',
                     'xl/queryTables/', 'xl/connections', 'xl/slicers/', 'xl/activeX/', 'xl/embeddings/',
                     'xl/ctrlProps/', 'customXml/', '_xmlsignatures/', 'xl/comments', 'xl/threadedComments/')


class ExcelConflict(RuntimeError):
    pass


@dataclass(frozen=True)
class ExcelRequest:
    current: RegistroFlowV1
    sheet_name: str
    expected_date: date | None
    expected_cc: int | None

    def __post_init__(self):
        if (not isinstance(self.current, RegistroFlowV1) or not isinstance(self.sheet_name, str) or not self.sheet_name or
                (self.expected_date is not None and type(self.expected_date) is not date) or
                (self.expected_cc is not None and (type(self.expected_cc) is not int or self.expected_cc not in (0, 1)))):
            raise ExcelConflict('SOLICITUD_EXCEL_INVALIDA')


def _hash(data):
    return sha256(data).hexdigest()


def _load(data):
    from openpyxl import load_workbook
    with ZipFile(BytesIO(data)) as archive:
        if any(name.startswith(UNSUPPORTED_PARTS) for name in archive.namelist()):
            raise ExcelConflict('LIBRO_COMPLEJO_REQUIERE_EXCEL_NATIVO')
    return load_workbook(BytesIO(data), data_only=False, keep_links=True)


def _id(cell):
    if cell.data_type == 'f':
        return None
    value = cell.value
    if isinstance(value, str) and value and value == value.strip():
        return value
    if type(value) is int and 0 <= value <= 999_999_999_999_999:
        return str(value)
    return None


def _date(value):
    if value is None or value == '':
        return None
    if isinstance(value, datetime) and value.time() == datetime.min.time():
        return value.date()
    if type(value) is date:
        return value
    if isinstance(value, str):
        for pattern in ('%d/%m/%Y', '%Y-%m-%d'):
            try:
                return datetime.strptime(value, pattern).date()
            except ValueError:
                pass
    raise ExcelConflict('FECHA_EXCEL_NO_RECONOCIDA')


def _cc(value):
    if value is None or value == '':
        return None
    if type(value) is int and value in (0, 1):
        return value
    if type(value) is str and value in ('0', '1'):
        return int(value)
    raise ExcelConflict('CC_NO_RECONOCIDO')


def _cells(book, request):
    if request.sheet_name not in book.sheetnames:
        raise ExcelConflict('HOJA_NO_ENCONTRADA')
    sheet = book[request.sheet_name]
    if sheet.protection.sheet:
        raise ExcelConflict('HOJA_PROTEGIDA')
    columns = {}
    for name in HEADERS:
        found = [c.column for c in sheet[1] if c.value == name and c.data_type != 'f']
        if len(found) != 1:
            raise ExcelConflict('COLUMNA_NO_UNICA_O_AUSENTE')
        columns[name] = found[0]
    ident = request.current.remote_identity
    expected = (ident.tribunal_id, ident.causa_id, ident.ingreso_id)
    matches = []
    for number in range(2, sheet.max_row + 1):
        values = tuple(_id(sheet.cell(number, columns[name])) for name in HEADERS[:3])
        if values == expected:
            matches.append(number)
        elif sum(a == b for a, b in zip(values, expected)) >= 2 and None in values and all(a is None or a == b for a, b in zip(values, expected)):
            raise ExcelConflict('FILA_POSIBLE_SIN_IDENTIDAD_COMPLETA')
    if len(matches) != 1:
        raise ExcelConflict('INGRESO_EXCEL_NO_UNICO')
    cells = [sheet.cell(matches[0], columns[name]) for name in HEADERS]
    if any(c.data_type == 'f' or c.coordinate in sheet.merged_cells for c in cells):
        raise ExcelConflict('CELDA_FORMULA_O_COMBINADA')
    if cells[3].value != request.current.approved_text:
        raise ExcelConflict('TEXTO_EXCEL_CAMBIADO')
    return cells[-2], cells[-1]


def _fingerprint(book, ignored):
    """Compara celdas ajenas, fórmulas, estilos y combinaciones sin normalizar texto."""
    result = []
    for sheet in book.worksheets:
        cells = {}
        for cell in sheet._cells.values():
            if (sheet.title, cell.coordinate) in ignored or (cell.value is None and not cell.has_style and not cell.hyperlink):
                continue
            if cell.data_type == 'f' and not isinstance(cell.value, str):
                raise ExcelConflict('FORMULA_COMPLEJA_REQUIERE_EXCEL_NATIVO')
            cells[cell.coordinate] = (cell.value, cell.data_type, tuple(cell._style or ()),
                                      cell.hyperlink.target if cell.hyperlink else None)
        result.append((sheet.title, sheet.sheet_state, tuple(sorted(str(r) for r in sheet.merged_cells.ranges)), cells))
    return result


def reconcile_simulated(journal, requests, source, destination):
    """Solo recibos simulados. Todas las devoluciones se verifican antes de confirmar."""
    if not isinstance(journal, SimulationJournal):
        raise ExcelConflict('DIARIO_SIMULADO_REQUERIDO')
    requests = tuple(requests)
    if not requests or any(not isinstance(r, ExcelRequest) for r in requests):
        raise ExcelConflict('SOLICITUD_EXCEL_INVALIDA')
    if len({r.current.operation_id for r in requests}) != len(requests):
        raise ExcelConflict('OPERACION_REPETIDA')
    source, destination = Path(source), Path(destination)
    if (source.suffix.lower() != '.xlsx' or destination.suffix.lower() != '.xlsx' or source.is_symlink() or
            source.resolve() == destination.resolve() or not source.is_file() or destination.exists() or destination.is_symlink()):
        raise ExcelConflict('RUTA_XLSX_INVALIDA_O_EXISTENTE')
    original = source.read_bytes()
    original_hash = _hash(original)
    book = _load(original)
    with journal._tx():
        journal.db.execute('''CREATE TABLE IF NOT EXISTS excel_reconciliations(
            operation_id TEXT PRIMARY KEY REFERENCES readback_receipts(operation_id),
            source_sha256 TEXT NOT NULL, output_sha256 TEXT NOT NULL, simulated INTEGER NOT NULL CHECK(simulated=1))''')
        targets = {}
        for req in requests:
            op = req.current
            row = journal._require_current(op.operation_id, op)
            receipt = journal.db.execute('SELECT * FROM readback_receipts WHERE operation_id=? AND simulated=1',
                                         (op.operation_id,)).fetchone()
            if (row['state'] != 'PENDIENTE_EXCEL' or receipt is None or receipt['approval_digest'] != row['digest'] or
                    receipt['destination'] != op.destination or receipt['identity_key'] != row['identity_key']):
                raise ExcelConflict('RECIBO_NO_ELEGIBLE')
            date_cell, cc_cell = _cells(book, req)
            key = (req.sheet_name, date_cell.coordinate, cc_cell.coordinate)
            if key in targets:
                raise ExcelConflict('FILA_REPETIDA_EN_SOLICITUD')
            desired = datetime.fromisoformat(receipt['server_timestamp']).date()
            actual_date, actual_cc = _date(date_cell.value), _cc(cc_cell.value)
            if (actual_date != req.expected_date or actual_cc != req.expected_cc or actual_date not in (None, desired) or
                    (actual_date is not None and actual_cc not in (None, op.cc))):
                raise ExcelConflict('FECHA_O_CC_EN_CONFLICTO')
            targets[key] = (req, desired, op.cc)
        ignored = {(sheet, coord) for sheet, d, c in targets for coord in (d, c)}
        original_fingerprint = _fingerprint(book, ignored)
        for (sheet, d, c), (_, desired, cc) in targets.items():
            book[sheet][d].value = desired
            book[sheet][c].value = cc
        buffer = BytesIO()
        book.save(buffer)
        output = buffer.getvalue()
        output_hash = _hash(output)
        check = _load(output)
        if _fingerprint(check, ignored) != original_fingerprint:
            raise ExcelConflict('CELDAS_AJENAS_MODIFICADAS')
        for (sheet, d, c), (req, desired, cc) in targets.items():
            date_cell, cc_cell = _cells(check, req)
            if (date_cell.coordinate, cc_cell.coordinate, _date(date_cell.value), _cc(cc_cell.value)) != (d, c, desired, cc):
                raise ExcelConflict('COPIA_NO_VERIFICADA')
        if _hash(source.read_bytes()) != original_hash:
            raise ExcelConflict('ORIGINAL_CAMBIO_DURANTE_CONCILIACION')
        # Publicación sin reemplazo. Si el sistema de archivos no admite hardlink, se detiene.
        tmp = None
        try:
            with tempfile.NamedTemporaryFile(dir=destination.parent, suffix='.tmp', delete=False) as f:
                tmp = Path(f.name)
                f.write(output); f.flush(); os.fsync(f.fileno())
            os.link(tmp, destination)
        finally:
            if tmp is not None:
                tmp.unlink(missing_ok=True)
        if _hash(destination.read_bytes()) != output_hash or _hash(source.read_bytes()) != original_hash:
            raise ExcelConflict('ARCHIVO_CAMBIO_TRAS_PUBLICACION')
        for req in requests:
            op_id = req.current.operation_id
            journal.db.execute('INSERT INTO excel_reconciliations VALUES(?,?,?,1)', (op_id, original_hash, output_hash))
            journal.db.execute("UPDATE operations SET state='COMPROBADA' WHERE operation_id=?", (op_id,))
            journal._audit(op_id, 'EXCEL_SIMULADO_VERIFICADO')
    return output_hash
