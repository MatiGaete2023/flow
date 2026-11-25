"""E-S1 · Oráculo de los productos de Resultados de Flow Python (`descargador/resultados.py`): `export_csv`,
`export_ics`, `export_zip` y `compare_lots`, sobre las carpetas ficticias de `app/tests/datos/paquete.json`.

Además arma una copia modificada del lote de listados (una fila nueva con un nombre que empieza con «=», para la
neutralización de fórmulas del CSV) para comparar lotes. Reloj fijo: `motor.now` (nombres de `publish`) y `DTSTAMP`
del ICS. El ZIP de entrega se vuelca por contenido (nombres, `INDICE.txt`, `MANIFIESTO.json` y huellas), porque
`zipfile` fecha cada entrada. La carpeta temporal se escribe `<BASE>` con separadores `/`.

Uso (desde la raíz): python app/qa/generar_resultados.py [salida.json]   (por omisión app/tests/datos/resultados.json)
"""

import base64
import hashlib
import json
import shutil
import sys
import tempfile
import zipfile
from datetime import datetime, timezone
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'app' / 'qa'), str(RAIZ / 'descargador'), str(RAIZ / 'src'), str(RAIZ / 'tests')]

import generar_lote  # noqa: E402,F401  (reloj fijo de `motor.now`)
import resultados  # noqa: E402
from generar_excel import reconstruir, volcar  # noqa: E402


class _Fija(datetime):
    @classmethod
    def now(cls, tz=None):
        return datetime(2026, 10, 8, 16, 0, 0, tzinfo=timezone.utc)


resultados.datetime = _Fija


def lote_modificado(origen, destino):
    """Copia del lote de listados con la persona de la página 2 reemplazada (una fila nueva y una ausente al comparar),
    con su huella actualizada en la verificación."""
    shutil.copytree(origen, destino)
    v = destino / 'verificacion 001.json'
    datos = json.loads(v.read_text(encoding='utf-8'))
    for item in datos['archivos']:
        if item['pagina'] == 2:
            ruta = destino / item['archivo']
            contenido = ruta.read_bytes().replace(b'Persona Ficticia 2', b'=Persona Nueva')
            ruta.write_bytes(contenido)
            item['sha256'] = hashlib.sha256(contenido).hexdigest()
            item['bytes'] = len(contenido)
    v.write_text(json.dumps(datos), encoding='utf-8')
    return destino


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'resultados.json'
    paquete = {e['escenario']: e for e in json.loads(
        (RAIZ / 'app' / 'tests' / 'datos' / 'paquete.json').read_text(encoding='utf-8'))['escenarios']}
    salida = {'csv': [], 'ics': [], 'zip': [], 'comparacion': []}
    with tempfile.TemporaryDirectory() as tmp:
        # En Windows `tmp` puede venir con el nombre corto (RUNNER~1) y Python resuelve las rutas al largo: se usa el resuelto.
        base = Path(tmp).resolve()
        texto_base = str(base)

        def lote(escenario, indice=0):
            e = paquete[escenario]
            datos = [e['lote'], *e.get('otros_lotes', [])][indice]
            c = base / escenario / 'lotes' / datos['carpeta']
            if not c.exists():
                reconstruir(c, datos['archivos'])
            return c

        modificado = lote_modificado(lote('listados'), base / 'modificado' / 'lotes' / '2026-10-08 13-00-00')
        archivos_modificado = {p.name: base64.b64encode(p.read_bytes()).decode() for p in sorted(modificado.iterdir())}
        casos = [('listados_dos_pestanas', 0), ('calendario_informes', 0), ('calendario_informes', 2), ('carga', 1)]
        for escenario, i in casos:
            carpeta = lote(escenario, i)
            for ruta in resultados.export_csv(resultados.load_lot(carpeta)):
                salida['csv'].append({'escenario': escenario, 'lote': i, 'archivo': ruta.name,
                                      'contenido': base64.b64encode(ruta.read_bytes()).decode()})
        for ruta in resultados.export_csv(resultados.load_lot(modificado)):
            salida['csv'].append({'escenario': 'modificado', 'lote': 0, 'archivo': ruta.name,
                                  'contenido': base64.b64encode(ruta.read_bytes()).decode()})
            ruta.unlink()  # la copia se compara después sin productos dentro
        for escenario, i in (('calendario_informes', 0), ('calendario_informes', 2), ('listados', 0)):
            caso = {'escenario': escenario, 'lote': i}
            try:
                ruta = resultados.export_ics(resultados.load_lot(lote(escenario, i)))
            except Exception as exc:  # noqa: BLE001  (sin fechas: el rechazo es parte del contrato)
                caso['error'] = str(exc)
            else:
                caso.update(archivo=ruta.name, contenido=ruta.read_bytes().decode('utf-8'))
            salida['ics'].append(caso)
        for escenario, i in (('listados_dos_pestanas', 0), ('carga', 0)):
            ruta = resultados.export_zip(resultados.load_lot(lote(escenario, i)))
            with zipfile.ZipFile(ruta) as z:
                entradas = {n: (z.read(n).decode('utf-8') if n in ('INDICE.txt', 'MANIFIESTO.json')
                                else hashlib.sha256(z.read(n)).hexdigest()) for n in z.namelist()}
            salida['zip'].append({'escenario': escenario, 'lote': i, 'archivo': ruta.name, 'orden': list(entradas),
                                  'entradas': entradas})
        anterior = resultados.load_lot(lote('listados'))
        for nombre, nuevo in (('modificado', modificado), ('mismo', lote('listados'))):
            ruta = resultados.compare_lots(anterior, resultados.load_lot(nuevo))
            salida['comparacion'].append({'escenario': nombre, 'archivo': ruta.name, 'libro': volcar(ruta, texto_base)})
            ruta.unlink()
        try:
            resultados.compare_lots(anterior, resultados.load_lot(lote('carga')))
        except Exception as exc:  # noqa: BLE001
            salida['comparacion'].append({'escenario': 'alcances_distintos', 'error': str(exc)})
        salida['modificado'] = {'carpeta': modificado.name, 'archivos': archivos_modificado}
        texto = json.dumps(salida, ensure_ascii=False, indent=1)
        if texto_base in texto or json.dumps(texto_base)[1:-1] in texto:
            raise SystemExit('La carpeta temporal quedó en el oráculo.')
    destino.write_text(texto + '\n', encoding='utf-8', newline='\n')
    for clave in ('csv', 'ics', 'zip', 'comparacion'):
        for x in salida[clave]:
            print(clave, x['escenario'], x.get('lote', ''), x.get('archivo') or x.get('error'))


if __name__ == '__main__':
    main()
