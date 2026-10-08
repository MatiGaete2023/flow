"""Exportación de tablas del trabajo actual."""
from pathlib import Path
from nurus.services.file_output import write_new_file

def _book(sheets, destination):
    from openpyxl import Workbook
    from .excel_presentation import format_sheet
    from .attachment_store import workbook_bytes
    book=Workbook();book.remove(book.active)
    try:
        for name,headers,rows in sheets:
            sheet=book.create_sheet(name);sheet.append(headers)
            for row in rows:sheet.append(row)
            format_sheet(sheet,headers)
        content=workbook_bytes(book)
        write_new_file(destination,lambda p:Path(p).write_bytes(content))
    finally:book.close()
    return str(Path(destination))
