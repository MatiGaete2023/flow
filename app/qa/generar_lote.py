"""E-Q5 · Oráculo de grabación del lote de listados Integral de Flow Python (`descargador/lotes.py::BatchRunner`).

Ejecuta `BatchRunner` con el puente ficticio de `descargador/test_lectura_bitacoras.py` (listado con enlaces de
bitácora, SITFA ficticio, sin red) y graba cada orden al puente con su respuesta, `resumen.json`, los manifiestos
`verificacion NNN.json` y las huellas de los XLS. La prueba Rust reproduce esas respuestas con `lote::ejecutar` y
compara. Son también la entrada del paquete CSMP (E-Q5): el lote queda guardado en el fixture.

Uso (desde la raíz): python app/qa/generar_lote.py [salida.json]   (por omisión app/tests/datos/lote.json)
Campos no deterministas declarados: `fecha_hora` y `fecha_descarga` (hora local del lote).
"""

import base64
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

from datetime import datetime  # noqa: E402
from zoneinfo import ZoneInfo  # noqa: E402

import motor  # noqa: E402

# Reloj fijo: carpeta del lote, `fecha_hora` y `fecha_descarga` quedan deterministas.
AHORA = datetime(2026, 10, 8, 13, 0, 0, 123456, tzinfo=ZoneInfo('America/Santiago'))
motor.now = lambda: AHORA

from lotes import Batch, BatchRunner  # noqa: E402
from perfiles import selection_profile  # noqa: E402
from test_lotes import CATALOG, FakeBridge  # noqa: E402


def rut_invalido(cuerpo):
    """RUT ficticio con dígito verificador INVÁLIDO a propósito (nunca una identidad real)."""
    suma, factor = 0, 2
    for d in reversed(str(cuerpo)):
        suma += int(d) * factor
        factor = 2 if factor == 7 else factor + 1
    valido = {11: '0', 10: 'K'}.get(11 - suma % 11, str(11 - suma % 11))
    return f'{cuerpo}-' + next(c for c in '0123456789' if c != valido)


def tabla(tabla_id, tribunal, pestana, numero, enlaces):
    """Listado (con enlaces de ingreso) o XLS (sin enlaces) de la página `numero`: una persona ficticia por página."""
    tipo = {'Espera': '11', 'Cumplimiento': '12', 'Informes': '13'}.get(pestana, '12')
    rit, rut, nombre = f'X-{numero}-2099', rut_invalido(11111110 + numero), f'Persona Ficticia {numero}'
    causa, ingreso = str(100 + numero), str(200 + numero)
    acciones = (
        f"<a onclick=\"ShowHistoria('{tribunal}','{rit}','{causa}')\">Causa</a>"
        f"<a onclick=\"ShowObservaciones('{causa}','{tribunal}','{ingreso}','{tipo}','3','0')\">Obs</a>"
        f"<a onclick=\"ShowInformesProgramados('{causa}','300','{tribunal}','400','{rit}','01/03/2026',"
        f"'PRM Centro Ficticio','{nombre}','6 Meses','3 Meses','{ingreso}','1')\">Inf</a>"
    ) if enlaces else ''
    return (f'<table id="{tabla_id}"><tr><th>RIT</th><th>RUT</th><th>Nombre</th><th>Derivación</th><th>Acciones</th></tr>'
            f'<tr><td>{rit}</td><td>{rut}</td><td>{nombre}</td><td>PRM Centro Ficticio</td><td>{acciones}</td></tr></table>')


class ConVinculos(FakeBridge):
    """`test_lotes.FakeBridge` con un listado de Seguimiento que trae enlaces de ingreso comprobables y un XLS coherente."""

    def __init__(self):
        super().__init__(total=2)

    def call(self, command, payload=None, timeout=75):
        resultado = super().call(command, payload, timeout)
        if command not in ('search', 'download') or self.selection is None:
            return resultado
        perfil = selection_profile(self.selection, CATALOG)
        if command == 'search':
            fuente = base64.b64decode(resultado['body']).decode('cp1252')
            inicio = fuente.index('<table')
            fin = fuente.index('</table>', inicio) + 8
            nueva = tabla(perfil.table_id, self.selection.tribunal, self.selection.tab, self.number, True)
            # Sin `<meta charset>`, el listado se lee como windows-1252 (como lo entrega SITFA).
            resultado['body'] = base64.b64encode((fuente[:inicio] + nueva + fuente[fin:]).encode('cp1252')).decode()
        else:
            xls = '<html><meta charset="utf-8">' + tabla('x', self.selection.tribunal, self.selection.tab, self.number, False) + '</html>'
            resultado['body'] = base64.b64encode(xls.encode()).decode()
        return resultado


class Grabador(ConVinculos):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.guion = []

    def call(self, command, payload=None, timeout=75):
        try:
            respuesta = super().call(command, payload, timeout)
        except Exception as e:
            self.guion.append({'comando': command, 'payload': payload, 'error': str(e)})
            raise
        self.guion.append({'comando': command, 'payload': payload, 'respuesta': respuesta})
        return respuesta


def sin_horas(valor):
    """`fecha_hora` y `fecha_descarga` se declaran: son la hora local del lote."""
    if isinstance(valor, dict):
        return {k: ('<HORA>' if k in ('fecha_hora', 'fecha_descarga') else sin_horas(v)) for k, v in valor.items()}
    if isinstance(valor, list):
        return [sin_horas(v) for v in valor]
    return valor


ESCENARIOS = {
    'cumplimiento': Batch(('49',), ('2',), ('Cumplimiento',)),
    'dos_modalidades_dos_pestanas': Batch(('49',), ('1', '2'), ('Espera', 'Cumplimiento')),
}


def escenario(nombre, lote):
    puente = Grabador()
    with tempfile.TemporaryDirectory() as tmp:
        BatchRunner(puente, CATALOG, lambda *a: None, threading.Event(), pause=0).run(lote, Path(tmp))
        carpeta = next(p for p in Path(tmp).iterdir() if p.is_dir())
        jsons = {p.name: sin_horas(json.loads(p.read_text(encoding='utf-8'))) for p in sorted(carpeta.glob('*.json'))}
        # Los JSON del lote los escribe Python en modo texto: en Windows salen con CRLF. Se fijan en LF para que el
        # oráculo se regenere con los mismos bytes en cualquier sistema (los XLS y PDF se conservan tal cual).
        archivos = {p.name: base64.b64encode(p.read_bytes().replace(b'\r\n', b'\n') if p.suffix == '.json' else p.read_bytes()).decode()
                    for p in sorted(carpeta.iterdir()) if p.is_file() and p.suffix in ('.json', '.xls', '.pdf')}
        huellas = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(carpeta.iterdir())
                   if p.is_file() and p.suffix in ('.xls', '.pdf')}
    return {
        'escenario': nombre,
        'lote': {'tribunales': list(lote.tribunals), 'modalidades': list(lote.modalities), 'pestanas': list(lote.tabs)},
        'guion': puente.guion,
        'carpeta': carpeta.name,
        'jsons': jsons,
        'huellas': huellas,
        'archivos': archivos,
    }


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'lote.json'
    datos = {'catalogo': CATALOG, 'escenarios': [escenario(n, b) for n, b in ESCENARIOS.items()]}
    destino.parent.mkdir(parents=True, exist_ok=True)
    destino.write_text(json.dumps(datos, ensure_ascii=False, indent=1) + '\n', encoding='utf-8', newline='\n')
    for e in datos['escenarios']:
        print(e['escenario'], len(e['guion']), 'órdenes', sorted(e['jsons']), sorted(e['huellas']))


if __name__ == '__main__':
    main()
