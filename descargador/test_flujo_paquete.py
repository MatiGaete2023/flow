"""Pipeline completo con originales ficticios y exportador real; sin SITFA."""

import json
import os
from pathlib import Path
from threading import Event
import subprocess
import sys

import pytest
from nurus.capture import read_capture

import flujo_paquete
from lotes import Batch
from motor import PocError
from test_reutilizacion_local import bound_lot, diaries


class Scope:
    screen = 'seguimiento'
    tribunals = ('113',)
    tabs = ('Cumplimiento',)

    def plan(self, catalog):
        return ['consulta ficticia']


def runners(monkeypatch, cancel, *, partial=False, cancel_after_queries=False):
    used = []

    class Queries:
        def __init__(self, bridge, catalog, emit, gate, pause):
            self.emit = emit
            used.append(('consultas', pause, gate is cancel))

        def run(self, batch, destination):
            lot = bound_lot(destination / 'principal')
            self.emit('folder', str(lot.folder))
            if cancel_after_queries:
                cancel.set()

    class Diaries:
        def __init__(self, bridge, catalog, emit, gate, pause, reuse_base=None):
            self.emit = emit
            used.append(('bitacoras', pause, gate is cancel))
            Diaries.reuse_base = reuse_base

        def run(self, batch, destination):
            manifest = diaries(destination / 'bitacoras', 2)
            self.emit('folder', str(manifest.parent))
            return {
                'manifest': str(manifest),
                'estado': 'INCOMPLETA' if partial else 'COMPLETA',
                'fallidas': int(partial),
                'consultas_fallidas': 0,
            }

    monkeypatch.setattr(flujo_paquete, 'BatchRunner', Queries)
    monkeypatch.setattr(flujo_paquete, 'DiaryRunner', Diaries)
    return used


def test_consultar_entrega_zip_con_originales_en_respaldo(monkeypatch, tmp_path):
    cancel = Event()
    used = runners(monkeypatch, cancel)
    events = []
    result = flujo_paquete.run(None, {}, lambda *event: events.append(event), cancel, Scope(), tmp_path, pause=1.5)
    path = Path(result['path'])
    assert path.is_file() and not result['partial']
    assert sorted(p.name for p in path.parent.iterdir()) == ['Captura_CSMP.zip', 'respaldo']
    assert Path(result['sheets_folder']).is_relative_to(path.parent / 'respaldo')
    assert Path(result['diary_manifest']).is_file()
    assert len(read_capture(path).rows('bitacoras')) == 2
    assert json.loads((path.parent / 'respaldo/proceso.json').read_text())['estado'] == 'COMPLETO'
    assert used == [('consultas', 1.5, True), ('bitacoras', 1.5, True)]
    assert [value for name, value in events if name == 'phase'] == ['consultas', 'bitacoras', 'paquete']
    assert events[-1][0] == 'package_done'


def test_cancelar_conserva_originales_y_no_crea_zip(monkeypatch, tmp_path):
    cancel = Event()
    used = runners(monkeypatch, cancel, cancel_after_queries=True)
    with pytest.raises(PocError, match='detenido'):
        flujo_paquete.run(None, {}, lambda *event: None, cancel, Scope(), tmp_path)
    assert used == [('consultas', 1.0, True)]
    assert list(tmp_path.rglob('resumen.json'))
    assert not list(tmp_path.rglob('*.zip'))
    assert json.loads(next(tmp_path.rglob('proceso.json')).read_text())['estado'] == 'INCOMPLETO'


def test_lectura_parcial_se_declara_en_resultado(monkeypatch, tmp_path):
    cancel = Event()
    runners(monkeypatch, cancel, partial=True)
    result = flujo_paquete.run(None, {}, lambda *event: None, cancel, Scope(), tmp_path, False)
    assert result['partial']
    assert read_capture(result['path']).consolidated is None
    assert json.loads(next(tmp_path.rglob('proceso.json')).read_text())['estado'] == 'PARCIAL'


def test_error_al_exportar_conserva_fuentes(monkeypatch, tmp_path):
    cancel = Event()
    runners(monkeypatch, cancel)
    import descargador

    def fail(*args):
        raise OSError('disco ficticio no disponible')

    monkeypatch.setattr(descargador, 'package_current_lot', fail)
    with pytest.raises(OSError, match='disco ficticio'):
        flujo_paquete.run(None, {}, lambda *event: None, cancel, Scope(), tmp_path)
    assert list(tmp_path.rglob('resumen.json')) and list(tmp_path.rglob('bitacoras.json'))
    assert not list(tmp_path.rglob('*.zip'))


def test_otras_consultas_no_inician_pipeline(tmp_path):
    with pytest.raises(PocError, match='Seguimiento'):
        flujo_paquete.run(
            None, {}, lambda *event: None, Event(), Batch(('113',), (), (), screen='calendario_informes'), tmp_path
        )
    assert not list(tmp_path.iterdir())


def test_tema_arranca_sin_nurus_instalado():
    script = '''
import sys
from pathlib import Path
src=str(Path.cwd().parent/'src')
class Bloqueo:
    def find_spec(self,name,path=None,target=None):
        if name.split('.')[0]=='nurus' and src not in sys.path:
            raise ModuleNotFoundError("No module named 'nurus'",name='nurus')
sys.meta_path.insert(0,Bloqueo())
from apariencia import apply
from nurus.ui_theme import tokens
assert tokens('Claro')['text']
'''
    result = subprocess.run(
        [sys.executable, '-c', script],
        cwd=Path(__file__).resolve().parent,
        env={**os.environ, 'PYTHONPATH': ''},
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert result.returncode == 0, result.stdout + result.stderr


def runners_historia(monkeypatch, cancel):
    """Diaries ficticio que, si recibe el lector de historia, lo usa como lo hace DiaryRunner real."""
    from test_lectura_historia import LISTADO, PERFIL, QUERY, SELECCION

    class Diaries:
        def __init__(self, bridge, catalog, emit, gate, pause, reuse_base=None, historia=None):
            self.historia = historia

        def run(self, batch, destination):
            manifest = diaries(destination / 'bitacoras', 2)
            if self.historia:
                self.historia.abrir(manifest.parent)
                self.historia.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
                self.historia.cerrar('COMPLETA')
            return {'manifest': str(manifest), 'estado': 'COMPLETA', 'fallidas': 0, 'consultas_fallidas': 0}

    class Queries:
        def __init__(self, bridge, catalog, emit, gate, pause):
            self.emit = emit

        def run(self, batch, destination):
            self.emit('folder', str(bound_lot(destination / 'principal').folder))

    monkeypatch.setattr(flujo_paquete, 'BatchRunner', Queries)
    monkeypatch.setattr(flujo_paquete, 'DiaryRunner', Diaries)


def test_historia_apagada_no_agrega_nada(monkeypatch, tmp_path):
    runners_historia(monkeypatch, Event())
    result = flujo_paquete.run(None, {}, lambda *e: None, Event(), Scope(), tmp_path)
    assert 'historia' not in result and not result['partial']
    assert not list(tmp_path.rglob('Historia_SITFA*.xlsx'))
    assert len(read_capture(result['path']).rows('historia_tramites')) == 0


def test_historia_encendida_crea_excel_y_tabla(monkeypatch, tmp_path):
    from test_lectura_historia import FakeBridge

    runners_historia(monkeypatch, Event())
    result = flujo_paquete.run(FakeBridge(), {}, lambda *e: None, Event(), Scope(), tmp_path, pause=0, historia=True)
    assert not result['partial'] and result['historia']['estado'] == 'COMPLETA'
    assert len(list(Path(result['path']).parent.rglob('Historia_SITFA.xlsx'))) == 1
    assert [f['folio'] for f in read_capture(result['path']).rows('historia_tramites')] == ['9120', '9119', '9117']


def test_historia_sin_incluir_en_paquete_deja_el_excel_pero_no_la_tabla(monkeypatch, tmp_path):
    from test_lectura_historia import FakeBridge

    runners_historia(monkeypatch, Event())
    result = flujo_paquete.run(
        FakeBridge(), {}, lambda *e: None, Event(), Scope(), tmp_path, pause=0, historia=True, historia_en_paquete=False
    )
    assert len(list(Path(result['path']).parent.rglob('Historia_SITFA.xlsx'))) == 1
    assert len(read_capture(result['path']).rows('historia_tramites')) == 0


def test_historia_fallida_no_impide_el_paquete_y_lo_marca_parcial(monkeypatch, tmp_path):
    from test_lectura_historia import FakeBridge

    runners_historia(monkeypatch, Event())
    bridge = FakeBridge(b'<html><table></table></html>')
    result = flujo_paquete.run(bridge, {}, lambda *e: None, Event(), Scope(), tmp_path, pause=0, historia=True)
    assert Path(result['path']).is_file() and result['partial']
    assert read_capture(result['path']).manifest['table_states']['historia_tramites'] == 'FALLIDA'


def test_error_al_incluir_historia_reintenta_sin_ella(monkeypatch, tmp_path):
    from test_lectura_historia import FakeBridge
    import descargador

    runners_historia(monkeypatch, Event())
    original = descargador.package_current_lot

    def con_fallo(*args, **kwargs):
        if kwargs.get('historia'):
            raise ValueError('historia ficticia inconsistente')
        return original(*args)

    monkeypatch.setattr(descargador, 'package_current_lot', con_fallo)
    avisos = []
    result = flujo_paquete.run(
        FakeBridge(), {}, lambda *e: avisos.append(e), Event(), Scope(), tmp_path, pause=0, historia=True
    )
    assert Path(result['path']).is_file() and result['partial']
    assert len(read_capture(result['path']).rows('historia_tramites')) == 0
    assert any(name == 'warning' for name, _ in avisos)


def test_e3_consulta_nueva_no_reutiliza_historia_de_otro_lote(monkeypatch, tmp_path):
    import test_lectura_historia as fixtures

    primero = fixtures.FakeBridge()
    runners_historia(monkeypatch, Event())
    flujo_paquete.run(primero, {}, lambda *e: None, Event(), Scope(), tmp_path, pause=0, historia=True)
    assert len(primero.calls) == 1
    monkeypatch.setattr(fixtures, 'LISTADO', fixtures.LISTADO.replace(b',100)', b',101)'))
    segundo = fixtures.FakeBridge()
    runners_historia(monkeypatch, Event())
    result = flujo_paquete.run(segundo, {}, lambda *e: None, Event(), Scope(), tmp_path, pause=0, historia=True)
    assert len(segundo.calls) == 1 and segundo.calls[0]['causa'] == '101'
    manifest = Path(result['historia']['manifiesto'])
    assert set(json.loads(manifest.read_text())['causas']) == {'RUS|49|101'}
    assert {f['cause_id'] for f in read_capture(result['path']).rows('historia_tramites')} == {'101'}


def test_periodo_a_recolectar_recorta_las_bitacoras_del_paquete(tmp_path):
    from export_csmp_capture import export_capture

    lot = bound_lot(tmp_path / 'principal')
    manifest = diaries(tmp_path / 'bitacoras', 2)
    largo = read_capture(export_capture([lot], tmp_path / 'cuatro.zip', diaries=[manifest], match_diaries=True))
    corto = read_capture(export_capture([lot], tmp_path / 'uno.zip', diaries=[manifest], match_diaries=True, meses=1))
    assert len(largo.rows('bitacoras')) == 2 and corto.rows('bitacoras') == []


def test_periodo_fuera_de_rango_se_rechaza():
    from nurus.capture.temporal import earliest
    from datetime import date

    for meses in (0, 5, True, '2'):
        with pytest.raises(ValueError):
            earliest(date(2026, 10, 14), meses)
    assert earliest(date(2026, 10, 14), 1) == date(2026, 9, 14)
    assert earliest(date(2026, 10, 14)) == date(2026, 6, 14)


def runners_con_calendario(monkeypatch, cancel):
    pantallas = []

    class Queries:
        def __init__(self, bridge, catalog, emit, gate, pause):
            self.emit = emit

        def run(self, batch, destination):
            if batch.screen == 'calendario_informes':
                pantallas.append((batch.month, batch.year))
                lot = bound_lot(destination / f'calendario-{batch.year}-{batch.month}', mode='Informes')
            else:
                lot = bound_lot(destination / 'principal')
            self.emit('folder', str(lot.folder))

    class Diaries:
        def __init__(self, bridge, catalog, emit, gate, pause, reuse_base=None):
            self.emit = emit

        def run(self, batch, destination):
            manifest = diaries(destination / 'bitacoras', 2)
            self.emit('folder', str(manifest.parent))
            return {'manifest': str(manifest), 'estado': 'COMPLETA', 'fallidas': 0, 'consultas_fallidas': 0}

    monkeypatch.setattr(flujo_paquete, 'BatchRunner', Queries)
    monkeypatch.setattr(flujo_paquete, 'DiaryRunner', Diaries)
    return pantallas


class PuenteCatalogo:
    def __init__(self):
        self.llamadas = []

    def call(self, name, args=None, **kwargs):
        self.llamadas.append((name, args))
        return {}


def test_paquete_incluye_los_informes_por_vencer_y_cambia_de_pantalla_solo(monkeypatch, tmp_path):
    from datetime import date

    cancel = Event()
    pantallas = runners_con_calendario(monkeypatch, cancel)
    eventos = []
    puente = PuenteCatalogo()
    result = flujo_paquete.run(
        puente,
        {},
        lambda *event: eventos.append(event),
        cancel,
        Scope(),
        tmp_path,
        informes=True,
        today=date(2026, 10, 14),
    )
    # La extensión abre el calendario sin pedirlo al usuario, cubre hoy + 60 días (14-oct → 13-dic: tres meses) y al
    # terminar vuelve a Seguimiento.
    assert puente.llamadas == [
        ('catalog', {'screen': 'calendario_informes'}),
        ('catalog', {'screen': 'seguimiento'}),
    ]
    assert pantallas == [('10', '2026'), ('11', '2026'), ('12', '2026')] and not result['partial']
    assert [v for n, v in eventos if n == 'phase'] == ['consultas', 'bitacoras', 'informes', 'paquete']
    consultas = read_capture(result['path']).rows('consultas')
    assert len([q for q in consultas if q['screen'] != 'bitacoras']) == 4


def test_si_no_se_puede_abrir_el_calendario_el_paquete_se_crea_parcial(monkeypatch, tmp_path):
    cancel = Event()
    pantallas = runners_con_calendario(monkeypatch, cancel)
    eventos = []

    class SinCalendario(PuenteCatalogo):
        def call(self, name, args=None, **kwargs):
            super().call(name, args, **kwargs)
            if args == {'screen': 'calendario_informes'}:
                raise PocError('No se pudo abrir la pantalla.')
            return {}

    puente = SinCalendario()
    result = flujo_paquete.run(
        puente, {}, lambda *event: eventos.append(event), cancel, Scope(), tmp_path, informes=True
    )
    assert pantallas == [] and result['partial'] and Path(result['path']).is_file()
    assert any(n == 'warning' and 'informes por vencer' in v for n, v in eventos)
    assert puente.llamadas[-1] == ('catalog', {'screen': 'seguimiento'})


def test_un_paquete_por_hoja_entrega_cada_hoja_por_separado(monkeypatch, tmp_path):
    from datetime import date

    cancel = Event()
    pantallas = runners_con_calendario(monkeypatch, cancel)

    puente = PuenteCatalogo()
    batch = Batch(('113',), ('3',), ('Espera', 'Cumplimiento'))
    monkeypatch.setattr(Batch, 'plan', lambda self, catalog: ['consulta ficticia'])
    resultados = flujo_paquete.run_por_hoja(
        puente, {}, lambda *event: None, cancel, batch, tmp_path, informes=True, today=date(2026, 10, 14)
    )
    assert len(resultados) == 2 and all(Path(r['path']).is_file() for r in resultados)
    assert resultados[0]['path'] != resultados[1]['path']
    # El calendario se descarga una vez, con la hoja de Cumplimiento.
    assert puente.llamadas.count(('catalog', {'screen': 'calendario_informes'})) == 1
    assert pantallas == [('10', '2026'), ('11', '2026'), ('12', '2026')]


def test_meses_del_calendario_cubren_sesenta_dias():
    from datetime import date

    assert flujo_paquete.meses_informes(date(2026, 10, 1)) == [('10', '2026'), ('11', '2026')]
    assert flujo_paquete.meses_informes(date(2026, 10, 16)) == [('10', '2026'), ('11', '2026'), ('12', '2026')]
    assert flujo_paquete.meses_informes(date(2026, 12, 20)) == [('12', '2026'), ('1', '2027'), ('2', '2027')]


def test_sin_cumplimiento_no_se_pide_el_calendario(monkeypatch, tmp_path):
    cancel = Event()
    pantallas = runners_con_calendario(monkeypatch, cancel)
    puente = PuenteCatalogo()

    class Espera(Scope):
        tabs = ('Espera',)

    result = flujo_paquete.run(puente, {}, lambda *event: None, cancel, Espera(), tmp_path, informes=True)
    assert puente.llamadas == [] and pantallas == [] and not result['partial']


def test_un_mes_que_falla_conserva_los_otros_y_el_paquete_queda_parcial_con_aviso(monkeypatch, tmp_path):
    from datetime import date

    cancel = Event()
    pantallas = runners_con_calendario(monkeypatch, cancel)
    original = flujo_paquete.BatchRunner

    class FallaDiciembre(original):
        def run(self, batch, destination):
            if batch.screen == 'calendario_informes' and batch.month == '12':
                raise PocError('SITFA no ofrece ese mes.')
            return super().run(batch, destination)

    monkeypatch.setattr(flujo_paquete, 'BatchRunner', FallaDiciembre)
    result = flujo_paquete.run(
        PuenteCatalogo(), {}, lambda *event: None, cancel, Scope(), tmp_path, informes=True, today=date(2026, 10, 16)
    )
    assert pantallas == [('10', '2026'), ('11', '2026')] and result['partial']
    codigos = [i['code'] for i in read_capture(result['path']).rows('incidencias')]
    assert 'CALENDARIO_INCOMPLETO' in codigos
