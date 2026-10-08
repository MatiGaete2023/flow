import pytest
from datetime import date

from carga import HEADERS, record_rows, records_in_grid, signatures
from motor import PocError


def report_grid(*, header=None, tribunals=("Tribunal ficticio 777", "Tribunal ficticio 888")):
    header = list(HEADERS if header is None else header)
    rows = [header]
    for rit, court in zip(("X-1-2026", "Y-2-2026"), tribunals):
        row = [""] * 23
        row[0] = rit
        row[1] = court
        row[4] = "Resolución ficticia"
        row[5] = "NO"
        row[6] = "Documento ficticio"
        row[21] = "02/10/2026"
        row[22] = "09:30"
        rows.append(row)
    return rows


def test_load_report_rejects_reordered_23_column_header():
    header = list(HEADERS)
    header[4], header[5] = header[5], header[4]
    with pytest.raises(PocError, match="(?i)encabezado.*esperado.*recibido"):
        record_rows(report_grid(header=header))


def test_load_report_rejects_renamed_column_before_returning_rows():
    header = list(HEADERS)
    header[21] = "FECHA DOCUMENTO"
    with pytest.raises(PocError, match="(?i)encabezado.*esperado.*recibido"):
        record_rows(report_grid(header=header))


def test_load_report_accepts_normalized_header_and_fixed_concepcion_title():
    header = [f"  {name.lower()}  " for name in HEADERS]
    grid = [["Concepción"] * 23, *report_grid(header=header, tribunals=("Tribunal ficticio 777", "Tribunal ficticio 777"))]
    rows = record_rows(grid)
    assert [row[0] for row in rows] == ["X-1-2026", "Y-2-2026"]
    signatures_found = signatures(
        grid, "777", "Tribunal ficticio 777", date(2026, 10, 1), date(2026, 10, 3),
    )
    assert [(item["rit"], item["tribunal_codigo"]) for item in signatures_found] == [("X-1-2026", "777"), ("Y-2-2026", "777")]


def test_signature_key_ignores_flow_changes_and_keeps_legacy_alias():
    grid = report_grid(tribunals=("Tribunal ficticio 777",))
    before = signatures(grid, "777", "Tribunal ficticio 777", date(2026, 10, 1), date(2026, 10, 3))[0]
    grid[1][2] = "Funcionario ficticio nuevo"
    grid[1][10] = "01/10/2026"
    after = signatures(grid, "777", "Tribunal ficticio 777", date(2026, 10, 1), date(2026, 10, 3))[0]
    assert after["huella"] == before["huella"]
    before_alias = before["identidad"].split("huella_legacy=", 1)[1]
    after_alias = after["identidad"].split("huella_legacy=", 1)[1]
    assert after_alias != before_alias


def test_signature_identity_uses_available_document_and_hour_and_preserves_ambiguous_duplicates():
    single = report_grid(tribunals=("Tribunal ficticio 777",))
    base = signatures(single, "777", "Tribunal ficticio 777", date(2026, 10, 1), date(2026, 10, 3))[0]["huella"]
    single[1][6] = "Documento alternativo"
    with_document = signatures(single, "777", "Tribunal ficticio 777", date(2026, 10, 1), date(2026, 10, 3))[0]["huella"]
    assert with_document != base
    single[1][22] = "10:45"
    with_hour = signatures(single, "777", "Tribunal ficticio 777", date(2026, 10, 1), date(2026, 10, 3))[0]["huella"]
    assert with_hour != with_document

    grid = report_grid(tribunals=("Tribunal ficticio 777", "Tribunal ficticio 777"))
    grid[2][0] = grid[1][0]
    first, second = signatures(grid, "777", "Tribunal ficticio 777", date(2026, 10, 1), date(2026, 10, 3))
    assert first["huella"] != second["huella"]
    assert first["identidad"].startswith("huella_compuesta_ambigua_")
    grid[1][6] = "Documento dos"
    grid[2][6] = "Documento tres"
    first, second = signatures(grid, "777", "Tribunal ficticio 777", date(2026, 10, 1), date(2026, 10, 3))
    assert first["huella"] != second["huella"]
    assert all(item["identidad"].startswith("huella_compuesta_local_") for item in (first, second))

    grid[1][22] = grid[2][22] = ""
    grid[1][6] = grid[2][6] = ""
    first, second = signatures(grid, "777", "Tribunal ficticio 777", date(2026, 10, 1), date(2026, 10, 3))
    assert first["huella"] != second["huella"]
    assert all(item["identidad"].startswith("huella_compuesta_ambigua_") for item in (first, second))


def test_same_rit_in_different_selected_courts_has_distinct_signature_key():
    first_grid = report_grid(tribunals=("Tribunal ficticio 777",))
    other_grid = report_grid(tribunals=("Tribunal ficticio 888",))
    first = signatures(first_grid, "777", "Tribunal ficticio 777", date(2026, 10, 1), date(2026, 10, 3))[0]
    other = signatures(other_grid, "888", "Tribunal ficticio 888", date(2026, 10, 1), date(2026, 10, 3))[0]
    assert first["rit"] == other["rit"]
    assert first["huella"] != other["huella"]
