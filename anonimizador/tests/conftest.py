"""Fixtures sintéticos. No hay datos reales ni RUT literales con dígito verificador válido."""

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from anonimizador_csmp.textengine import Options, Session, check_digit  # noqa: E402


def valid_rut(number=17654321, dots=True):
    """RUT con dígito verificador válido, calculado en la prueba para no dejar literales en el repositorio."""
    digits = f"{number:,}".replace(",", ".") if dots else str(number)
    return f"{digits}-{check_digit(number)}"


@pytest.fixture
def rut():
    return valid_rut()


@pytest.fixture
def session():
    return Session(Options(mode="conservador"))


@pytest.fixture
def strict_session():
    return Session(Options(mode="estricto"))


def make_xlsx(path, rows, title="ESPERA", extra_sheets=()):
    from openpyxl import Workbook

    book = Workbook()
    sheet = book.active
    sheet.title = title
    for row in rows:
        sheet.append(row)
    for name, data in extra_sheets:
        other = book.create_sheet(name)
        for row in data:
            other.append(row)
    book.save(path)
    book.close()
    return path
