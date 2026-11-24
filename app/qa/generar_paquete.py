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
from descargador import package_current_lot  # noqa: E402
from lectura_bitacoras import DiaryRunner  # noqa: E402
from lectura_historia import LectorHistoria  # noqa: E402
import lectura_historia  # noqa: E402
import generar_bitacoras as gb  # noqa: E402  (fecha de historia fija en Santiago)

lectura_historia.datetime = gb._FechaSantiago


# Trámites anteriores a la ventana (lectura 08-10-2026, 4 meses): dos «Orden de búsqueda» (se conserva la más reciente),
# una «Designa abogado», una nomenclatura fuera de CONTEXTO (no viaja) y una fila dentro de la ventana.
FILAS_CONTEXTO = ''.join(
    f'<tr><td>{folio}</td><td></td><td></td><td></td><td>Cumplimiento</td><td>Resoluci&oacute;n</td><td>{desc}</td>'
    f'<td>Ref. ficticia {folio}</td><td>{fecha}</td></tr>'
    for folio, desc, fecha in (
        ('8001', 'Orden de b&uacute;squeda', '01/02/2026'),
        ('8002', 'Orden de b&uacute;squeda', '01/03/2026'),
        ('8003', 'Otro tr&aacute;mite ficticio', '01/03/2026'),
        ('8004', 'Designa abogado', '15/05/2026'),
        ('8005', 'Resoluci&oacute;n ficticia', '01/09/2026'),
        # Texto sobre 30.000 unidades UTF-16 (emoji = 2): Historia_SITFA.xlsx lo parte en columnas de continuación.
        # Folio menor con fecha más nueva: el orden por fecha de Historia_SITFA.xlsx no coincide con el de folio.
        ('8007', 'Resoluci&oacute;n tard&iacute;a ficticia', '01/12/2099'),
        ('8006', 'Resoluci&oacute;n extensa ficticia ' + '&ntilde;&#128512;abc ' * 4500, '02/09/2026'),
    )
).encode()


class PuenteContexto(gl.ConVinculos):
    """Listado con vínculos (`generar_lote`) más las ventanas de contexto ficticias de las pruebas de Python: cada
    página es otra persona (histórico leído), la historia trae trámites de contexto anteriores a la ventana y la
    bitácora del ingreso 202 tiene una entrada a medianoche (precisión DIA)."""

    def call(self, command, payload=None, timeout=75):
        import hashlib
        from test_calendario_lectura import calendar_page
        from test_lectura_bitacoras import popup
        from test_lectura_historia import VENTANA

        if command in ('diary_open', 'calendar_open', 'history_open', 'historico_open'):
            if command == 'diary_open':
                datos, eco = popup(payload['params']).encode(), {'params': payload['params']}
                if payload['params'].get('ID_Ingreso') == '202':
                    datos = datos.replace(b'12/08/2026 10:13', b'12/08/2026 00:00')
            elif command == 'calendar_open':
                a = payload['args']
                datos, eco = calendar_page(a[0], a[2], a[10]).encode(), {'args': a}
            elif command == 'history_open':
                p = payload['params']
                datos = VENTANA.replace(b'C-99999-2099', p['rit'].encode()).replace(
                    b'Juzgado Canario de Prueba', CATALOG['tribunales'][p['tribunal']].encode('cp1252'))
                # Antes de la última fila (la incompleta de 4 celdas, que SITFA solo entrega al final).
                fin = datos.rindex(b'<tr', 0, datos.index(b'</tbody>', datos.index(b'<th>Folio</th>')))
                datos = datos[:fin] + FILAS_CONTEXTO + datos[fin:]
                eco = {'params': p}
            else:
                datos, eco = gb.HISTORIAL, {'params': payload['params']}
            return {'status': 200, 'type': 'text/html', 'body': base64.b64encode(datos).decode(),
                    'digest': hashlib.sha256(datos).hexdigest(), 'startedAt': '2026-10-08T12:00:00Z', **eco}
        resultado = super().call(command, payload, timeout)
        if command == 'search' and self.selection is not None:
            fuente = base64.b64decode(resultado['body']).replace(b"','300','", f"','{300 + self.number}','".encode())
            resultado['body'] = base64.b64encode(fuente).decode()
        return resultado


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


def escenario_contexto(nombre, lote):
    """Lote de listados + lote de bitácoras (calendario, historia e histórico) y `package_current_lot`."""
    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp)
        BatchRunner(PuenteContexto(), CATALOG, lambda *a: None, threading.Event(), pause=0).run(lote, base / 'lotes')
        carpeta = next((base / 'lotes').iterdir())
        puente = PuenteContexto()
        historia = LectorHistoria(puente, lambda *a: None, threading.Event(), pause=0)
        r = DiaryRunner(puente, CATALOG, lambda *a: None, threading.Event(), pause=0, calendars=True, historico=True,
                        historia=historia).run(lote, base / 'bitacoras')
        bitacoras = Path(r['folder'])
        destino = base / 'paquete.zip'
        resultado = package_current_lot(carpeta, bitacoras / 'bitacoras.json', destino,
                                        historia=bitacoras / 'historia' / 'historias.json', historico=bitacoras)
        archivos = {k: v for k, v in carpeta_json(bitacoras).items() if not k.endswith('.xlsx')}
        return {
            'escenario': nombre,
            'lote': {'carpeta': carpeta.name, 'archivos': carpeta_json(carpeta)},
            'bitacoras': {'carpeta': bitacoras.name, 'archivos': archivos},
            'resultado': {k: v for k, v in resultado.items() if k != 'path'},
            'paquete': paquete_json(destino),
        }


def xlsx_fijo(datos):
    """openpyxl fija `created`/`modified` y la fecha de cada entrada al guardar: se fijan para un fixture reproducible."""
    import io
    import re

    entrada = zipfile.ZipFile(io.BytesIO(datos))
    final = io.BytesIO()
    with zipfile.ZipFile(final, 'w', zipfile.ZIP_DEFLATED) as z:
        for info in entrada.infolist():
            contenido = entrada.read(info.filename)
            if info.filename == 'docProps/core.xml':
                contenido = re.sub(rb'(<dcterms:(?:created|modified)[^>]*>)[^<]*', rb'\g<1>2026-10-08T00:00:00Z', contenido)
            nuevo = zipfile.ZipInfo(info.filename, date_time=(2026, 10, 8, 0, 0, 0))
            nuevo.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(nuevo, contenido)
    return final.getvalue()


def lote_fijo(carpeta, **seleccion):
    """Normaliza los XLS de un lote de `test_resultados.fixture` (huella en su verificación) y ajusta la selección."""
    import hashlib

    v = carpeta / 'verificacion 001.json'
    datos = json.loads(v.read_text(encoding='utf-8'))
    for item in datos['archivos']:
        contenido = xlsx_fijo((carpeta / item['archivo']).read_bytes())
        (carpeta / item['archivo']).write_bytes(contenido)
        item.update(sha256=hashlib.sha256(contenido).hexdigest(), bytes=len(contenido))
    datos['seleccion'].update(seleccion)
    v.write_text(json.dumps(datos), encoding='utf-8')
    r = carpeta / 'resumen.json'
    resumen = json.loads(r.read_text(encoding='utf-8'))
    for s in (*resumen['plan'], *(q['seleccion'] for q in resumen['consultas'])):
        s.update(seleccion)
    r.write_text(json.dumps(resumen), encoding='utf-8')
    return carpeta


def escenario_informes(nombre):
    """Seguimiento con vínculos + calendario de informes sin ids remotos: «Persona uno» se asocia a su único ingreso
    (tribunal, RIT, nombre y programa); «Persona tres» no tiene ingreso, «Persona dos» empata entre dos ingresos:
    ambas quedan NO_ASOCIADO."""
    from test_resultados import fixture

    # Como `test_reutilizacion_local.bound_lot`; «Persona dos» tiene además otro ingreso en un lote de Espera (empate).
    def vinculado(carpeta, modo, nombres, primero):
        carpeta = fixture(carpeta, mode=modo, names=nombres, program='PRM Centro')
        v = carpeta / 'verificacion 001.json'
        datos = json.loads(v.read_text())
        datos['archivos'][0]['vinculos_ingreso'] = [
            {'tribunal_codigo': '113', 'rit': 'X-1-2026', 'rut': '00111111-1', 'nombre': n, 'programa': 'PRM Centro',
             'causa_id': str(primero - 101 + i), 'ingreso_id': str(primero + i), 'etapa': '3', 'antiguo': '0'}
            for i, n in enumerate(nombres)
        ]
        v.write_text(json.dumps(datos), encoding='utf-8')
        return lote_fijo(carpeta)

    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp)
        principal = vinculado(base / 'principal', 'Cumplimiento', ('Persona uno', 'Persona dos'), 201)
        espera = vinculado(base / 'espera', 'Espera', ('Persona dos',), 299)
        calendario = lote_fijo(fixture(base / 'calendario', mode='Informes', names=('Persona uno', 'Persona tres', 'Persona dos'),
                                       program='PRM Centro'),
                               screen='calendario_informes', tab='', report=None, month='10', year='2026')
        destino = base / 'paquete.zip'
        _, nota = export_capture_with_note(load_sources([principal, espera, calendario]), destino)
        return {
            'escenario': nombre,
            'lote': {'carpeta': principal.name, 'archivos': carpeta_json(principal)},
            'otros_lotes': [{'carpeta': c.name, 'archivos': carpeta_json(c)} for c in (espera, calendario)],
            'nota': nota,
            'paquete': paquete_json(destino),
        }


def lote_carga(carpeta, formato):
    """Lote de Carga ficticio (tribunal 113, octubre 2026): HTML tabular como lo entrega SITFA (con el título fijo
    «Concepción») o XLSX con celdas de fecha y número. Dos filas del mismo RIT: Carga trae una fila por trámite."""
    import hashlib
    from carga import HEADERS
    from openpyxl import Workbook

    carpeta.mkdir(parents=True)
    filas = []
    for rit, tramite, dias, firma in (('X-1-2026', 'Resolución ficticia', 3, (2026, 10, 2)),
                                      ('X-1-2026', 'Decreto ficticio', 12, (2026, 10, 9)),
                                      ('Y-2-2026', 'Acta ficticia, con coma', 0, (2026, 10, 15))):
        fila = [''] * 23
        fila[0], fila[1], fila[2], fila[3], fila[4], fila[5] = rit, 'Jgdo. L. y G. de Laja', 'Funcionario Ficticio', 'Juez Ficticio', tramite, 'NO'
        fila[6], fila[8], fila[9], fila[21], fila[22] = 'Documento ficticio', dias, 'Cargo ficticio', firma, '09:30'
        filas.append(fila)
    if formato == 'html':
        celda = lambda v: '%02d/%02d/%04d' % (v[2], v[1], v[0]) if isinstance(v, tuple) else str(v)
        cuerpo = ''.join('<tr>' + ''.join(f'<td>{celda(v)}</td>' for v in f) + '</tr>' for f in filas)
        contenido = ('<html><meta charset="utf-8"><table><tr><td>Concepción</td></tr>'
                     '<tr>' + ''.join(f'<td>{h}</td>' for h in HEADERS) + '</tr>' + cuerpo + '</table></html>').encode()
    else:
        from datetime import datetime
        libro = Workbook()
        hoja = libro.active
        hoja.append(list(HEADERS))
        for f in filas:
            hoja.append([datetime(*v) if isinstance(v, tuple) else v for v in f])
        import io
        salida = io.BytesIO()
        libro.save(salida)
        contenido = xlsx_fijo(salida.getvalue())
    (carpeta / 'pagina.xls').write_bytes(contenido)
    seleccion = {'screen': 'carga', 'tribunal': '113', 'tribunal_nombre': 'Jgdo. L. y G. de Laja', 'modality': '', 'tab': '',
                 'report': None, 'state': None, 'month': None, 'year': None, 'start': '01/10/2026', 'end': '31/10/2026',
                 'fecha_descarga': '2026-10-31T18:00:00'}
    item = {'archivo': 'pagina.xls', 'pagina': 1, 'registros': len(filas), 'sha256': hashlib.sha256(contenido).hexdigest(),
            'bytes': len(contenido), 'estado': 'OK'}
    (carpeta / 'verificacion 001.json').write_text(json.dumps(
        {'estado': 'VALIDADA', 'consulta': 'Carga ficticia', 'seleccion': seleccion, 'archivos': [item],
         'paginas_esperadas': 1, 'paginas_verificadas': 1}), encoding='utf-8')
    consulta = {'consulta': 'Carga ficticia', 'estado': 'VALIDADA', 'seleccion': seleccion, 'verificacion': 'verificacion 001.json'}
    (carpeta / 'resumen.json').write_text(json.dumps(
        {'estado': 'VALIDADA', 'version_contrato': 1, 'consultas_esperadas': 1, 'consultas_completadas': 1,
         'consultas': [consulta], 'plan': [seleccion], 'otros_filtros': 'todos los registros', 'filtro_fecha': None,
         'fecha_hora': seleccion['fecha_descarga']}), encoding='utf-8')
    return carpeta


def escenario_carga(nombre):
    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp)
        html, xlsx = lote_carga(base / 'carga-html', 'html'), lote_carga(base / 'carga-xlsx', 'xlsx')
        destino = base / 'paquete.zip'
        _, nota = export_capture_with_note(load_sources([html, xlsx]), destino)
        return {
            'escenario': nombre,
            'lote': {'carpeta': html.name, 'archivos': carpeta_json(html)},
            'otros_lotes': [{'carpeta': xlsx.name, 'archivos': carpeta_json(xlsx)}],
            'nota': nota,
            'paquete': paquete_json(destino),
        }


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'paquete.json'
    escenarios = [escenario(n, b) for n, b in ESCENARIOS.items()]
    escenarios.append(escenario_contexto('contexto', Batch(('49',), ('2',), ('Cumplimiento',))))
    escenarios.append(escenario_informes('calendario_informes'))
    escenarios.append(escenario_carga('carga'))
    datos = {'catalogo': CATALOG, 'escenarios': escenarios}
    destino.parent.mkdir(parents=True, exist_ok=True)
    destino.write_text(json.dumps(datos, ensure_ascii=False, indent=1) + '\n', encoding='utf-8', newline='\n')
    for e in datos['escenarios']:
        print(e['escenario'], sorted(e['paquete']), e['paquete']['manifest.json']['schema_version'])


if __name__ == '__main__':
    main()
