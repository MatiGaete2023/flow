from __future__ import annotations

import hashlib
from openpyxl import Workbook

from nurus.rus import Mode, read_workbook


def write_sheet(workbook: Workbook, name: str, headers: list[str], rows: list[list[object]]) -> None:
    sheet = workbook.create_sheet(name)
    sheet.append(headers)
    for row in rows:
        sheet.append(row)


def test_waiting_reader_preserves_source_and_applies_rule(tmp_path):
    path = tmp_path / "espera.xlsx"
    book = Workbook(); book.remove(book.active)
    write_sheet(book, "Espera", ["DERIVACION", "TRIBUNAL", "NOMBRE", "RIT", "T ESPERA"], [["FAE Norte", "Juzgado de Laja", "Ana Pérez", "C-1", 45]])
    book.save(path)

    batch = read_workbook(path, Mode.ESPERA)

    # WP-21: el evaluador NuRus se retiró; la regla E-05 vigente se prueba en tests/test_reglas_*.
    reference = batch.records[0].source
    assert reference.workbook_name == "espera.xlsx"
    assert reference.workbook_sha256 == hashlib.sha256(path.read_bytes()).hexdigest()
    assert (reference.sheet_name, reference.row_number) == ("Espera", 2)


def test_compliance_cross_keeps_secondary_row_provenance(tmp_path):
    path = tmp_path / "cumplimiento.xlsx"
    book = Workbook(); book.remove(book.active)
    headers = ["DERIVACION", "TRIBUNAL", "NOMBRE", "RIT", "RUT", "DIAS DE CUMPLIMIENTO", "DIAS PARA EGRESAR"]
    write_sheet(book, "Cumplimiento", headers, [["PRM Centro", "Juzgado de Tomé", "Juan Pérez", "C-2", "12.345.678-9", 100, 100]])
    write_sheet(book, "Hoja2", ["NOMBRE CENTRO", "TRIBUNAL", "NOMBRE MENOR", "RIT", "RUT MENOR", "FECHA VENCIMIENTO"], [["PRM Centro", "Juzgado de Tomé", "Juan Pérez", "C-2", "12.345.678-9", "20/09/2026"]])
    book.save(path)

    batch = read_workbook(path, Mode.CUMPLIMIENTO)

    assert len(batch.cross_records) == 1
    cross = batch.cross_records[0].source
    assert (cross.sheet_name, cross.row_number) == ("Hoja2", 2)
    assert cross.workbook_sha256 == batch.records[0].source.workbook_sha256


def test_compliance_does_not_use_an_unrelated_second_sheet(tmp_path):
    path = tmp_path / "sin_cruce.xlsx"
    book = Workbook(); book.remove(book.active)
    write_sheet(book, "Cumplimiento", ["DERIVACION", "TRIBUNAL", "NOMBRE", "RIT", "RUT"], [["PRM Centro", "Juzgado de Tomé", "Juan", "C-3", "1-9"]])
    write_sheet(book, "VALIDACION", ["FILA_EXCEL", "MOTIVO"], [[2, "dato"]])
    book.save(path)

    batch = read_workbook(path, Mode.CUMPLIMIENTO)

    assert not batch.cross_records
    assert any(warning.startswith("CROSS_SHEET_NOT_SELECTED") for warning in batch.warnings)


def test_named_cross_sheet_with_incomplete_mapping_requires_missing_cross_exception(tmp_path):
    path = tmp_path / "cruce_incompleto.xlsx"
    book = Workbook(); book.remove(book.active)
    write_sheet(book, "Cumplimiento", [
        "DERIVACION", "TRIBUNAL", "NOMBRE", "RIT", "RUT",
        "DIAS DE CUMPLIMIENTO", "DIAS PARA EGRESAR",
    ], [["PRM Centro", "Juzgado de Tomé", "Juan Pérez", "C-2", "12.345.678-9", 100, 100]])
    write_sheet(book, "Hoja2", ["NOTA"], [["sin columnas de cruce"]])
    book.save(path)

    result = read_workbook(path, Mode.CUMPLIMIENTO)

    assert result.cross_sheet == ""
    assert not result.cross_records
    assert any(warning.startswith("CROSS_MAPPING_MISSING") for warning in result.warnings)
    assert any(warning.startswith("CROSS_SHEET_NOT_SELECTED") for warning in result.warnings)


