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
    vista = {
        'conflictos_abiertos': [{'id': 'X', 'descripcion': 'abierto', 'respuesta': None}, {'respuesta': None}],
        '_sin_archivo': ['WP-04'],
        'paquetes': {
            'WP-01': {'estado': 'INTEGRADO', 'depende_de': ['WP-02'], 'verificacion': 'PASA'},
            'WP-02': {'estado': 'EN_VERIFICACION', 'depende_de': ['WP-99'], 'ejecutor': None, 'rama': None},
            'WP-03': {'estado': 'PENDIENTE', 'depende_de': ['TODAS_LAS_DEMAS']},
        },
    }
    problemas = '\n'.join(estado.problems(vista))
    for texto in (
        'INTEGRADO con la dependencia WP-02',
        'depende de WP-99',
        'sin ejecutor',
        'sin rama',
        'TODAS_LAS_DEMAS',
        'falta id o descripción',
        'WP-04: listado en orden_paquetes sin archivo',
    ):
        assert texto in problemas
    assert "'X'" not in problemas  # un conflicto abierto con id y descripción espera al usuario: no es error


def test_estado_dependencia_de_archivo_no_bloquea_integracion():
    vista = {
        'paquetes': {
            'WP-01': {
                'estado': 'INTEGRADO',
                'verificacion': 'PASA',
                'depende_de': [{'wp': 'WP-02', 'tipo': 'archivo'}],
            },
            'WP-02': {'estado': 'PENDIENTE', 'depende_de': [{'wp': 'WP-01', 'tipo': 'otro'}]},
        }
    }
    problemas = estado.problems(vista)
    assert problemas == ["WP-02: dependencia WP-01 de tipo desconocido 'otro'"]


def base_y_ahora():
    suite = {
        'aprobadas': 10,
        'fallidas': 0,
        'omitidas': 0,
        'subpruebas': 0,
        'codigo': 0,
        'medicion_valida': True,
        'resumen': '10 passed',
    }
    base = {
        'commit': 'f' * 40,
        'pruebas_recolectadas': {
            'asistente': ['tests/test_a.py::test_uno', 'tests/test_a.py::test_dos', 'tests/test_a.py::test_tres']
            + [f'tests/test_a.py::test_ficticio_{i}' for i in range(7)],
            'descargador': [f'descargador/test_synthetic.py::test_ficticio_{i}' for i in range(10)],
        },
        'asistente': dict(suite),
        'descargador': dict(suite),
        'identidades_reales': 0,
        'paridad_golden': 'PASA',
        'restricciones': [],
        'ruff_src': 0,
        'modulos_no_alcanzables': 0,
        'enlaces_rotos': 0,
    }
    return base, json.loads(json.dumps(base))


def test_control_sin_medir_da_falla():
    base, ahora = base_y_ahora()
    ahora['enlaces_rotos'] = None
    ahora['identidades_reales'] = None
    controles = {c['control']: c for c in verificar.compare(ahora, base, 0, 0)}
    assert (
        controles['Enlaces rotos']['resultado'] == 'FALLA' and 'no disponible' in controles['Enlaces rotos']['detalle']
    )
    assert controles['Identidades reales']['resultado'] == 'FALLA'


def test_lista_de_eliminadas_reemplaza_conteos_a_mano(tmp_path, monkeypatch):
    base, ahora = base_y_ahora()
    ahora['asistente'].update(aprobadas=8, resumen='8 passed')
    ahora['pruebas_recolectadas']['asistente'] = base['pruebas_recolectadas']['asistente'][2:]
    lista = tmp_path / 'eliminadas.txt'
    lista.write_text(
        'tests/test_a.py::test_uno\ntests/test_a.py::test_dos  # solo cubría código retirado\n', encoding='utf-8'
    )
    monkeypatch.setattr(verificar, 'collected_nodeids', lambda suite: {'tests/test_a.py::test_tres'})
    controles = {
        c['control']: c
        for c in verificar.compare(
            ahora, base, 0, 0, verificar.removed_from_lists([lista], base['commit'], base['pruebas_recolectadas'])
        )
    }
    assert controles['Suite asistente']['resultado'] == 'PASA'
    assert controles['Pruebas eliminadas listadas']['resultado'] == 'PASA'
    monkeypatch.setattr(verificar, 'collected_nodeids', lambda suite: {'tests/test_a.py::test_uno'})
    controles = {
        c['control']: c
        for c in verificar.compare(
            ahora, base, 0, 0, verificar.removed_from_lists([lista], base['commit'], base['pruebas_recolectadas'])
        )
    }
    assert controles['Pruebas eliminadas listadas']['resultado'] == 'FALLA'
    sin_lista = {c['control']: c for c in verificar.compare(ahora, base, 2, 0)}
    assert sin_lista['Suite asistente']['resultado'] == 'FALLA'
    assert '--lista-eliminadas' in sin_lista['Suite asistente']['detalle']


def test_controles_que_no_se_pudieron_medir(monkeypatch):
    monkeypatch.setattr(verificar, 'run', lambda command, **kw: (1, 'Traceback: no es un repositorio git'))
    assert verificar.count_real_ids() is None
    monkeypatch.setattr(verificar, 'run', lambda command, **kw: (2, 'error: configuración inválida'))
    assert verificar.ruff_count() is None
    monkeypatch.setattr(verificar, 'run', lambda command, **kw: (1, '[{"code": "F821"}]'))
    assert verificar.ruff_count() == 1
    monkeypatch.setattr(verificar, 'run', lambda command, **kw: (3, '10 passed in 1.0s'))
    suite = verificar.pytest_counts([])
    assert suite['codigo'] == 3
    assert not verificar.suite_valid(suite)


def test_lista_de_eliminadas_rechaza_nodos_inventados_o_repetidos(tmp_path, monkeypatch):
    lista = tmp_path / 'eliminadas.txt'
    lista.write_text('tests/test_a.py::test_real\ntests/inventado.py::x\ntests/inventado.py::x\n', encoding='utf-8')
    monkeypatch.setattr(verificar, 'collected_nodeids', lambda suite: set())
    inventario = {'asistente': ['tests/test_a.py::test_real'], 'descargador': []}
    _, problemas, declaradas = verificar.removed_from_lists([lista], 'abc1234', inventario)
    assert 'tests/inventado.py::x está repetido' in problemas
    assert any('tests/inventado.py::x no existía en la línea base' in p for p in problemas)
    assert not any('test_real' in p for p in problemas)


def test_existed_in_base_lee_el_archivo_de_la_linea_base(tmp_path, monkeypatch):
    # E2 replaces Git/source inference with exact nodeids measured in the baseline.
    # Keep this historical nodeid so the inventory continues to track its coverage.
    known = 'tests/test_herramientas_plan.py::test_control_sin_medir_da_falla'
    invented = 'tests/test_herramientas_plan.py::test_que_no_existe'
    absent_file = 'tests/no_existe.py::test_x'
    baseline = tmp_path / 'linea_base.json'
    baseline.write_text(
        json.dumps({'commit': 'f' * 40, 'pruebas_recolectadas': {'asistente': [known], 'descargador': []}})
    )
    recorded = json.loads(baseline.read_text())
    manifest = tmp_path / 'removed.txt'
    manifest.write_text('\n'.join([known, invented, absent_file]))
    monkeypatch.setattr(verificar, 'collected_nodeids', lambda suite: set())
    _, problems, declared = verificar.removed_from_lists(
        [manifest], recorded['commit'], recorded['pruebas_recolectadas']
    )
    assert not any(known in problem for problem in problems)
    assert any(invented in problem for problem in problems)
    assert any(absent_file in problem for problem in problems)


def test_check_real_ids_reads_unknown_suffix_by_content(tmp_path):
    import importlib.util
    import io
    import zipfile

    spec = importlib.util.spec_from_file_location('ids', Path(__file__).parents[1] / 'tools/check_real_ids.py')
    ids = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(ids)
    # El dígito verificador se calcula aquí: el archivo de prueba no contiene un RUT válido literal.
    rut = '12345678-' + ids.check_digit('12345678')
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, 'w') as book:
        book.writestr('xl/sharedStrings.xml', '<t>RUT ' + rut + '</t>')
    session = tmp_path / 'abc.bin'
    session.write_bytes(buffer.getvalue())
    script = tmp_path / 'instalar.ps1'
    script.write_text('$rut = "' + rut + '"', encoding='utf-8')
    image = tmp_path / 'logo.png'
    image.write_bytes(b'\x89PNG\x00\x00')
    found = {name: text for path in (session, script, image) for name, text in ids.texts(path)}
    assert any(rut in text for name, text in found.items() if '.bin!' in name)
    assert any(rut in text for name, text in found.items() if name.endswith('.ps1'))
    # La identidad del nombre se revisa; el contenido binario no se interpreta como texto.
    assert [text for name, text in found.items() if name.endswith('.png')] == ['logo.png']


def test_check_real_ids_reads_renamed_member_inside_zip(tmp_path):
    # Revisión externa H5: un XLSX guardado como sesion.bin dentro de otro ZIP se revisa por contenido.
    import importlib.util
    import io
    import zipfile

    spec = importlib.util.spec_from_file_location('ids', Path(__file__).parents[1] / 'tools/check_real_ids.py')
    ids = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(ids)
    rut = '12345678-' + ids.check_digit('12345678')
    inner = io.BytesIO()
    with zipfile.ZipFile(inner, 'w') as book:
        book.writestr('xl/sharedStrings.xml', '<t>RUT ' + rut + '</t>')
    outer = tmp_path / 'anidado.zip'
    with zipfile.ZipFile(outer, 'w') as container:
        container.writestr('sesion.bin', inner.getvalue())
        container.writestr('notas.dat', 'RUT ' + rut)
        container.writestr('logo.png', b'\x89PNG\x00\x00')
    found = dict(ids.texts(outer))
    assert any(rut in text for name, text in found.items() if 'sesion.bin!' in name)
    assert any(rut in text for name, text in found.items() if name.endswith('notas.dat'))
    assert not any(rut in text for name, text in found.items() if name.endswith('logo.png'))


def test_check_real_ids_decodes_utf16_and_cp1252():
    import importlib.util

    spec = importlib.util.spec_from_file_location('ids', Path(__file__).parents[1] / 'tools/check_real_ids.py')
    ids = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(ids)
    rut = '12345678-' + ids.check_digit('12345678')
    assert rut in ids.decode_text(('Año ' + rut).encode('utf-16'))
    assert rut in ids.decode_text(('Año ' + rut).encode('cp1252'))
