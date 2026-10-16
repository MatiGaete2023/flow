import pytest

from carga import HEADERS, record_rows
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
    grid = [
        ["Concepción"] * 23,
        *report_grid(header=header, tribunals=("Tribunal ficticio 777", "Tribunal ficticio 777")),
    ]
    rows = record_rows(grid)
    assert [row[0] for row in rows] == ["X-1-2026", "Y-2-2026"]
