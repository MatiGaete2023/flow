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

from lectura_bitacoras import DiaryRunner  # noqa: E402
from lotes import Batch  # noqa: E402
from test_lectura_bitacoras import Bridge  # noqa: E402
from test_lotes import CATALOG  # noqa: E402


class Grabador(Bridge):
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


ESCENARIOS = {
    'basico': (dict(), Batch(('49',), ('2',), ('Cumplimiento',))),
    'identidad_ajena': (dict(wrong=True), Batch(('49',), ('2',), ('Cumplimiento',))),
    'sin_vinculo': (dict(missing=True), Batch(('49',), ('2',), ('Cumplimiento',))),
    'cuatro_modalidades': (dict(), Batch(tuple(CATALOG['tribunales']), ('1', '2', '3', '4'), ('Espera', 'Cumplimiento'))),
}


def escenario(nombre, opciones, lote):
    puente = Grabador(**opciones)
    with tempfile.TemporaryDirectory() as tmp:
        resultado = DiaryRunner(puente, CATALOG, lambda *a: None, threading.Event(), pause=0).run(lote, Path(tmp))
        carpeta = Path(resultado['manifest']).parent
        manifiesto = json.loads(Path(resultado['manifest']).read_text(encoding='utf-8'))
        instantaneas = {
            p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(carpeta.glob('bitacora-*.html'))
        }
    manifiesto['fecha'] = '<FECHA>'
    claves = ('estado', 'leidas', 'fallidas', 'consultas_fallidas', 'detalle', 'faltantes_por_tribunal')
    return {
        'escenario': nombre,
        'lote': {'tribunales': list(lote.tribunals), 'modalidades': list(lote.modalities), 'pestanas': list(lote.tabs)},
        'guion': puente.guion,
        'manifiesto': manifiesto,
        'instantaneas': instantaneas,
        'resultado': {k: resultado[k] for k in claves},
    }


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'bitacoras.json'
    datos = {'catalogo': CATALOG, 'escenarios': [escenario(n, o, b) for n, (o, b) in ESCENARIOS.items()]}
    destino.parent.mkdir(parents=True, exist_ok=True)
    destino.write_text(json.dumps(datos, ensure_ascii=False, indent=1) + '\n', encoding='utf-8', newline='\n')
    for e in datos['escenarios']:
        print(e['escenario'], e['resultado']['estado'], len(e['guion']), 'órdenes', len(e['manifiesto']['registros']),
              'registros')


if __name__ == '__main__':
    main()
