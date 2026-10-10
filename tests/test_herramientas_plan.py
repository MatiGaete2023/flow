"""Herramientas del plan: estado por WP y verificador que falla ante la duda."""
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def cargar(nombre):
    spec = importlib.util.spec_from_file_location(nombre, ROOT / 'tools' / f'{nombre}.py')
    modulo = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(modulo)
    return modulo


estado = cargar('estado')
verificar = cargar('verificar_wp')


def test_estado_ida_y_vuelta_identica_y_validacion(tmp_path):
    vista = tmp_path / 'ESTADO.json'
    vista.write_text((ROOT / 'docs/implementacion/ESTADO.json').read_text(encoding='utf-8'), encoding='utf-8')
    fuentes = tmp_path / 'estado'
    estado.split(vista, fuentes)
    assert estado.aggregate(fuentes) == json.loads(vista.read_text(encoding='utf-8'))
    assert estado.dump(estado.aggregate(fuentes)) == estado.dump(json.loads(vista.read_text(encoding='utf-8')))
    paquete = json.loads((fuentes / 'WP-00.json').read_text(encoding='utf-8'))
    paquete['estado'] = 'INVENTADO'
    (fuentes / 'WP-00.json').write_text(estado.dump(paquete), encoding='utf-8')
    assert any('estado desconocido' in p for p in estado.problems(estado.aggregate(fuentes)))


def test_estado_detecta_incoherencias():
    vista = {'conflictos_abiertos': [{'id': 'X', 'respuesta': None}], 'paquetes': {
        'WP-01': {'estado': 'INTEGRADO', 'depende_de': ['WP-02'], 'verificacion': 'PASA'},
        'WP-02': {'estado': 'EN_VERIFICACION', 'depende_de': ['WP-99'], 'ejecutor': None, 'rama': None},
        'WP-03': {'estado': 'PENDIENTE', 'depende_de': ['TODAS_LAS_DEMAS']}}}
    problemas = '\n'.join(estado.problems(vista))
    for texto in ('INTEGRADO con la dependencia WP-02', 'depende de WP-99', 'sin ejecutor', 'sin rama',
                  'TODAS_LAS_DEMAS', 'sin dueño'):
        assert texto in problemas


def test_estado_dependencia_de_archivo_no_bloquea_integracion():
    vista = {'paquetes': {
        'WP-01': {'estado': 'INTEGRADO', 'verificacion': 'PASA',
                  'depende_de': [{'wp': 'WP-02', 'tipo': 'archivo'}]},
        'WP-02': {'estado': 'PENDIENTE', 'depende_de': [{'wp': 'WP-01', 'tipo': 'otro'}]}}}
    problemas = estado.problems(vista)
    assert problemas == ["WP-02: dependencia WP-01 de tipo desconocido 'otro'"]


def base_y_ahora():
    suite = {'aprobadas': 10, 'fallidas': 0, 'resumen': '10 passed'}
    base = {'asistente': dict(suite), 'descargador': dict(suite), 'identidades_reales': 0, 'paridad_golden': 'PASA',
            'restricciones': [], 'ruff_src': 0, 'modulos_no_alcanzables': 0, 'enlaces_rotos': 0}
    return base, json.loads(json.dumps(base))


def test_control_sin_medir_da_falla():
    base, ahora = base_y_ahora()
    ahora['enlaces_rotos'] = None
    ahora['identidades_reales'] = None
    controles = {c['control']: c for c in verificar.compare(ahora, base, 0, 0)}
    assert controles['Enlaces rotos']['resultado'] == 'FALLA' and 'NO_MEDIDO' in controles['Enlaces rotos']['detalle']
    assert controles['Identidades reales']['resultado'] == 'FALLA'


def test_lista_de_eliminadas_reemplaza_conteos_a_mano(tmp_path, monkeypatch):
    base, ahora = base_y_ahora()
    ahora['asistente'].update(aprobadas=8, resumen='8 passed')
    lista = tmp_path / 'eliminadas.txt'
    lista.write_text('tests/test_a.py::test_uno\ntests/test_a.py::test_dos  # solo cubría código retirado\n',
                     encoding='utf-8')
    monkeypatch.setattr(verificar, 'collected_nodeids', lambda suite: {'tests/test_a.py::test_tres'})
    controles = {c['control']: c for c in verificar.compare(ahora, base, 0, 0, verificar.removed_from_lists([lista]))}
    assert controles['Suite asistente']['resultado'] == 'PASA'
    assert controles['Pruebas eliminadas listadas']['resultado'] == 'PASA'
    monkeypatch.setattr(verificar, 'collected_nodeids', lambda suite: {'tests/test_a.py::test_uno'})
    controles = {c['control']: c for c in verificar.compare(ahora, base, 0, 0, verificar.removed_from_lists([lista]))}
    assert controles['Pruebas eliminadas listadas']['resultado'] == 'FALLA'
    sin_lista = {c['control']: c for c in verificar.compare(ahora, base, 2, 0)}
    assert 'sin lista verificable' in sin_lista['Suite asistente']['detalle']
