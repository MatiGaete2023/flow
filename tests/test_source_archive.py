from hashlib import sha256

from openpyxl import Workbook

from nurus.rus import Mode, read_workbook


def source_file(tmp_path):
    path = tmp_path / "original.xlsx"
    book = Workbook()
    sheet = book.active
    sheet.title = "Espera"
    sheet.append(["RIT", "TRIBUNAL", "DERIVACION", "T ESPERA"])
    sheet.append(["X-1", "Juzgado de Laja", "PRM Prueba", 45])
    other = book.create_sheet("OB")
    other["A1"] = "=1+2"
    other.column_dimensions["A"].width = 27
    book.save(path)
    return path



def test_snapshot_bytes_hash_and_repr(tmp_path):
    path = source_file(tmp_path)
    batch = read_workbook(path, Mode.ESPERA)
    assert batch.source_bytes == path.read_bytes()
    assert sha256(batch.source_bytes).hexdigest() == batch.workbook_sha256
    assert "source_bytes=" not in repr(batch)
