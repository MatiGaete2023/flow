"""Retomar un paquete cortado: reutiliza consultas validadas y bitácoras leídas, y rehace el ZIP."""

import json
import threading
from datetime import date
from pathlib import Path

import pytest
from nurus.capture import read_capture

import flujo_paquete
import lectura_bitacoras as lb
from lotes import Batch
from motor import PocError
from test_lectura_bitacoras import Bridge, run as leer
from test_lotes import CATALOG
from test_reutilizacion_local import bound_lot, diaries

BATCH = Batch(('49',), ('2',), ('Cumplimiento',))


def fakes(monkeypatch, llamadas, *, parcial, meses_con_falla=()):
    class Consultas:
        def __init__(self, bridge, catalog, emit, gate, pause, evidencia=True):
            self.emit = emit

        def run(self, batch, destination):
            llamadas.append(('consultas', batch))
            intento = len(tuple(destination.glob('consulta-*')))
            if batch.screen == 'calendario_informes' and batch.month in meses_con_falla:
                raise PocError('El mes ficticio no respondió.')
            modo = 'Informes' if batch.screen == 'calendario_informes' else 'Cumplimiento'
            lot = bound_lot(destination / f'consulta-{intento}', mode=modo)
            self.emit('folder', str(lot.folder))

    class Bitacoras:
        def __init__(self, bridge, catalog, emit, gate, pause, reuse_base=None, calendars=False, historico=False):
            self.emit = emit
            self.calendars = calendars
            self.historico = historico

        def run(self, batch, destination, resume=None):
            llamadas.append(('bitacoras', batch, resume, self.calendars, self.historico))
            manifest = Path(resume) / 'bitacoras.json' if resume else diaries(destination / 'bitacoras', 2)
            data = json.loads(manifest.read_text(encoding='utf-8'))
            data['seleccion'] = {
                'tribunals': list(batch.tribunals),
                'modalities': list(batch.modalities),
                'tabs': list(batch.tabs),
                'report': batch.report,
                'dates': None,
                'keep_filters': False,
                'screen': 'seguimiento',
                'state': '1',
                'month': None,
                'year': None,
            }
            manifest.write_text(json.dumps(data), encoding='utf-8')
            self.emit('folder', str(manifest.parent))
            return {
                'manifest': str(manifest),
                'estado': 'INCOMPLETA' if parcial[0] else 'COMPLETA',
                'fallidas': int(parcial[0]),
                'consultas_fallidas': 0,
            }

    monkeypatch.setattr(flujo_paquete, 'BatchRunner', Consultas)
    monkeypatch.setattr(flujo_paquete, 'DiaryRunner', Bitacoras)


def test_retomar_reutiliza_consultas_y_bitacoras_y_rehace_el_zip(monkeypatch, tmp_path):
    llamadas, parcial = [], [True]
    fakes(monkeypatch, llamadas, parcial=parcial)
    primero = flujo_paquete.run(None, CATALOG, lambda *e: None, threading.Event(), BATCH, tmp_path)
    carpeta = Path(primero['path']).parent
    assert primero['partial']
    assert json.loads((carpeta / 'respaldo/proceso.json').read_text())['estado'] == 'PARCIAL'

    parcial[0] = False
    llamadas.clear()
    segundo = flujo_paquete.retomar(None, CATALOG, lambda *e: None, threading.Event(), carpeta)
    assert not segundo['partial'] and Path(segundo['path']) == carpeta / 'Captura_CSMP.zip'
    # Las consultas validadas no se repiten; las bitácoras continúan en su mismo lote.
    assert [c[0] for c in llamadas] == ['bitacoras']
    assert llamadas[0][1] == BATCH and Path(llamadas[0][2]) == Path(primero['diary_manifest']).parent
    assert len(list((carpeta / 'respaldo').glob('Captura_CSMP anterior *.zip'))) == 1
    proceso = json.loads((carpeta / 'respaldo/proceso.json').read_text())
    assert proceso['estado'] == 'COMPLETO' and proceso['reintentos'] == 1
    with pytest.raises(PocError, match='completo'):
        flujo_paquete.retomar(None, CATALOG, lambda *e: None, threading.Event(), carpeta)


def test_retomar_rechaza_una_carpeta_que_no_es_paquete(tmp_path):
    with pytest.raises(PocError, match='Paquete CSMP'):
        flujo_paquete.retomar(None, CATALOG, lambda *e: None, threading.Event(), tmp_path)


def test_retomar_repite_consulta_si_un_original_perdio_su_huella(monkeypatch, tmp_path):
    llamadas, parcial = [], [True]
    fakes(monkeypatch, llamadas, parcial=parcial)
    args = (None, CATALOG, lambda *e: None, threading.Event())
    primero = flujo_paquete.run(*args, BATCH, tmp_path)
    carpeta = Path(primero['path']).parent
    danado = next(Path(primero['sheets_folder']).glob('*.xls'))
    danado.write_bytes(b'original ficticio que cambio')

    parcial[0] = False
    llamadas.clear()
    retomado = flujo_paquete.retomar(*args, carpeta)
    assert [c[0] for c in llamadas] == ['consultas', 'bitacoras']
    assert Path(retomado['sheets_folder']) != danado.parent
    assert danado.read_bytes() == b'original ficticio que cambio'
    assert not retomado['partial'] and read_capture(retomado['path']).rows('registros')


@pytest.mark.parametrize(
    ('falla_noviembre', 'danar_octubre', 'meses_repetidos'),
    [(True, False, ['11']), (True, True, ['10', '11']), (False, False, [])],
)
def test_retomar_calendario_reintenta_solo_meses_fallidos_o_danados(
    monkeypatch, tmp_path, falla_noviembre, danar_octubre, meses_repetidos
):
    from test_flujo_paquete import PuenteCatalogo

    llamadas, fallas, parcial = [], {'11'} if falla_noviembre else set(), [not falla_noviembre]
    fakes(monkeypatch, llamadas, parcial=parcial, meses_con_falla=fallas)
    puente = PuenteCatalogo()
    args = (puente, CATALOG, lambda *e: None, threading.Event())
    primero = flujo_paquete.run(*args, BATCH, tmp_path, False, informes=True, today=date(2026, 10, 14))
    carpeta = Path(primero['path']).parent
    assert primero['partial']
    assert [c[1].month for c in llamadas if c[0] == 'consultas'] == [None, '10', '11', '12']
    proceso = json.loads((carpeta / 'respaldo/proceso.json').read_text())
    if danar_octubre:
        octubre = carpeta / proceso['carpetas']['informes-2026-10']
        next(octubre.glob('*.xls')).unlink()

    parcial[0] = False
    fallas.clear()
    llamadas.clear()
    puente.llamadas.clear()
    retomado = flujo_paquete.retomar(*args, carpeta, today=date(2026, 12, 20))
    assert [c[1].month for c in llamadas if c[0] == 'consultas'] == meses_repetidos
    assert bool(puente.llamadas) == bool(meses_repetidos)
    assert not retomado['partial']
    comprobadas = [q for q in read_capture(retomado['path']).rows('consultas') if q['screen'] != 'bitacoras']
    assert len(comprobadas) == 4


def test_retomar_conserva_opciones_originales_del_paquete(monkeypatch, tmp_path):
    import descargador

    llamadas, parcial, exportaciones = [], [True], []
    fakes(monkeypatch, llamadas, parcial=parcial)
    real = descargador.package_current_lot

    def exportar(hojas, bitacoras, ruta, consolidated, **opciones):
        exportaciones.append((consolidated, opciones))
        return real(hojas, bitacoras, ruta, consolidated, **opciones)

    monkeypatch.setattr(descargador, 'package_current_lot', exportar)
    args = (None, CATALOG, lambda *e: None, threading.Event())
    original = flujo_paquete.run(*args, BATCH, tmp_path, False, calendars=True, historico=True, meses=1)
    flujo_paquete.retomar(*args, Path(original['path']).parent, True, calendars=False, historico=False, meses=4)
    assert [c[3:] for c in llamadas if c[0] == 'bitacoras'] == [(True, True), (True, True)]
    assert exportaciones[0] == exportaciones[1]
    assert exportaciones[0][0] is False
    assert exportaciones[0][1]['meses'] == 1


def test_retomar_fallido_conserva_el_zip_anterior_en_su_ruta(monkeypatch, tmp_path):
    import descargador

    fakes(monkeypatch, [], parcial=[True])
    args = (None, CATALOG, lambda *e: None, threading.Event())
    original = flujo_paquete.run(*args, BATCH, tmp_path)
    ruta = Path(original['path'])
    contenido = ruta.read_bytes()

    def exportar(_hojas, _bitacoras, destino, *_args, **_opciones):
        Path(destino).write_bytes(b'ZIP simulado que no supera la validacion')
        raise ValueError('validación simulada fallida')

    monkeypatch.setattr(descargador, 'package_current_lot', exportar)
    with pytest.raises(ValueError, match='validación simulada'):
        flujo_paquete.retomar(*args, ruta.parent)
    assert ruta.read_bytes() == contenido
    assert json.loads((ruta.parent / 'respaldo/proceso.json').read_text())['estado'] == 'INCOMPLETO'
    assert not list((ruta.parent / 'respaldo').glob('Captura_CSMP anterior *.zip'))


def test_retomar_paquete_antiguo_guarda_las_opciones_del_primer_reintento(monkeypatch, tmp_path):
    llamadas = []
    fakes(monkeypatch, llamadas, parcial=[True])
    args = (None, CATALOG, lambda *e: None, threading.Event())
    original = flujo_paquete.run(*args, BATCH, tmp_path)
    carpeta = Path(original['path']).parent
    proceso = carpeta / 'respaldo/proceso.json'
    estado = json.loads(proceso.read_text(encoding='utf-8'))
    del estado['opciones']
    proceso.write_text(json.dumps(estado), encoding='utf-8')

    flujo_paquete.retomar(*args, carpeta, calendars=True)
    assert [c[3] for c in llamadas if c[0] == 'bitacoras'] == [False, True]
    assert json.loads(proceso.read_text(encoding='utf-8'))['opciones']['calendarios'] is True


def test_fallas_seguidas_detienen_el_lote_y_se_puede_retomar(tmp_path):
    class SinRespuesta(Bridge):
        def __init__(self):
            super().__init__()
            self.total = 8

        def call(self, command, payload=None, timeout=75):
            if command == 'diary_open':
                self.openings.append(payload['params'])
                raise PocError('Chrome no respondio a tiempo.')
            return super().call(command, payload, timeout)

    bridge = SinRespuesta()
    cortado = leer(tmp_path, bridge)
    assert cortado['estado'] == 'INCOMPLETA' and len(bridge.openings) == lb.MAX_FALLAS_SEGUIDAS
    assert 'Retomar paquete' in json.loads(Path(cortado['manifest']).read_text(encoding='utf-8'))['error']
    sano = Bridge()
    sano.total = 8
    retomado = leer(tmp_path, sano, resume=cortado['folder'])
    assert retomado['estado'] == 'COMPLETA' and retomado['leidas'] == 8
