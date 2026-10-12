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
        def __init__(self, bridge, catalog, emit, gate, pause):
            self.emit = emit
            used.append(('bitacoras', pause, gate is cancel))

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
