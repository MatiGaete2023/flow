"""Pruebas de SITFA Lite con SITFA ficticio. Sin red, sin datos reales."""

import json
import re
import shutil
import sys
import uuid
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'app'))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import motor as m  # noqa: E402
from lotes import Batch, BatchRunner, Dates, file_base  # noqa: E402
from pausa import PauseGate  # noqa: E402
from perfiles import SCREENS, Selection  # noqa: E402
from puente import Bridge  # noqa: E402
from sitfa_ficticio import CATALOGS, FakeBridge  # noqa: E402
from version import verify_extension  # noqa: E402


@pytest.fixture
def folder():
    base = ROOT / 'tests' / '_tmp'
    path = base / uuid.uuid4().hex
    path.mkdir(parents=True)
    yield path
    shutil.rmtree(path)


def run(bridge, batch, folder):
    events = []
    result = BatchRunner(bridge, CATALOGS[batch.screen], lambda *e: events.append(e), PauseGate()).run(batch, folder)
    lot = next(folder.iterdir())
    return result, lot, events


def names(lot, ext):
    return sorted(p.name for p in lot.glob('*' + ext))


def test_solo_cuatro_consultas_rus():
    assert set(SCREENS.values()) == {'seguimiento', 'litigantes', 'calendario_informes', 'calendario_medidas'}


def test_puente_rechaza_bitacoras_historia_calendario_por_ingreso():
    bridge = Bridge()
    try:
        for command in ('diary_open', 'history_open', 'calendar_open', 'navigate', 'carga'):
            with pytest.raises(m.PocError, match='Accion no permitida'):
                bridge.call(command)
    finally:
        bridge.close()


def test_seguimiento_pestanas_modalidades_informes_medidas_y_fechas(folder):
    bridge = FakeBridge('seguimiento', pages={('901', 'Cumplimiento', '2', '5'): 3})
    batch = Batch(
        'seguimiento',
        ('901',),
        modalities=('2',),
        tabs=('Cumplimiento', 'Informes'),
        informes=('1', '2'),
        medidas=('0', '5'),
        dates=Dates('01/10/2026', '30/10/2026'),
    )
    result, lot, _ = run(bridge, batch, folder)
    assert len(result) == 4
    xls = names(lot, '.xls')
    assert xls == sorted(
        [
            'Seguimiento - Cumplimiento - Ambulatorio - Todas las medidas - Jdo Familia Ficticio Norte - 01-10-2026 al 30-10-2026.xls',
            'Seguimiento - Cumplimiento - Ambulatorio - Vencidos - Jdo Familia Ficticio Norte - 01-10-2026 al 30-10-2026 - p1 de 3.xls',
            'Seguimiento - Cumplimiento - Ambulatorio - Vencidos - Jdo Familia Ficticio Norte - 01-10-2026 al 30-10-2026 - p2 de 3.xls',
            'Seguimiento - Cumplimiento - Ambulatorio - Vencidos - Jdo Familia Ficticio Norte - 01-10-2026 al 30-10-2026 - p3 de 3.xls',
            'Seguimiento - Informes - Ambulatorio - Por vencer (a 25 dias) - Jdo Familia Ficticio Norte - 01-10-2026 al 30-10-2026.xls',
            'Seguimiento - Informes - Ambulatorio - Vencidos - Jdo Familia Ficticio Norte - 01-10-2026 al 30-10-2026.xls',
        ]
    )
    summary = json.loads((lot / 'resumen.json').read_text(encoding='utf-8'))
    assert summary['estado'] == 'VALIDADA' and summary['consultas_completadas'] == 4
    assert not any(c in bridge.calls for c in ('diary_open', 'history_open', 'calendar_open'))


def test_sin_registros_guarda_pantallazo_pdf(folder):
    bridge = FakeBridge('seguimiento', pages={('902', 'Espera', '1', None): 0})
    batch = Batch('seguimiento', ('901', '902'), modalities=('1',), tabs=('Espera',))
    result, lot, events = run(bridge, batch, folder)
    assert [r['estado'] for r in result] == ['VALIDADA', 'SIN_RESULTADOS']
    pdfs = names(lot, '.pdf')
    assert pdfs == ['Seguimiento - Espera - Residencia - Jdo Familia Ficticio Sur - SIN REGISTROS.pdf']
    assert (lot / pdfs[0]).read_bytes().startswith(b'%PDF-')
    assert names(lot, '.xls') == ['Seguimiento - Espera - Residencia - Jdo Familia Ficticio Norte.xls']
    manifest = json.loads((lot / 'verificacion 002.json').read_text(encoding='utf-8'))
    assert manifest['estado'] == 'SIN_RESULTADOS' and manifest['evidencia']['archivo'] == pdfs[0]
    assert 'evidence_capture' in bridge.calls


def test_busqueda_litigantes_estados_y_fechas(folder):
    bridge = FakeBridge('litigantes', pages={('901', '2'): 2})
    batch = Batch('litigantes', ('901',), states=('1', '2'), dates=Dates('01/09/2026', '30/09/2026'))
    _, lot, _ = run(bridge, batch, folder)
    assert names(lot, '.xls') == [
        'Busqueda Lit - Cumplidas - Jdo Familia Ficticio Norte - 01-09-2026 al 30-09-2026 - p1 de 2.xls',
        'Busqueda Lit - Cumplidas - Jdo Familia Ficticio Norte - 01-09-2026 al 30-09-2026 - p2 de 2.xls',
        'Busqueda Lit - Pendientes - Jdo Familia Ficticio Norte - 01-09-2026 al 30-09-2026.xls',
    ]


@pytest.mark.parametrize('screen,title', [('calendario_informes', 'Informes'), ('calendario_medidas', 'Medidas')])
def test_informes_y_medidas_por_vencer_mes_ano(folder, screen, title):
    bridge = FakeBridge(screen, pages={('902', '11'): 0})
    batch = Batch(screen, ('901', '902'), months=('10', '11'), year='2026')
    result, lot, _ = run(bridge, batch, folder)
    assert len(result) == 4
    assert names(lot, '.xls') == [
        f'{title} por vencer - Noviembre 2026 - Jdo Familia Ficticio Norte.xls',
        f'{title} por vencer - Octubre 2026 - Jdo Familia Ficticio Norte.xls',
        f'{title} por vencer - Octubre 2026 - Jdo Familia Ficticio Sur.xls',
    ]
    assert names(lot, '.pdf') == [f'{title} por vencer - Noviembre 2026 - Jdo Familia Ficticio Sur - SIN REGISTROS.pdf']


def test_calendario_no_admite_rango_de_fechas():
    with pytest.raises(m.PocError):
        Batch('calendario_informes', ('901',), months=('10',), year='2026', dates=Dates('01/10/2026', '02/10/2026')).plan(
            CATALOGS['calendario_informes']
        )


def test_plan_exige_filtros_de_cada_pestana():
    catalog = CATALOGS['seguimiento']
    with pytest.raises(m.PocError, match='tipo de informe'):
        Batch('seguimiento', ('901',), modalities=('2',), tabs=('Informes',)).plan(catalog)
    with pytest.raises(m.PocError, match='medidas'):
        Batch('seguimiento', ('901',), modalities=('2',), tabs=('Cumplimiento',)).plan(catalog)
    plan = Batch('seguimiento', ('901', '902'), modalities=('1', '2', '3', '4'), tabs=('Espera', 'Egresados')).plan(
        catalog
    )
    assert len(plan) == 16


def test_nombres_validos_en_windows_y_acotados():
    catalog = {**CATALOGS['seguimiento'], 'tribunales': {'901': 'Juzgado de Familia ' + 'Muy Largo / Con: Signos? ' * 10}}
    base = file_base(Selection('901', 'Informes', '2', '1'), catalog, Dates('01/10/2026', '30/10/2026'))
    assert len(base) <= 150
    assert not re.search(r'[<>:"/\\|?*]', base)
    assert base.startswith('Seguimiento - Informes - Ambulatorio - Por vencer') and base.endswith('01-10-2026 al 30-10-2026')


def test_verifica_extension_lite():
    assert verify_extension(FakeBridge('seguimiento'))['tipo'] == 'SITFA_LITE_CAPACIDADES'

    class Integral(FakeBridge):
        def call(self, command, payload=None, timeout=75):
            return {'tipo': 'CSMP_RUS_CAPACIDADES', 'version': '2.8.1', 'escritura_rus': False}

    with pytest.raises(m.PocError, match='otra versión'):
        verify_extension(Integral('seguimiento'))


def test_extension_lite_sin_comandos_excluidos():
    js = (ROOT / 'extension' / 'pagina.js').read_text(encoding='utf-8').split('*/', 1)[1]
    js += (ROOT / 'extension' / 'conexion.js').read_text(encoding='utf-8')
    for banned in ('diary_open', 'history_open', 'calendar_open', 'ShowObservaciones', 'ShowHistoria',
                   'Encrypt_AjaxAccion', 'IrPopUpInformesAccion', 'MaoDAction', 'document.cookie'):
        assert banned not in js
    manifest = json.loads((ROOT / 'extension' / 'manifest.json').read_text(encoding='utf-8'))
    assert manifest['permissions'] == ['activeTab', 'scripting']
    assert manifest['version'] == '1.0.0'


def test_selector_dinamico_de_vencimiento_sitfa():
    from filtros_respuesta import selected_report

    script = (ROOT / 'tests' / 'selector_vencimiento_sitfa.js').read_text(encoding='utf-8')
    assert selected_report(m.root_html('<script>' + script + '</script>'), 'tdCumplimiento') == '0'
    with pytest.raises(m.PocError):
        selected_report(m.root_html('<script>' + script.replace("'0'", "'2'") + '</script>'), 'tdCumplimiento')
