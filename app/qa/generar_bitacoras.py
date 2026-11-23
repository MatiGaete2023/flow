"""E-Q4 · Oráculo de grabación del lote de bitácoras de Flow Python (`descargador/lectura_bitacoras.py::DiaryRunner`).

Ejecuta `DiaryRunner` con el puente ficticio de `descargador/test_lectura_bitacoras.py` (SITFA ficticio, sin red) y
graba, por escenario, cada orden al puente con su respuesta, el manifiesto `bitacoras.json` resultante y las huellas
de las instantáneas HTML. La prueba Rust reproduce esas respuestas y compara órdenes y manifiesto.

Uso (desde la raíz): python app/qa/generar_bitacoras.py [salida.json]   (por omisión app/tests/datos/bitacoras.json)
Campos no deterministas declarados: `fecha` del manifiesto y la carpeta del lote (marca de tiempo local).
"""

import hashlib
import json
import sys
import tempfile
import threading
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'descargador'), str(RAIZ / 'src'), str(RAIZ / 'tests')]

from csmp_shared import parser  # noqa: E402

parser()

import base64  # noqa: E402
from datetime import datetime  # noqa: E402
from zoneinfo import ZoneInfo  # noqa: E402

import motor  # noqa: E402

# Reloj fijo: carpetas, `fecha` y la ventana de reutilización (12 h del mismo día local) quedan deterministas. La
# captura ficticia es 2026-10-08T12:00:00Z (09:00 en Santiago).
AHORA = datetime(2026, 10, 8, 13, 0, 0, 123456, tzinfo=ZoneInfo('America/Santiago'))
motor.now = lambda: AHORA

from lectura_bitacoras import DiaryRunner  # noqa: E402
from lotes import Batch  # noqa: E402
from test_lectura_bitacoras import Bridge  # noqa: E402
from test_lotes import CATALOG  # noqa: E402


from test_calendario_lectura import CalendarBridge  # noqa: E402


def grabador(base):
    class Grabador(base):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, **kwargs)
            self.guion = []

        def call(self, command, payload=None, timeout=75):
            try:
                respuesta = super().call(command, payload, timeout)
            except Exception as e:  # el error del puente también es parte del guion
                self.guion.append({'comando': command, 'payload': payload, 'error': str(e)})
                raise
            self.guion.append({'comando': command, 'payload': payload, 'respuesta': respuesta})
            return respuesta

    return Grabador


Grabador = grabador(Bridge)
GrabadorCalendario = grabador(CalendarBridge)


ESCENARIOS = {
    'basico': (dict(), Batch(('49',), ('2',), ('Cumplimiento',))),
    'identidad_ajena': (dict(wrong=True), Batch(('49',), ('2',), ('Cumplimiento',))),
    'sin_vinculo': (dict(missing=True), Batch(('49',), ('2',), ('Cumplimiento',))),
    'cuatro_modalidades': (dict(), Batch(tuple(CATALOG['tribunales']), ('1', '2', '3', '4'), ('Espera', 'Cumplimiento'))),
}


def escenario(nombre, opciones, lote, calendarios=False):
    puente = (GrabadorCalendario if calendarios else Grabador)(**opciones)
    with tempfile.TemporaryDirectory() as tmp:
        corredor = DiaryRunner(puente, CATALOG, lambda *a: None, threading.Event(), pause=0, calendars=calendarios)
        resultado = corredor.run(lote, Path(tmp))
        carpeta = Path(resultado['manifest']).parent
        carpeta_nombre = carpeta.name
        manifiesto = json.loads(Path(resultado['manifest']).read_text(encoding='utf-8'))
        instantaneas = {
            p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(carpeta.glob('*.html'))
        }
    manifiesto['fecha'] = '<FECHA>'
    claves = ('estado', 'leidas', 'fallidas', 'consultas_fallidas', 'detalle', 'faltantes_por_tribunal',
              'calendarios_leidos', 'calendarios_fallidos')
    return {
        'escenario': nombre,
        'calendarios': calendarios,
        'ahora': AHORA.isoformat(),
        'carpeta': carpeta_nombre,
        'lote': {'tribunales': list(lote.tribunals), 'modalidades': list(lote.modalities), 'pestanas': list(lote.tabs)},
        'guion': puente.guion,
        'manifiesto': manifiesto,
        'instantaneas': instantaneas,
        'resultado': {k: resultado[k] for k in claves},
    }


def carpeta_json(carpeta):
    """Archivos de una carpeta de lote (manifiesto e instantáneas) para reconstruirla en la prueba Rust."""
    return {p.name: base64.b64encode(p.read_bytes()).decode()
            for p in sorted(carpeta.iterdir()) if p.name == 'bitacoras.json' or p.suffix == '.html'}


def dos_corridas(nombre, primera, segunda, lote, retomar, cancelada=False, calendarios=False):
    """Primera corrida (estado inicial) y segunda grabada: retomando la misma carpeta o reutilizando otro lote."""
    clase = GrabadorCalendario if calendarios else Grabador
    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp)
        inicial = DiaryRunner(clase(**primera), CATALOG, lambda *a: None, threading.Event(), pause=0,
                              calendars=calendarios).run(lote, base)
        carpeta_inicial = Path(inicial['folder'])
        archivos = carpeta_json(carpeta_inicial)
        puente = clase(**segunda)
        if retomar:
            cancelar = threading.Event()
            if cancelada:
                cancelar.set()  # se cancela antes de volver a enumerar: lo leído debe quedar en anteriores_no_reenumeradas
            corredor = DiaryRunner(puente, CATALOG, lambda *a: None, cancelar, pause=0, calendars=calendarios)
            resultado = corredor.run(lote, base, resume=carpeta_inicial)
        else:
            AHORA_SEGUNDA = AHORA.replace(minute=30)
            motor.now = lambda: AHORA_SEGUNDA
            try:
                corredor = DiaryRunner(puente, CATALOG, lambda *a: None, threading.Event(), pause=0, reuse_base=base)
                resultado = corredor.run(lote, base)
            finally:
                motor.now = lambda: AHORA
        carpeta = Path(resultado['manifest']).parent
        manifiesto = json.loads(Path(resultado['manifest']).read_text(encoding='utf-8'))
        instantaneas = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(carpeta.glob('*.html'))}
    manifiesto['fecha'] = '<FECHA>'
    claves = ('estado', 'leidas', 'fallidas', 'consultas_fallidas', 'detalle', 'faltantes_por_tribunal',
              'calendarios_leidos', 'calendarios_fallidos')
    return {
        'escenario': nombre,
        'calendarios': calendarios,
        'lote': {'tribunales': list(lote.tribunals), 'modalidades': list(lote.modalities), 'pestanas': list(lote.tabs)},
        'inicial': {'carpeta': carpeta_inicial.name, 'archivos': archivos},
        'modo': 'retomar' if retomar else 'reutilizar',
        'cancelada': cancelada,
        'ahora': (AHORA if retomar else AHORA.replace(minute=30)).isoformat(),
        'guion': puente.guion,
        'carpeta': carpeta.name,
        'manifiesto': manifiesto,
        'instantaneas': instantaneas,
        'resultado': {k: resultado[k] for k in claves},
    }


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'bitacoras.json'
    escenarios = [escenario(n, o, b) for n, (o, b) in ESCENARIOS.items()]
    uno = Batch(('49',), ('2',), ('Cumplimiento',))
    escenarios.append(dos_corridas('retomar_tras_falla', dict(wrong=True), dict(), uno, retomar=True))
    escenarios.append(dos_corridas('reutilizar_de_otro_lote', dict(), dict(), uno, retomar=False))
    escenarios.append(dos_corridas('retomar_cancelado', dict(wrong=True), dict(), uno, retomar=True, cancelada=True))
    escenarios.append(escenario('calendario', dict(), uno, calendarios=True))
    escenarios.append(escenario('calendario_ajeno', dict(wrong=('202',)), uno, calendarios=True))
    escenarios.append(dos_corridas('retomar_con_calendario', dict(wrong=('202',)), dict(), uno, retomar=True, calendarios=True))
    datos = {'catalogo': CATALOG, 'ahora': AHORA.isoformat(), 'escenarios': escenarios}
    destino.parent.mkdir(parents=True, exist_ok=True)
    destino.write_text(json.dumps(datos, ensure_ascii=False, indent=1) + '\n', encoding='utf-8', newline='\n')
    for e in datos['escenarios']:
        print(e['escenario'], e['resultado']['estado'], len(e['guion']), 'órdenes', len(e['manifiesto']['registros']),
              'registros')


if __name__ == '__main__':
    main()
