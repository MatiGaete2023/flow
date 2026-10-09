"""Sonda de anonimización: sin filtraciones y con las mismas reglas antes y después."""
from datetime import date
import importlib.util
from pathlib import Path
import sys

from openpyxl import Workbook, load_workbook

from nurus.personal.config import defaults
from nurus.personal.work import Work

ROOT = Path(__file__).resolve().parents[1]


def _tool(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'tools' / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


anonimizar = _tool('anonimizar_excel')
check_real_ids = _tool('check_real_ids')
CORTE = date(2026, 10, 9)


def _rut(number):
    return f'{number}-{check_real_ids.check_digit(str(number))}'


PERSONAS = [
    ('X-501-2025', 'BENJAMIN INVENTADO SONDA', _rut(23456789)),
    ('X-501-2025', 'TOMASA INVENTADO SONDA', _rut(24567891)),
    ('X-777-2024', 'AMANDA IMAGINARIA PRUEBA', _rut(21345679)),
    ('X-888-2023', 'RODRIGO FICCION SONDEO', _rut(22345671)),
]
CURADOR = 'HERMINIA CURADORA SUPUESTA'


def _sonda(path):
    book = Workbook()
    sheet = book.active
    sheet.title = 'ESPERA'
    sheet.append(['Reporte de espera ficticio'])
    sheet.append(['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'NOMBRE CENTRO', 'T ESPERA', 'CURADOR', 'OBSERVACION'])
    for index, (rit, name, rut) in enumerate(PERSONAS):
        note = f'Se informa que {name}, RUT {rut}, causa {rit}, mantiene curador {CURADOR}.' if index == 0 else ''
        sheet.append([rit, 'LAJA', name, rut, 'PRM EJEMPLO', (5, 95, 200, 400)[index], CURADOR if index == 0 else '', note])
    book.save(path)


def _reglas(path):
    work = Work(defaults())
    work.analyze(path, 'ESPERA', as_of=CORTE)
    return [(row.values.get('RIT') is not None, sorted(map(str, row.rules)), len(row.actions), len(row.warnings)) for row in work.rows]


def test_copia_anonima_sin_filtraciones_y_mismas_reglas(tmp_path):
    original, copia = tmp_path / 'original.xlsx', tmp_path / 'copia.xlsx'
    _sonda(original)
    assert check_real_ids.scan([original], set())
    assert anonimizar.main([str(original), str(copia)]) == 0
    texto = ' '.join(str(cell.value) for sheet in load_workbook(copia).worksheets for row in sheet.iter_rows() for cell in row if cell.value is not None)
    secretos = [CURADOR, *CURADOR.split()]
    for rit, name, rut in PERSONAS:
        secretos += [rit, rut, rut.split('-')[0], name, *name.split()]
    filtraciones = [secret for secret in secretos if secret.upper() in texto.upper()]
    assert filtraciones == []
    assert check_real_ids.scan([copia], set()) == []
    sheet = load_workbook(copia)['ESPERA']
    assert sheet['A3'].value == sheet['A4'].value != sheet['A5'].value
    assert sheet['A1'].value == 'Reporte de espera ficticio'
    reglas = _reglas(original)
    assert len({tuple(regla[1]) for regla in reglas}) > 1
    assert reglas == _reglas(copia)


def test_nunca_sobrescribe(tmp_path):
    original = tmp_path / 'original.xlsx'
    _sonda(original)
    before = original.read_bytes()
    assert anonimizar.main([str(original), str(original)]) == 1
    existing = tmp_path / 'existente.xlsx'
    existing.write_bytes(b'previo')
    assert anonimizar.main([str(original), str(existing)]) == 1
    assert original.read_bytes() == before and existing.read_bytes() == b'previo'
