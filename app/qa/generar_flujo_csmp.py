"""E-Q11: contraste sintético de plan, recuperación y libro de flujo_csmp."""
import hashlib
import io
import base64
import json
import re
import sys
import tempfile
import threading
import zipfile
from datetime import date, timedelta
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'app' / 'qa'), str(RAIZ / 'descargador'), str(RAIZ / 'src'), str(RAIZ / 'tests')]
import generar_lote
import motor
from flujo_csmp import JointRunner, plan
from test_recuperacion_flujo import FakeBridge, make_lot
from openpyxl import load_workbook


def run(destino):
    casos = []
    for nombre in ('espera', 'cumplimiento', 'modo_invalido', 'sin_tribunales', 'dias_invalidos',
                   'retomar_principal', 'retomar_informes', 'control_danado'):
        reloj = [0]
        def ahora():
            reloj[0] += 1
            return generar_lote.AHORA + timedelta(seconds=reloj[0])
        motor.now = ahora
        with tempfile.TemporaryDirectory(prefix='flujo-csmp-') as tmp:
            raiz = Path(tmp).resolve()
            llamadas = []
            falla = [nombre in ('retomar_principal', 'retomar_informes')]
            class Runner:
                def __init__(self, bridge, catalog, emit, cancel):
                    self.catalog, self.emit = catalog, emit
                def run(self, batch, destination, plan_override=None):
                    seleccion = batch.plan(self.catalog) if plan_override is None else plan_override
                    llamadas.append(batch.screen)
                    carpeta = Path(destination) / str(len(llamadas))
                    self.emit('folder', str(carpeta))
                    cortar = falla[0] and ((nombre == 'retomar_principal' and batch.screen == 'seguimiento') or
                                           (nombre == 'retomar_informes' and batch.screen == 'calendario_informes'))
                    make_lot(carpeta, seleccion[:1] if cortar else seleccion, complete=not cortar, planned=seleccion)
                    if batch.tabs == ('Espera',):
                        libro = load_workbook(io.BytesIO((carpeta / 'pagina-1.xls').read_bytes()))
                        libro.active.cell(1, 6, 'T ESPERA')
                        libro.save(carpeta / 'pagina-1.xls'); libro.close()
                        meta = carpeta / 'verificacion 001.json'
                        datos = json.loads(meta.read_text(encoding='utf-8'))
                        bruto = (carpeta / 'pagina-1.xls').read_bytes()
                        datos['archivos'][0].update(bytes=len(bruto), sha256=hashlib.sha256(bruto).hexdigest())
                        meta.write_text(json.dumps(datos), encoding='utf-8')
                    # Normaliza cada XLSX sintético para que los archivos incrustados
                    # no dependan del reloj, zlib ni de la plataforma.
                    for original in sorted(carpeta.glob('pagina-*.xls')):
                        libro = load_workbook(io.BytesIO(original.read_bytes()))
                        libro.properties.created = libro.properties.modified = generar_lote.AHORA.replace(tzinfo=None)
                        intermedio = io.BytesIO(); libro.save(intermedio); libro.close()
                        salida = io.BytesIO()
                        with zipfile.ZipFile(io.BytesIO(intermedio.getvalue())) as fuente, zipfile.ZipFile(salida, 'w') as destino_zip:
                            for entrada in fuente.infolist():
                                info = zipfile.ZipInfo(entrada.filename, (2020, 1, 1, 0, 0, 0))
                                info.compress_type = zipfile.ZIP_STORED
                                info.create_system = 3
                                info.external_attr = 0o600 << 16
                                contenido = fuente.read(entrada.filename)
                                if entrada.filename == 'docProps/core.xml':
                                    contenido = re.sub(rb'(<dcterms:modified\b[^>]*>).*?(</dcterms:modified>)', rb'\g<1>2026-10-08T13:00:00Z\g<2>', contenido)
                                destino_zip.writestr(info, contenido)
                        bruto = salida.getvalue(); original.write_bytes(bruto)
                        for meta in sorted(carpeta.glob('verificacion *.json')):
                            datos = json.loads(meta.read_text(encoding='utf-8'))
                            for item in datos.get('archivos', []):
                                if item.get('archivo') == original.name:
                                    item.update(bytes=len(bruto), sha256=hashlib.sha256(bruto).hexdigest())
                            meta.write_text(json.dumps(datos, ensure_ascii=False), encoding='utf-8')
                    if cortar:
                        falla[0] = False
                        raise motor.PocError('Corte ficticio')
                    return seleccion
            class Bridge(FakeBridge):
                def call(self, command, payload):
                    catalogo = super().call(command, payload)
                    catalogo['pestanas'] = ['Espera', 'Cumplimiento']
                    return catalogo
            modo = 'Espera' if nombre == 'espera' else 'Carga' if nombre == 'modo_invalido' else 'Cumplimiento'
            tribunales = () if nombre == 'sin_tribunales' else ('113', '119') if nombre == 'retomar_principal' else ('113',)
            dias = 59 if nombre == 'dias_invalidos' else 60
            fecha = date(2026, 10, 2)
            error = None
            libro = None
            estado = None
            try:
                if nombre == 'control_danado':
                    carpeta = raiz / 'danado'; carpeta.mkdir(); (carpeta / 'flujo.json').write_text('{', encoding='utf-8')
                    JointRunner(Bridge(), lambda *e: None, threading.Event(), runner_factory=Runner).run(tribunales, ('2',), modo, raiz, today=fecha, resume=carpeta)
                else:
                    corredor = JointRunner(Bridge(), lambda *e: None, threading.Event(), runner_factory=Runner)
                    try:
                        libro = corredor.run(tribunales, ('2',), modo, raiz, days=dias, today=fecha)
                    except motor.PocError:
                        if nombre not in ('retomar_principal', 'retomar_informes'): raise
                        carpeta = next(raiz.glob('Flujo CSMP *'))
                        libro = corredor.run(tribunales, ('2',), modo, raiz, days=dias, today=fecha, resume=carpeta)
            except (motor.PocError, ValueError, OSError) as exc:
                error = str(exc)
            fuentes = None
            if libro:
                datos = json.loads((libro.parent / 'flujo.json').read_text(encoding='utf-8'))
                estado = {'estado': datos['estado'], 'fases': [{'clave': k, 'estado': v['estado'], 'consultas': v.get('consultas'),
                          'carpeta': str(v.get('carpeta', '')).replace('\\', '/')} for k, v in datos['fases'].items()]}
                xlsx = load_workbook(libro, data_only=True)
                anchos = {'ESPERA': 8, 'CUMPLIMIENTO': 8, 'INFORMES': 6, 'SITFA_COBERTURA': 6}
                hojas = {s.title: [list(row[:anchos[s.title]]) for row in s.iter_rows(values_only=True)] for s in xlsx if s.title in anchos}
                xlsx.close()
                fuentes = []
                for fase in datos['fases'].values():
                    carpeta = libro.parent / fase['carpeta']
                    fuentes.append({p.relative_to(carpeta).as_posix(): base64.b64encode(p.read_bytes()).decode('ascii')
                                    for p in sorted(carpeta.rglob('*')) if p.is_file()})
            else:
                hojas = None
            casos.append({'caso': nombre, 'modo': modo, 'tribunales': list(tribunales), 'modalidades': ['2'], 'dias': dias,
                          'fecha': fecha.isoformat(), 'error': error, 'llamadas': llamadas, 'estado': estado, 'hojas': hojas, 'fuentes': fuentes})
    Path(destino).write_bytes((json.dumps({'version': 1, 'casos': casos}, ensure_ascii=False, indent=2, default=str) + '\n').encode('utf-8'))
    print(f'flujo_csmp: {len(casos)} casos')


if __name__ == '__main__':
    run(sys.argv[1] if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'flujo_csmp.json')
