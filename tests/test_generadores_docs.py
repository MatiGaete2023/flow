"""Los generadores de evidencia solo escriben con --escribir y no enlazan rutas retiradas."""

import importlib.util
import json
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]


def cargar(nombre):
    spec = importlib.util.spec_from_file_location(nombre, ROOT / 'tools' / f'{nombre}.py')
    modulo = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(modulo)
    return modulo


SALIDAS = [
    'docs/MATRIZ_FUNCIONAL_20261008.md',
    'docs/evidencia_p0/matriz_funcional.csv',
    'docs/TRAZABILIDAD_EJECUCION_20261008.md',
    'docs/evidencia_desarrollo_20261008/matriz_actual.json',
]


@pytest.mark.parametrize('nombre', ['flow_baseline_matrix', 'flow_plan_progress'])
def test_sin_escribir_no_modifica_nada(nombre, capsys):
    antes = {ruta: (ROOT / ruta).read_bytes() for ruta in SALIDAS}
    cargar(nombre).main([])
    with pytest.raises(SystemExit):
        cargar(nombre).main(['--help'])
    assert {ruta: (ROOT / ruta).read_bytes() for ruta in SALIDAS} == antes


def test_matriz_no_enlaza_rutas_retiradas():
    retiradas = json.loads((ROOT / 'docs/evidencia_p0/rutas_retiradas.json').read_text(encoding='utf-8'))['rutas']
    matriz = (ROOT / 'docs/MATRIZ_FUNCIONAL_20261008.md').read_text(encoding='utf-8')
    assert matriz.startswith('Histórico (P0).')
    assert not [ruta for ruta in retiradas if f'](../{ruta})' in matriz]
