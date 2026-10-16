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
