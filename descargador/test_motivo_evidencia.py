"""El error de la evidencia PDF nombra su motivo técnico y una extensión de otra carpeta se identifica."""

import threading
import time

import pytest

import motor as m
from contrato_integral import EXTENSION_DIR, PROTOCOLS, VERSION, verify_extension
from puente import Bridge


def respuesta(bridge, result):
    bridge.origin = 'chrome-extension://prueba'
    bridge.last_seen = time.monotonic()

    def responder():
        task = bridge.tasks.get(timeout=5)
        with bridge.lock:
            entry = bridge.pending[task['id']]
            entry['result'] = {'id': task['id'], **result}
            entry['event'].set()

    threading.Thread(target=responder, daemon=True).start()


@pytest.fixture
def bridge():
    b = Bridge()
    yield b
    b.close()


def test_captura_fallida_informa_el_motivo(bridge):
    respuesta(bridge, {'ok': False, 'error': 'EVIDENCE_CAPTURE', 'motivo': 'pantalla'})
    with pytest.raises(m.PocError) as error:
        bridge.call('evidence_capture', {'digest': 'a' * 64}, timeout=5)
    assert 'No se pudo capturar la consulta vacía' in str(error.value)
    assert str(error.value).endswith('Motivo técnico: pantalla.')


def test_motivo_fuera_del_vocabulario_no_se_muestra(bridge):
    respuesta(bridge, {'ok': False, 'error': 'EVIDENCE_CAPTURE', 'motivo': 'Juan Pérez 12.345.678-9'})
    with pytest.raises(m.PocError) as error:
        bridge.call('evidence_capture', {'digest': 'a' * 64}, timeout=5)
    assert 'Motivo' not in str(error.value) and 'Juan' not in str(error.value)


class Capacidades:
    def __init__(self, version, protocolos):
        self.result = {'version': version, 'protocolos': protocolos, 'escritura_rus': False}

    def call(self, command, payload=None, timeout=75):
        return self.result


def test_extension_de_otra_carpeta_se_identifica_con_la_ruta_a_cargar():
    anterior = {k: v for k, v in PROTOCOLS.items() if k != 'pdf_copia_fija'}
    with pytest.raises(m.PocError) as error:
        verify_extension(Capacidades('2.8.0', anterior))
    texto = str(error.value)
    assert 'copia anterior' in texto and '2.8.0' in texto and VERSION in texto and str(EXTENSION_DIR) in texto
    assert verify_extension(Capacidades(VERSION, dict(PROTOCOLS)))['version'] == VERSION
