"""E-Q5d / E-S1 · Oráculo de los Excel de Flow Python sobre las carpetas ficticias de `app/tests/datos/paquete.json`.

Reconstruye cada carpeta (lote de listados y lote de bitácoras) en una carpeta temporal y ejecuta:
- `nurus.capture.diaries.export_diaries` (`Bitacoras_RUS.xlsx`), también sobre un lote de bitácoras con textos de
  más de 30.000 unidades UTF-16;
- `nurus.capture.historias.export_historias` (`Historia_SITFA.xlsx`);
- `resultados.export_book` (libro consolidado y «Para CSMP») y su metadato JSON.

Vuelca cada hoja: título, panel congelado, filtro, anchos de columna y celdas con su tipo. La ruta absoluta de la
carpeta temporal se reemplaza por `<BASE>` y la huella del XLSX por `<SHA256>` (no son deterministas); la marca de hora de `publish` y
`fecha_exportacion` se fijan con el reloj de `generar_lote` (`motor.now`).

Uso (desde la raíz): python app/qa/generar_excel.py [salida.json]   (por omisión app/tests/datos/excel.json)
"""

import base64
import json
import sys
import tempfile
from datetime import date, datetime
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'app' / 'qa'), str(RAIZ / 'descargador'), str(RAIZ / 'src'), str(RAIZ / 'tests')]

import generar_lote  # noqa: E402,F401  (reloj fijo de `motor.now`)
from openpyxl import load_workbook  # noqa: E402
from resultados import export_book, load_lot  # noqa: E402
from nurus.capture.diaries import export_diaries  # noqa: E402
from nurus.capture.historias import export_historias  # noqa: E402

BASE = '<BASE>'


def reconstruir(destino, archivos):
    for ruta, b64 in archivos.items():
        p = destino / ruta
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(base64.b64decode(b64))


def sin_base(texto, base):
    """La carpeta temporal como `<BASE>` y sus separadores como `/` (el oráculo es el mismo en Windows y Linux)."""
    if base not in texto:
        return texto
    cabeza, _, cola = texto.partition(base)
    return cabeza + BASE + cola.replace('\\', '/')


def celda(v, base):
    if v is None:
        return None
    if isinstance(v, bool):
        return ['b', v]
    if isinstance(v, (int, float)):
        return ['n', v]
    if isinstance(v, (datetime, date)):
        return ['d', v.isoformat()]
    return ['s', sin_base(str(v), base)]


def volcar(ruta, base):
    libro = load_workbook(ruta)
    hojas = []
    for hoja in libro.worksheets:
        anchos = {k: d.width for k, d in sorted(hoja.column_dimensions.items()) if d.customWidth}
        filas = [[celda(c.value, base) for c in fila] for fila in hoja.iter_rows()]
        while filas and all(v is None for v in filas[-1]):
            filas.pop()
        filas = [f[: max((i + 1 for i, v in enumerate(f) if v is not None), default=0)] for f in filas]
        hojas.append({
            'titulo': hoja.title,
            'congelar': hoja.freeze_panes,
            'filtro': hoja.auto_filter.ref,
            'anchos': anchos,
            'celdas': filas,
        })
    libro.close()
    return {'hojas': hojas}


def escenarios_paquete():
    datos = json.loads((RAIZ / 'app' / 'tests' / 'datos' / 'paquete.json').read_text(encoding='utf-8'))
    return {e['escenario']: e for e in datos['escenarios']}


def bitacoras_largas(carpeta):
    """`test_reutilizacion_local.diaries` con textos de más de 30.000 unidades UTF-16 (emoji = 2 unidades)."""
    from test_reutilizacion_local import diaries

    texto = 'Texto extenso ficticio ' + 'ñ😀abc ' * 4500
    manifiesto = diaries(carpeta, 2, text=texto)
    # Alcance planificado con una consulta sin intentar (fila NO_CONSULTADA), código de extensión de una lectura y una
    # lectura anterior no reenumerada (hoja «Lecturas anteriores»).
    datos = json.loads(manifiesto.read_text(encoding='utf-8'))
    datos['seleccion'] = {'tribunals': ['113'], 'modalities': ['2'], 'tabs': ['Cumplimiento', 'Espera'], 'screen': 'seguimiento'}
    datos['consultas'] = [{'tribunal': '113', 'modality': '2', 'tab': 'Cumplimiento', 'tribunal_nombre': 'Tribunal ficticio',
                           'modalidad_nombre': 'Ambulatorio', 'estado': 'COMPLETA', 'paginas': 1, 'registros': 2, 'error': ''}]
    datos['registros'][1]['diagnostico'] = {'codigo_extension': 'CODIGO_FICTICIO'}
    datos['anteriores_no_reenumeradas'] = [{'tribunal_nombre': 'Tribunal ficticio', 'modalidad_nombre': 'Ambulatorio',
                                            'pestana': 'Cumplimiento', 'rit': 'X-9-2026', 'nombre': 'Persona anterior',
                                            'causa_id': '190', 'ingreso_id': '290', 'estado': 'LEIDA',
                                            'archivo': 'bitacora_anterior.html', 'sha256': '0' * 64}]
    manifiesto.write_text(json.dumps(datos, ensure_ascii=False), encoding='utf-8')
    return manifiesto


def libros(e, base, nombre, csmp_tambien):
    """`export_book` (consolidado y, si se pide, Para CSMP) sobre los lotes reconstruidos de un escenario."""
    carpetas = []
    for lote in [e['lote'], *e.get('otros_lotes', [])]:
        c = base / 'lotes' / lote['carpeta']
        reconstruir(c, lote['archivos'])
        carpetas.append(c)
    salida = []
    for csmp in ((False, True) if csmp_tambien else (False,)):
        destino = base / ('libro-csmp' if csmp else 'libro')
        try:
            ruta = export_book([load_lot(c) for c in carpetas], destino, csmp=csmp)
        except Exception as exc:  # noqa: BLE001  (el rechazo es parte del contrato)
            salida.append({'escenario': nombre, 'csmp': csmp, 'carpetas': [c.name for c in carpetas], 'error': str(exc)})
            continue
        metadato = json.loads(ruta.with_suffix('.json').read_text(encoding='utf-8'))
        metadato['sha256'] = '<SHA256>'  # huella del XLSX: openpyxl fecha el ZIP al guardar (Rust la comprueba sobre su archivo)
        for f in metadato['fuentes']:
            f['lote'] = sin_base(f['lote'], str(base))
        salida.append({
            'escenario': nombre, 'csmp': csmp, 'carpetas': [c.name for c in carpetas], 'archivo': ruta.name,
            'libro': volcar(ruta, str(base)), 'metadato': metadato,
        })
    return salida


def lotes_propios(base):
    """Lotes de `test_resultados.fixture` para los casos que no traen los de `paquete.json`:
    - `esquemas`: dos lotes de Cumplimiento con cabeceras distintas (hoja CUMPLIMIENTO_2) y uno sin programa (filas
      con requeridos vacíos);
    - `sin_resultados`: una consulta SIN_RESULTADOS con PDF junto a la validada;
    - `parcial`: lote INCOMPLETA abierto a sabiendas, con su consulta interrumpida sin archivos."""
    import generar_paquete as gp
    from test_resultados import fixture

    def lote(nombre, **kw):
        c = fixture(base / 'lotes' / nombre, mode='Cumplimiento', names=('Persona uno', 'Persona dos'), **kw)
        return gp.lote_fijo(c)

    uno = lote('esquema-a', program='PRM Centro')
    dos = lote('esquema-b', program='')
    # Columna extra en el segundo esquema: otra hoja de la misma familia.
    from openpyxl import load_workbook
    import io
    import hashlib
    libro = load_workbook(io.BytesIO((dos / 'pagina.xls').read_bytes()))
    hoja = libro.active
    hoja.cell(1, hoja.max_column + 1, 'OBSERVACION')
    salida = io.BytesIO()
    libro.save(salida)
    (dos / 'pagina.xls').write_bytes(salida.getvalue())
    gp.lote_fijo(dos)
    vacia = lote('sin-resultados', program='PRM Centro')
    v = json.loads((vacia / 'resumen.json').read_text(encoding='utf-8'))
    seleccion = dict(v['plan'][0], tab='Espera')
    pdf = b'%PDF-1.4 ficticio'
    (vacia / 'evidencia.pdf').write_bytes(pdf)
    evidencia = {'archivo': 'evidencia.pdf', 'sha256': hashlib.sha256(pdf).hexdigest(), 'bytes': len(pdf)}
    (vacia / 'verificacion 002.json').write_text(json.dumps({
        'estado': 'SIN_RESULTADOS', 'consulta': 'Consulta ficticia vacía', 'seleccion': seleccion, 'archivos': [],
        'paginas_esperadas': 0, 'evidencia': evidencia}), encoding='utf-8')
    v['consultas'].append({'consulta': 'Consulta ficticia vacía', 'estado': 'SIN_RESULTADOS', 'seleccion': seleccion,
                           'verificacion': 'verificacion 002.json'})
    v['plan'].append(seleccion)
    v['consultas_esperadas'] = 2
    (vacia / 'resumen.json').write_text(json.dumps(v), encoding='utf-8')
    parcial = lote('parcial', program='PRM Centro')
    v = json.loads((parcial / 'resumen.json').read_text(encoding='utf-8'))
    v['estado'] = 'INCOMPLETA'
    (parcial / 'resumen.json').write_text(json.dumps(v), encoding='utf-8')
    (parcial / 'verificacion 002.json').write_text(json.dumps({
        'estado': 'INCOMPLETA', 'consulta': 'Consulta interrumpida', 'seleccion': v['plan'][0], 'archivos': []}),
        encoding='utf-8')
    return [('esquemas', [uno, dos], False), ('sin_resultados', [vacia], False), ('parcial', [parcial], True)]


def libros_propios(base):
    salida = []
    base = base / 'propios'
    for nombre, carpetas, parcial in lotes_propios(base):
        archivos = [{'carpeta': c.name, 'archivos': {p.name: base64.b64encode(p.read_bytes()).decode()
                                                    for p in sorted(c.iterdir())}} for c in carpetas]
        for csmp in (False, True):
            destino = base / f'{nombre}-{csmp}'
            caso = {'escenario': nombre, 'csmp': csmp, 'parcial': parcial, 'lotes': archivos,
                    'carpetas': [c.name for c in carpetas]}
            try:
                ruta = export_book([load_lot(c, partial=parcial) for c in carpetas], destino, csmp=csmp)
            except Exception as exc:  # noqa: BLE001
                caso['error'] = str(exc)
            else:
                metadato = json.loads(ruta.with_suffix('.json').read_text(encoding='utf-8'))
                metadato['sha256'] = '<SHA256>'
                for f in metadato['fuentes']:
                    f['lote'] = sin_base(f['lote'], str(base))
                caso.update(archivo=ruta.name, libro=volcar(ruta, str(base)), metadato=metadato)
            salida.append(caso)
    return salida


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'excel.json'
    paquete = escenarios_paquete()
    datos = {'bitacoras': [], 'historias': [], 'libros': []}
    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp)
        contexto = paquete['contexto']
        carpeta = base / 'bitacoras' / contexto['bitacoras']['carpeta']
        reconstruir(carpeta, contexto['bitacoras']['archivos'])
        datos['bitacoras'].append({'escenario': 'contexto', 'excel': volcar(export_diaries(carpeta / 'bitacoras.json', base / 'b1.xlsx'), tmp)})
        datos['historias'].append({'escenario': 'contexto', 'excel': volcar(
            export_historias(carpeta / 'historia' / 'historias.json', base / 'h1.xlsx'), tmp)})
        largas = bitacoras_largas(base / 'largas')
        datos['bitacoras'].append({
            'escenario': 'textos_largos',
            'archivos': {p.name: base64.b64encode(p.read_bytes()).decode() for p in sorted(largas.parent.iterdir())},
            'excel': volcar(export_diaries(largas, base / 'b2.xlsx'), tmp),
        })
        for nombre, csmp in (('listados', True), ('listados_dos_pestanas', True), ('calendario_informes', True), ('carga', True)):
            datos['libros'].extend(libros(paquete[nombre], base / nombre, nombre, csmp))
        datos['libros'].extend(libros_propios(base))
    destino.write_text(json.dumps(datos, ensure_ascii=False, indent=1) + '\n', encoding='utf-8', newline='\n')
    for clave, lista in datos.items():
        for x in lista:
            print(clave, x['escenario'], x.get('csmp', ''), x.get('error') or [h['titulo'] for h in (x.get('excel') or x.get('libro'))['hojas']])


if __name__ == '__main__':
    main()
