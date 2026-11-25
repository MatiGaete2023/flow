"""E-Q11: contraste sintético de plan, recuperación y libro de flujo_csmp."""
import hashlib
import io
import json
import sys
import tempfile
import threading
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
                error = 'JSON inválido' if nombre == 'control_danado' else str(exc)
            if libro:
                datos = json.loads((libro.parent / 'flujo.json').read_text(encoding='utf-8'))
                estado = {'estado': datos['estado'], 'fases': [{'clave': k, 'estado': v['estado'], 'consultas': v.get('consultas')} for k, v in datos['fases'].items()]}
                xlsx = load_workbook(libro, data_only=True)
                hojas = {s.title: [list(row[:6]) for row in s.iter_rows(values_only=True)] for s in xlsx if s.title in ('ESPERA', 'CUMPLIMIENTO', 'INFORMES', 'SITFA_COBERTURA')}
                xlsx.close()
            else:
                hojas = None
            casos.append({'caso': nombre, 'modo': modo, 'tribunales': list(tribunales), 'modalidades': ['2'], 'dias': dias,
                          'fecha': fecha.isoformat(), 'error': error, 'llamadas': llamadas, 'estado': estado, 'hojas': hojas})
    Path(destino).write_bytes((json.dumps({'version': 1, 'casos': casos}, ensure_ascii=False, indent=2, default=str) + '\n').encode('utf-8'))
    print(f'flujo_csmp: {len(casos)} casos')


if __name__ == '__main__':
    run(sys.argv[1] if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'flujo_csmp.json')
