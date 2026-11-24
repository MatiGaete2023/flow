"""E-Q5 · Oráculo del paquete CSMP_CAPTURE de Flow Python (`export_csmp_capture.export_capture`, `package_current_lot`).

Arma con Python, sobre el SITFA ficticio de `app/qa/generar_lote.py` (reloj fijo, sin red), las carpetas de lote y
el paquete. Guarda los archivos de las carpetas (para reconstruirlas en la prueba Rust) y, del ZIP de Python, el
`manifest.json` y cada CSV de `data/` tal cual. La prueba Rust arma su paquete desde las mismas carpetas y compara
manifiesto y CSV byte a byte (el manifiesto lleva la huella de cada CSV).

Uso (desde la raíz): python app/qa/generar_paquete.py [salida.json]   (por omisión app/tests/datos/paquete.json)
"""

import base64
import json
import sys
import tempfile
import threading
import zipfile
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'app' / 'qa'), str(RAIZ / 'descargador'), str(RAIZ / 'src'), str(RAIZ / 'tests')]

import generar_lote as gl  # noqa: E402  (reloj fijo y puente ficticio con vínculos)
from archivo_local import load_sources  # noqa: E402
from export_csmp_capture import export_capture_with_note  # noqa: E402
from lotes import Batch, BatchRunner  # noqa: E402
from test_lotes import CATALOG  # noqa: E402


def carpeta_json(carpeta):
    return {str(p.relative_to(carpeta)): base64.b64encode(p.read_bytes()).decode()
            for p in sorted(carpeta.rglob('*')) if p.is_file()}


def paquete_json(ruta):
    with zipfile.ZipFile(ruta) as z:
        salida = {'manifest.json': json.loads(z.read('manifest.json'))}
        for nombre in sorted(z.namelist()):
            if nombre.startswith('data/'):
                salida[nombre] = z.read(nombre).decode('utf-8')
    return salida


ESCENARIOS = {
    'listados': Batch(('49',), ('2',), ('Cumplimiento',)),
    'listados_dos_pestanas': Batch(('49',), ('1', '2'), ('Espera', 'Cumplimiento')),
}


def escenario(nombre, lote):
    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp)
        BatchRunner(gl.ConVinculos(), CATALOG, lambda *a: None, threading.Event(), pause=0).run(lote, base / 'lotes')
        carpeta = next((base / 'lotes').iterdir())
        destino = base / 'paquete.zip'
        _, nota = export_capture_with_note(load_sources([carpeta]), destino)
        return {
            'escenario': nombre,
            'lote': {'carpeta': carpeta.name, 'archivos': carpeta_json(carpeta)},
            'nota': nota,
            'paquete': paquete_json(destino),
        }


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'paquete.json'
    datos = {'catalogo': CATALOG, 'escenarios': [escenario(n, b) for n, b in ESCENARIOS.items()]}
    destino.parent.mkdir(parents=True, exist_ok=True)
    destino.write_text(json.dumps(datos, ensure_ascii=False, indent=1) + '\n', encoding='utf-8', newline='\n')
    for e in datos['escenarios']:
        print(e['escenario'], sorted(e['paquete']), e['paquete']['manifest.json']['schema_version'])


if __name__ == '__main__':
    main()
