"""WP-33: pausa entre consultas, checkpoint por lotes y máximo de lecturas por lote."""
import json
import threading
import time

from lxml import html

import lectura_bitacoras as lb
import test_lectura_bitacoras as tlb
from lotes import Batch
from preferencias import PAUSA_DEFECTO_S, PAUSA_MINIMA_S, Preferences, clamp_pause


def listing_many(court='777', count=30):
    rows = ''.join(f'''<tr><td>X-{i}-2026</td><td>11111111-1</td><td>Persona ficticia {i}</td><td>PRM Centro ficticio</td><td>
        <a onclick="ShowHistoria('{court}','X-{i}-2026','{100 + i}')">Causa</a>
        <a onclick="ShowObservaciones('{100 + i}','{court}','{5000 + i}','12','3','0')">Obs</a>
        </td></tr>''' for i in range(count))
    return html.fromstring('<table><thead><tr><th>RIT</th><th>RUT</th><th>Nombre</th><th>Derivación</th>'
                           '<th>Acciones</th></tr></thead><tbody>' + rows + '</tbody></table>')


class Bridge30(tlb.Bridge):
    def __init__(self, count=30):
        super().__init__()
        self.total = 1
        self.count = count


def run_many(tmp_path, monkeypatch, count, pause=0):
    monkeypatch.setattr(tlb, 'listing', lambda court='777': listing_many(court, count))
    writes = []
    original = lb.write

    def counting(path, data):
        if path.name == lb.MANIFEST:
            writes.append(len(bridge.openings))
        return original(path, data)

    monkeypatch.setattr(lb, 'write', counting)
    bridge = Bridge30(count)
    tlb.parser()
    result = lb.DiaryRunner(bridge, tlb.CATALOG, lambda *a: None, threading.Event(), pause=pause).run(
        Batch(('49',), ('2',), ('Cumplimiento',)), tmp_path)
    return result, bridge, writes


def test_treinta_lecturas_reescriben_el_manifiesto_como_maximo_cinco_veces(tmp_path, monkeypatch):
    result, bridge, writes = run_many(tmp_path, monkeypatch, 30)
    assert result['estado'] == 'COMPLETA' and result['leidas'] == 30 and len(bridge.openings) == 30
    durante = [w for w in writes if 0 < w < 30]
    assert len(durante) <= 5, writes
    data = json.loads((tmp_path.glob('Bitacoras */' + lb.MANIFEST).__next__()).read_text(encoding='utf-8'))
    assert len(data['registros']) == 30 and all(r['estado'] == 'LEIDA' for r in data['registros'])


def test_maximo_de_lecturas_detiene_el_lote_y_permite_retomar(tmp_path, monkeypatch):
    monkeypatch.setattr(lb, 'MAX_LECTURAS_POR_LOTE', 12)
    result, bridge, _ = run_many(tmp_path, monkeypatch, 20)
    assert len(bridge.openings) == 12
    assert result['estado'] == 'INCOMPLETA' and 'máximo de 12 lecturas' in result['detalle']
    data = json.loads((tmp_path.glob('Bitacoras */' + lb.MANIFEST).__next__()).read_text(encoding='utf-8'))
    assert sum(r['estado'] == 'LEIDA' for r in data['registros']) == 12


def test_pausa_entre_lecturas_se_aplica(tmp_path, monkeypatch):
    inicio = time.monotonic()
    run_many(tmp_path, monkeypatch, 3, pause=0.2)
    assert time.monotonic() - inicio >= 0.5


def test_preferencia_de_pausa_con_minimo(tmp_path):
    assert clamp_pause(0) == PAUSA_MINIMA_S and clamp_pause(3) == 3
    path = tmp_path / 'preferencias.json'
    path.write_text(json.dumps({'pausa_consultas': 0.1}), encoding='utf-8')
    assert Preferences(path).data['pausa_consultas'] == PAUSA_MINIMA_S
    path.write_text(json.dumps({'pausa_consultas': True}), encoding='utf-8')
    assert Preferences(path).data['pausa_consultas'] == PAUSA_DEFECTO_S
    path.write_text(json.dumps({'pausa_consultas': 2}), encoding='utf-8')
    assert Preferences(path).data['pausa_consultas'] == 2


def test_descarga_paginada_espera_la_misma_pausa_entre_paginas(monkeypatch, tmp_path):
    import motor as m
    from lotes import BatchRunner
    from test_lotes import CATALOG, FakeBridge
    seen = []
    original = m.run_sequence

    def spy(*args, **kwargs):
        seen.append(kwargs.get('pause'))
        return original(*args, **kwargs)

    monkeypatch.setattr(m, 'run_sequence', spy)
    waits = []

    class Gate(threading.Event):
        def wait(self, timeout=None):
            waits.append(timeout)
            return False

    BatchRunner(FakeBridge(total=3), CATALOG, lambda *a: None, Gate(), pause=0.7).run(
        Batch(('49',), ('1',), ('Espera',)), tmp_path)
    assert seen == [0.7] and waits.count(0.7) == 2


class EsperaRegistrada(threading.Event):
    def __init__(self):
        super().__init__()
        self.esperas = []

    def wait(self, timeout=None):
        self.esperas.append(timeout)
        return False


def test_pausa_entre_consultas_y_entre_paginas(tmp_path):
    from lotes import BatchRunner
    from test_lotes import CATALOG, FakeBridge
    cancel = EsperaRegistrada()
    resultado = BatchRunner(FakeBridge(), CATALOG, lambda *a: None, cancel, pause=0.5).run(
        Batch(('49',), ('1', '4'), ('Espera', 'Cumplimiento')), tmp_path)
    paginas = sum(item['paginas'] for item in resultado)
    # Una espera antes de cada página n>1 de cada consulta y otra antes de cada consulta salvo la primera.
    assert cancel.esperas == [0.5] * ((paginas - len(resultado)) + (len(resultado) - 1))

