"""Devuelve a una copia de Excel únicamente fechas de registros RUS comprobados."""
from datetime import datetime
from hashlib import sha256
from io import BytesIO
from pathlib import Path
import platform
import tempfile
from zipfile import ZipFile
from xml.etree import ElementTree

from nurus.rus.columns import normalize
from nurus.rus.rules import as_date, as_int
from nurus.services.file_output import write_new_file
from .config import defaults
from .registration import RemoteIdentity, restore_intent
from .work import Work


def _rows(work):
    result = {}
    for row in work.rows:
        try:
            identity = RemoteIdentity.from_row(work, row)
        except ValueError:
            continue
        if identity in result:
            raise ValueError('El Excel contiene el mismo ingreso remoto más de una vez; selecciona una tabla inequívoca.')
        result[identity] = row
    return result


def _columns(work):
    headers = list(work.rows[0].values) if work.rows else []
    aliases = {'FECHA_OBS': ('FECHA_OBS', 'FECHA OBS', 'FECHA OBSERVACION'), 'CC': ('CC',)}
    found = {}
    for key, names in aliases.items():
        matches = [index for index, title in enumerate(headers, 1) if normalize(title) in {normalize(n) for n in names}]
        if len(matches) != 1:
            raise ValueError('El Excel de registro necesita una columna única ' + key + '.')
        found[key] = matches[0]
    return found


def _format(content):
    if not content.startswith(b'PK\x03\x04'):
        return '.xls'
    with ZipFile(BytesIO(content)) as package:
        names = {name.lower() for name in package.namelist()}
        if 'xl/connections.xml' in names or any('macrosheets/' in name for name in names):
            raise ValueError('Este libro requiere revisar sus conexiones o macros XLM antes de abrirlo en Excel.')
        types = ElementTree.fromstring(package.read('[Content_Types].xml'))
        macro = any(item.attrib.get('PartName') == '/xl/workbook.xml' and
                    item.attrib.get('ContentType') == 'application/vnd.ms-excel.sheet.macroEnabled.main+xml'
                    for item in types)
        return '.xlsm' if macro or 'xl/vbaproject.bin' in names else '.xlsx'


def _native(content, target, sheet_name, updates):
    if platform.system() != 'Windows':
        raise ValueError('La devolución fiel requiere Windows y Excel de escritorio.')
    import pythoncom
    import win32com.client
    with tempfile.TemporaryDirectory(prefix='csmp-retorno-', dir=target.parent) as folder:
        source = Path(folder) / ('origen' + target.suffix)
        source.write_bytes(content)
        target.unlink(missing_ok=True)  # Solo el archivo temporal creado por write_new_file.
        app = book = placeholder = None
        pythoncom.CoInitialize()
        try:
            app = win32com.client.DispatchEx('Excel.Application')
            app.Visible = False
            app.DisplayAlerts = False
            app.EnableEvents = False
            app.AskToUpdateLinks = False
            app.AutomationSecurity = 3
            placeholder = app.Workbooks.Add()
            book = app.Workbooks.Open(str(source), 0, False)
            placeholder.Close(False)
            placeholder = None
            sheet = book.Worksheets(sheet_name)
            for row, date_column, day, cc_column, charge in updates:
                if any(sheet.Cells(row, column).HasFormula or sheet.Cells(row, column).MergeCells
                       for column in (date_column, cc_column)):
                    raise ValueError('FECHA_OBS o CC contiene una fórmula o celda combinada; revisa su devolución.')
                sheet.Cells(row, date_column).Value = datetime.combine(day, datetime.min.time())
                if sheet.Cells(row, date_column).NumberFormat in ('General', '@'):
                    sheet.Cells(row, date_column).NumberFormat = 'dd/mm/yyyy'
                sheet.Cells(row, cc_column).Value2 = charge
            book.SaveCopyAs(str(target))
        finally:
            try:
                if placeholder is not None:
                    placeholder.Close(False)
                if book is not None:
                    book.Close(False)
            finally:
                try:
                    if app is not None:
                        app.Quit()
                finally:
                    pythoncom.CoUninitialize()


def _portable(content, target, sheet_name, updates):
    from openpyxl import load_workbook
    book = load_workbook(BytesIO(content), keep_vba=target.suffix == '.xlsm')
    try:
        sheet = book[sheet_name]
        for row, date_column, day, cc_column, charge in updates:
            from openpyxl.cell.cell import MergedCell
            if any(sheet.cell(row, column).data_type == 'f' or isinstance(sheet.cell(row, column), MergedCell)
                   or any(sheet.cell(row, column).coordinate in merged for merged in sheet.merged_cells.ranges)
                   for column in (date_column, cc_column)):
                raise ValueError('FECHA_OBS o CC contiene una fórmula o celda combinada; revisa su devolución.')
            cell = sheet.cell(row, date_column)
            cell.value = day
            if cell.number_format in ('General', '@'):
                cell.number_format = 'dd/mm/yyyy'
            sheet.cell(row, cc_column).value = charge
        book.save(target)
    finally:
        if book.vba_archive is not None:
            book.vba_archive.close()
        book.close()


def reconcile_excel(journal, source, destination, *, mode='ESPERA', sheet=None,
                    operation_ids=None, backend='native', reduced_fidelity=False):
    source = Path(source).resolve()
    target = Path(destination).resolve()
    if source == target:
        raise ValueError('El Excel de origen se conserva: elige un nombre nuevo.')
    if target.exists():
        raise ValueError('La copia ya existe: elige un nombre nuevo.')
    if source.stat().st_size > 80 * 1024 * 1024:
        raise ValueError('El Excel excede el límite de lectura de 80 MB.')
    work = Work.external(source, defaults(), mode=mode, sheet=sheet)
    if target.suffix.lower() != _format(work.content):
        raise ValueError('La copia debe conservar el formato real del libro de origen.')
    rows = _rows(work)
    columns = _columns(work)
    wanted = set(operation_ids) if operation_ids is not None else None
    selected = []
    for data in journal.list():
        if wanted is not None and data['operation_id'] not in wanted:
            continue
        if data.get('verified') is not True or data['state'] not in ('PENDIENTE_EXCEL', 'COMPROBADA'):
            if wanted is not None:
                raise ValueError('Una operación seleccionada carece de registro comprobado en RUS.')
            continue
        intent = restore_intent(data)
        if intent.identity not in rows:
            if wanted is not None:
                raise ValueError('Una operación seleccionada no tiene un ingreso único en este Excel.')
            continue
        selected.append((data, intent, rows[intent.identity]))
    if not selected or (wanted is not None and wanted != {data['operation_id'] for data, _, _ in selected}):
        raise ValueError('No hay recibos comprobados para todos los ingresos seleccionados.')
    if len({row.id for _, _, row in selected}) != len(selected):
        raise ValueError('Hay varias gestiones para una misma fila: selecciona el recibo que corresponde al Excel.')
    updates = []
    for data, intent, row in selected:
        if row.review.get('OBSERVACION') != intent.text:
            raise ValueError('La observación del Excel cambió desde el registro; no se devuelve una fecha al texto nuevo.')
        day = datetime.fromisoformat(data['registered_at']).date()
        old_date = row.review.get('FECHA_OBS')
        if old_date not in ('', None) and as_date(old_date) != day:
            raise ValueError('FECHA_OBS contiene una fecha distinta; no se sustituye una edición personal.')
        old_cc = row.review.get('CC')
        if old_cc not in ('', None) and as_int(old_cc) != data['cc']:
            raise ValueError('CC contiene un valor distinto al tipo comprobado; revisa el conflicto.')
        updates.append((row.source_row, columns['FECHA_OBS'], day, columns['CC'], data['cc']))
    if backend == 'portable':
        if not reduced_fidelity or not work.content.startswith(b'PK\x03\x04'):
            raise ValueError('La salida portable requiere XLSX/XLSM y aceptación de fidelidad reducida.')
        writer = _portable
    elif backend == 'native':
        writer = _native
    else:
        raise ValueError('Modo de devolución de Excel no reconocido.')
    # La copia se genera desde bytes leídos juntos; un cambio externo no desplaza filas.
    if sha256(source.read_bytes()).hexdigest() != work.source_hash:
        raise ValueError('El Excel cambió durante la lectura; vuelve a cargarlo.')
    write_new_file(target, lambda temporary: writer(work.content, temporary, work.sheet, updates))
    returned = Work.external(target, work.config, mode=work.mode, sheet=work.sheet)
    returned_rows = _rows(returned)
    for data, intent, _ in selected:
        row = returned_rows.get(intent.identity)
        if row is None or row.review.get('OBSERVACION') != intent.text:
            raise ValueError('La copia no conserva el ingreso y texto del recibo; el retorno sigue pendiente.')
        if as_date(row.review.get('FECHA_OBS')) != datetime.fromisoformat(data['registered_at']).date():
            raise ValueError('La copia no conserva la fecha efectiva; el retorno sigue pendiente.')
        if as_int(row.review.get('CC')) != data['cc']:
            raise ValueError('La copia no conserva la carga comprobada; el retorno sigue pendiente.')
    journal.complete_excel_many([data['operation_id'] for data, _, _ in selected], target, returned.source_hash)
    return str(target)


def main():
    import argparse
    from .registration import Journal
    parser = argparse.ArgumentParser(description='Conciliar recibos RUS y Excel sin enviar observaciones.')
    parser.add_argument('--diario', required=True)
    parser.add_argument('--excel', required=True)
    parser.add_argument('--salida', required=True)
    parser.add_argument('--modo', choices=['ESPERA', 'CUMPLIMIENTO', 'INFORMES'], default='ESPERA')
    parser.add_argument('--hoja')
    parser.add_argument('--portable', action='store_true', help='Aceptar menor fidelidad para XLSX/XLSM.')
    args = parser.parse_args()
    reconcile_excel(Journal(args.diario), args.excel, args.salida, mode=args.modo, sheet=args.hoja,
                    backend='portable' if args.portable else 'native', reduced_fidelity=args.portable)


if __name__ == '__main__':
    main()
