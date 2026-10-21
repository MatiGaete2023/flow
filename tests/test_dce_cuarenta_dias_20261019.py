"""WA-7: el diagnóstico DCE se pide en cuenta pasados 10 días del vencimiento RUS (40 días desde el ingreso)."""

from datetime import date

import pytest

from nurus.personal.config import defaults
from nurus.personal.work import Work
from test_personal_flow import source

VENCIMIENTO = date(2026, 9, 12)


def _fila(tmp_path, program, as_of):
    return Work(defaults()).analyze(source(tmp_path, 'INFORMES', program=program), 'INFORMES', as_of=as_of).rows[0]


@pytest.mark.parametrize('as_of', [date(2026, 9, 13), date(2026, 9, 22)])
def test_dce_vencido_hasta_diez_dias_recibe_correo_con_fecha_limite(tmp_path, as_of):
    row = _fila(tmp_path, 'DCE CENTRO', as_of)
    assert row.actions == ['programa_por_vencer']
    assert 'PC_INFO' not in row.actions
    assert 'debe ser remitido a más tardar el 22 de septiembre de 2026' in row.observation


def test_dce_vencido_mas_de_diez_dias_pide_cuenta(tmp_path):
    row = _fila(tmp_path, 'DCE CENTRO', date(2026, 9, 23))
    assert row.actions == ['programa_vencido', 'PC_INFO']
    assert 'informe diagnóstico vencido desde el 12 de septiembre de 2026' in row.observation


def test_programas_no_dce_mantienen_el_pide_cuenta_desde_el_primer_dia(tmp_path):
    row = _fila(tmp_path, 'PRM CENTRO', date(2026, 9, 13))
    assert row.actions == ['programa_vencido', 'PC_INFO']


def test_dce_por_vencer_conserva_su_fecha_original(tmp_path):
    row = _fila(tmp_path, 'DCE CENTRO', date(2026, 9, 8))
    assert 'debe ser remitido a más tardar el 12 de septiembre de 2026' in row.observation
