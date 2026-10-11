from __future__ import annotations

from datetime import date
from io import BytesIO
import platform

import pytest
from openpyxl import Workbook, load_workbook
from openpyxl.styles import Font, PatternFill
from openpyxl.worksheet.datavalidation import DataValidation

from nurus.personal.config import defaults
from nurus.personal.record_edits import apply
from nurus.personal.work import Work
from nurus.services.exports import ExportError


def _source(path):
    book = Workbook()
    sheet = book.active
    sheet.title = "Espera"
    sheet.append(["DERIVACION", "TRIBUNAL", "NOMBRE", "RIT", "T ESPERA"])
    sheet.append(["PRM Norte", "Juzgado de Laja", "Ana", "C-1", 45])
    sheet.append(["PRM Norte", "Juzgado de Laja", "Beto", "C-2", 45])
    sheet.freeze_panes = "A2"
    sheet.auto_filter.ref = "A1:E3"
    sheet.column_dimensions["B"].width = 31
    sheet["A1"].font = Font(bold=True)
    sheet["C2"].fill = PatternFill(fill_type="solid", fgColor="92D050")
    validation = DataValidation(type="list", formula1='"uno,dos"', allow_blank=True)
    sheet.add_data_validation(validation)
    validation.add("E2:E3")
    ob = book.create_sheet("OB")
    ob["A1"] = "=1+2"
    ob.column_dimensions["A"].width = 27
    ob.sheet_state = "hidden"
    expired = book.create_sheet("Medidas vencidas")
    expired["A1"] = "No procesar"
    other = book.create_sheet("Otra")
    other["A1"] = "Conservar"
    book.save(path)
    return path.read_bytes()


def _work(path):
    # WP-21: la exportación vigente es Work.export (personal), sin la base SQLite retirada.
    original = _source(path)
    work = Work(defaults())
    work.analyze(path, "ESPERA", as_of=date(2026, 9, 8))
    return work, original


def _approved_excluded(path):
    work, original = _work(path)
    first, second = work.rows
    apply(work, [first.id], review={"OBSERVACION": "=texto de prueba"})
    apply(work, [second.id], excluded=True)
    return work, original


def test_preserved_export_uses_archived_bytes_and_marks_excluded(tmp_path):
    source = tmp_path / "entrada.xlsx"
    work, original = _approved_excluded(source)
    source.write_bytes(b"cambio externo")
    source.unlink()

    target = tmp_path / "salida.xlsx"
    result = work.export(target, backend="portable", reduced_fidelity=True)

    assert result == str(target.resolve())
    expected = load_workbook(BytesIO(original), data_only=False)
    generated = load_workbook(target, data_only=False)
    try:
        assert generated.sheetnames[:4] == expected.sheetnames
        assert generated["OB"]["A1"].value == "=1+2"
        assert generated["OB"].sheet_state == "hidden"
        assert generated["OB"].column_dimensions["A"].width == 27
        assert generated["Medidas vencidas"]["A1"].value == "No procesar"
        sheet = generated["Espera"]
        assert sheet.freeze_panes == "A2"
        assert sheet.auto_filter.ref == "A1:E3"
        assert sheet.column_dimensions["B"].width == 31
        assert sheet["A1"].font.bold is True
        assert sheet["C2"].fill.fgColor.rgb == "0092D050"
        validations = sheet.data_validations.dataValidation
        assert len(validations) == 2
        assert any('PC_IE,PC_INFO,NOMENCL' in str(item.formula1) for item in validations)
        headers = [cell.value for cell in sheet[1]]
        observation = headers.index("NURUS_OBSERVACION_FINAL") + 1
        state = headers.index("NURUS_ESTADO_REVISION") + 1
        # Decisión del usuario (11 de octubre de 2026): texto literal sin apóstrofo visible, nunca fórmula.
        assert sheet.cell(2, observation).value == "=texto de prueba"
        assert sheet.cell(2, observation).data_type == "s"
        # La columna OBSERVACION revisada tampoco lleva apóstrofo visible (auditoría 11 de octubre de 2026).
        reviewed = sheet.cell(2, headers.index("OBSERVACION") + 1)
        assert (reviewed.value, reviewed.data_type) == ("=texto de prueba", "s")
        assert sheet.cell(2, state).value == "REVISADO"
        assert sheet.cell(3, state).value == "EXCLUIDO"
        rules = list(sheet.conditional_formatting)
        assert rules
        assert "EXCLUIDO" in str(sheet.conditional_formatting[rules[0]][0].formula)
        trace = generated["NURUS_TRAZABILIDAD"]
        assert trace.sheet_state == "hidden"
        assert trace["B2"].value == "CONSTANCIA_REVISADA"
        assert trace["B5"].value == work.source_hash
    finally:
        expected.close()
        generated.close()


def test_proposal_export_does_not_require_human_approval(tmp_path):
    source = tmp_path / "entrada.xlsx"
    work, original = _work(source)
    assert not any(row.review for row in work.rows)

    source.unlink()
    target = tmp_path / "propuestas.xlsx"
    work.export(target, backend="portable", reduced_fidelity=True)

    expected = load_workbook(BytesIO(original), data_only=False)
    generated = load_workbook(target, data_only=False)
    try:
        assert generated.sheetnames[:4] == expected.sheetnames
        assert generated["OB"]["A1"].value == "=1+2"
        sheet = generated["Espera"]
        headers = [cell.value for cell in sheet[1]]
        proposal = headers.index("NURUS_PROPUESTA") + 1
        state = headers.index("NURUS_ESTADO_REVISION") + 1
        assert sheet.cell(2, proposal).value
        assert sheet.cell(2, state).value == "PROPUESTA"
        trace = generated["NURUS_TRAZABILIDAD"]
        assert trace["B2"].value == "PROPUESTA_NO_REVISADA"
        assert [sheet.cell(row, state).value for row in (2, 3)] == ["PROPUESTA", "PROPUESTA"]
    finally:
        expected.close()
        generated.close()


def test_preserved_export_requires_explicit_portable_confirmation(tmp_path):
    work, _ = _approved_excluded(tmp_path / "entrada.xlsx")
    with pytest.raises(ExportError, match="fidelidad reducida"):
        work.export(tmp_path / "salida.xlsx", backend="portable")


def test_preserved_export_rejects_source_and_existing_destination(tmp_path):
    source = tmp_path / "entrada.xlsx"
    work, _ = _approved_excluded(source)
    # Work.export rechaza el origen antes de llegar a services.exports (mismo resguardo, otro mensaje).
    with pytest.raises(ValueError, match="origen se conserva intacto"):
        work.export(source, backend="portable", reduced_fidelity=True)
    existing = tmp_path / "existente.xlsx"
    existing.write_bytes(b"keep")
    with pytest.raises(ExportError, match="ya existe"):
        work.export(existing, backend="portable", reduced_fidelity=True)
    assert existing.read_bytes() == b"keep"


@pytest.mark.skipif(platform.system() == "Windows", reason="requiere Excel de escritorio real")
def test_native_backend_requires_windows_excel_environment(tmp_path):
    work, _ = _approved_excluded(tmp_path / "entrada.xlsx")
    with pytest.raises(ExportError, match="Windows"):
        work.export(tmp_path / "salida.xlsx")


@pytest.mark.parametrize("suffix", [".xls", ".xlsx", ".xlsm"])
@pytest.mark.parametrize("failure", [False, True])
def test_native_savecopyas_separates_input_output_and_closes_excel(tmp_path, monkeypatch, suffix, failure):
    """Contrato COM simulado; no sustituye la aceptación en Excel real."""
    import sys
    from pathlib import Path
    from unittest.mock import MagicMock
    from nurus.services.exports import _native_preserved

    content = b"legacy xls fixture"
    if suffix != ".xls":
        buffer = BytesIO()
        source_book = Workbook()
        source_book.save(buffer)
        content = buffer.getvalue()
    target = tmp_path / ("output" + suffix)
    target.touch()
    sheet = MagicMock()
    sheet.Name = "Espera"
    sheet.UsedRange.Column = sheet.UsedRange.Row = 1
    sheet.UsedRange.Columns.Count = 1
    sheet.UsedRange.Rows.Count = 2
    cells = {}

    def cell(row, column):
        if (row, column) not in cells:
            cells[row, column] = MagicMock(Value2="RIT" if (row, column) == (1, 1) else None)
        return cells[row, column]

    sheet.Cells.side_effect = cell
    book = MagicMock()
    book.Worksheets.return_value = sheet
    book.Worksheets.Count = 1
    placeholder = MagicMock()
    app = MagicMock()
    app.Workbooks.Add.return_value = placeholder
    opened = []

    def open_book(path, *args):
        opened.append(Path(path))
        assert Path(path).read_bytes() == content
        assert Path(path) != target
        assert args == (0, False)
        return book

    def save_copy_as(path):
        assert path == str(target)
        assert not target.exists()
        if failure:
            raise RuntimeError("SaveCopyAs refused")
        target.write_bytes(b"output-created-by-fake-excel")

    app.Workbooks.Open.side_effect = open_book
    book.SaveCopyAs.side_effect = save_copy_as
    pythoncom = MagicMock()
    win32com = MagicMock()
    win32com.client.DispatchEx.return_value = app
    monkeypatch.setitem(sys.modules, "pythoncom", pythoncom)
    monkeypatch.setitem(sys.modules, "win32com", win32com)
    monkeypatch.setitem(sys.modules, "win32com.client", win32com.client)
    monkeypatch.setattr("platform.system", lambda: "Windows")
    snapshot = {
        "batch": {
            "primary_sheet": "Espera",
            "header_row": 1,
            "source_hash": "hash",
            "mode": "ESPERA",
            "export_stage": "proposal",
        },
        "records": [
            {
                "record_id": "id",
                "source_sheet": "Espera",
                "source_row": 2,
                "decision": "excluded",
                "evaluation_status": "excluded",
                "edited_observation": "Propuesta",
                "edit_reason": "",
                "rule_ids_json": "[]",
                "source_hash": "hash",
            }
        ],
    }
    if failure:
        with pytest.raises(ExportError, match="guardar la copia"):
            _native_preserved(content, target, snapshot)
    else:
        _native_preserved(content, target, snapshot)
        assert target.read_bytes() == b"output-created-by-fake-excel"
    book.Save.assert_not_called()
    book.SaveAs.assert_not_called()
    book.SaveCopyAs.assert_called_once_with(str(target))
    placeholder.Close.assert_called_once_with(False)
    book.Close.assert_called_once_with(False)
    app.Quit.assert_called_once()
    pythoncom.CoUninitialize.assert_called_once()
    assert sheet.Range.return_value.Interior.Color == 255 + 242 * 256 + 204 * 65536
    assert opened and not opened[0].exists()


def test_snapshot_save_keeps_primary_session(tmp_path):
    # Un respaldo (anteriores/incorporada) no desvía los recibos de borradores fuera de la sesión principal.
    work, _ = _work(tmp_path / "entrada.xlsx")
    work.save(tmp_path / "sesion")
    work.save(tmp_path / "respaldo", snapshot=True)
    assert work.storage_directory == str(tmp_path / "sesion")
    assert (tmp_path / "respaldo" / "trabajo.json").is_file()
