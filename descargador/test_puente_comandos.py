"""El puente real acepta cada comando que usan los lectores; los fakes de las demás pruebas no lo comprueban."""

import pytest

import motor as m
from puente import Bridge
from test_motivo_evidencia import respuesta


@pytest.fixture
def bridge():
    b = Bridge()
    yield b
    b.close()


def test_calendario_por_ingreso_llega_a_la_extension(bridge):
    # Prueba del usuario 29-oct-2026: todos los calendarios fallaban con «Accion no permitida.» (WD-1).
    respuesta(bridge, {'ok': True, 'result': {'status': 200}})
    assert bridge.call('calendar_open', {'args': ['1'] * 12, 'timeout': 60000}, timeout=5) == {'status': 200}


def test_error_del_calendario_conserva_su_codigo(bridge):
    respuesta(bridge, {'ok': False, 'error': 'CALENDAR_LINK'})
    with pytest.raises(m.PocError) as error:
        bridge.call('calendar_open', {'args': ['1'] * 12}, timeout=5)
    assert error.value.code == 'CALENDAR_LINK' and 'calendario de informes' in str(error.value)


def test_comando_desconocido_sigue_rechazado(bridge):
    with pytest.raises(m.PocError, match='Accion no permitida'):
        bridge.call('escribir_rus')
