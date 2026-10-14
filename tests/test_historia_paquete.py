"""Paquete ZIP ficticio con historia de la causa, de punta a punta por import_capture."""

from datetime import date

import pytest
from historia_fixtures import captura_con_historia

from nurus.capture import write_capture
from nurus.personal.capture_input import import_capture
from nurus.personal.config import defaults

CORTE = date(2026, 10, 8)
INGRESO = ('Ingreso efectivo', '2026-10-02', 41)


def _importar(tmp_path, tramites, cobertura='COMPLETA', nombre='h'):
    metadata, tablas = captura_con_historia(tramites, cobertura)
    return import_capture(write_capture(tmp_path / f'{nombre}.zip', metadata, tablas), defaults(), as_of=CORTE)


def test_la_historia_de_la_causa_llega_a_los_ingresos_de_esa_causa(tmp_path):
    trabajo = _importar(tmp_path, [INGRESO])
    r0, r1, r2, r3 = trabajo.rows
    for fila in (r0, r1):  # causa-ficticia-0
        assert (
            'folio 41.' in fila.observation and 'Medida revisada. En la historia' in fila.observation.split(': ', 1)[1]
        )
        assert [a[:2] for a in fila.avisos] == [(2, 'R3_INGRESO_EFECTIVO')] and fila.actions == []
    for fila in (r2, r3):  # causa-ficticia-1: sin historia, regla original
        assert 'ESPERA.E05_PROYECTO_Y_CORREO' in fila.rules and fila.avisos == []


@pytest.mark.parametrize('cobertura', ['VACIA_COMPROBADA', 'FALLIDA', 'CANCELADA', 'NO_CONSULTADA'])
def test_sin_trámites_capturados_rigen_las_reglas_originales(tmp_path, cobertura):
    con = _importar(tmp_path, [], cobertura, 'con')
    base = _importar(tmp_path, [], 'VACIA_COMPROBADA', 'base')
    assert [(r.observation, r.rules, r.avisos) for r in con.rows] == [
        (r.observation, r.rules, r.avisos) for r in base.rows
    ]


def test_cobertura_parcial_cuenta_como_presencia(tmp_path):
    trabajo = _importar(tmp_path, [INGRESO], 'PARCIAL')
    assert [a[:2] for a in trabajo.rows[0].avisos] == [(2, 'R3_INGRESO_EFECTIVO')]


def test_importar_de_nuevo_con_otra_regla_no_reutiliza_un_trabajo_antiguo(tmp_path):
    trabajo = _importar(tmp_path, [INGRESO])
    assert trabajo.ruleset_version.endswith('+historia-1')
