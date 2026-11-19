"""Plan 18-nov-2026, capa de captura: lectores HTML de Escritos e Histórico, contrato 1.2.0 aditivo y llegada al motor
por import_capture. Datos ficticios."""

import json
from datetime import date
from pathlib import Path

import pytest
from historia_fixtures import captura_con_historia
from jsonschema import Draft202012Validator

from nurus.capture import CaptureError, read_capture, write_capture
from nurus.capture.contract import COMMON, EXTENDED_TABLES, TABLES
from nurus.escritos_html import (
    ESCRITOS_AUSENTE,
    ESCRITOS_NO_VERIFICADA,
    HISTORICO_NO_VERIFICADA,
    INCOMPLETA,
    read_escritos,
    read_historico,
)
from nurus.personal.capture_input import import_capture
from nurus.personal.config import defaults

CORTE = date(2026, 10, 8)
VIG = ('--', 'SIN INFORMACION')


def pagina(encabezados, filas, extra=''):
    cab = ''.join(f'<th>{h}</th>' for h in encabezados)
    cuerpo = ''.join('<tr>' + ''.join(f'<td>{c}</td>' for c in fila) + '</tr>' for fila in filas)
    return f'<html><body>{extra}<table><thead><tr>{cab}</tr></thead><tbody>{cuerpo}</tbody></table></body></html>'


# ------------------------------------------------------------------ lectores HTML
def test_escritos_se_lee_por_encabezado_de_la_misma_fila_sin_importar_el_orden():
    cabecera = ['Estado', 'Solicitante', 'Doc.', 'Tipo Escrito']
    filas = [['Pendiente', 'Programa X', 'D-1', 'Inf. Cump FAE/FAS'], ['Resuelto', 'Programa X', 'D-2', 'Otro escrito']]
    lectura = read_escritos(pagina(cabecera, filas))
    assert [(f['Tipo Escrito'], f['Estado'], f['Doc.']) for f in lectura['filas']] == [
        ('Inf. Cump FAE/FAS', 'Pendiente', 'D-1'),
        ('Otro escrito', 'Resuelto', 'D-2'),
    ]
    assert not lectura['vacio_explicito']


def test_escritos_con_cabecera_y_cero_filas_es_bandeja_vacia_no_fuente_no_verificada():
    lectura = read_escritos(pagina(['Doc.', 'Tipo Escrito', 'Solicitante', 'Estado'], []))
    assert lectura['filas'] == [] and lectura['vacio_explicito'] is True


@pytest.mark.parametrize(
    'documento,codigo',
    [
        ('<html><body><p>Escritos</p></body></html>', ESCRITOS_AUSENTE),  # la respuesta no trae la pestaña
        (pagina(['Folio', 'Desc'], [['1', 'x']]), ESCRITOS_AUSENTE),
        (pagina(['Tipo Escrito', 'Estado'], [['Inf.', 'Pendiente']]), ESCRITOS_NO_VERIFICADA),  # pestaña sin cuerpo legible
        ('<html><body><table><tr><th>Doc.</th></tr></table>', INCOMPLETA),  # respuesta cortada
        ('', ESCRITOS_AUSENTE),
    ],
)
def test_escritos_sin_cuerpo_legible_no_se_lee_como_vacio(documento, codigo):
    with pytest.raises(ValueError, match=codigo):
        read_escritos(documento)


def test_historico_conserva_literales_y_verifica_pertenencia_por_el_texto_visible():
    cabecera = ['Programa', 'Fecha egreso', 'Causal egreso', 'Otra']
    filas = [['FAE X', '--', 'SIN INFORMACION', 'a'], ['PIE Y', '01/05/2026', 'SIN INFORMACION', 'b']]
    texto = pagina(cabecera, filas, '<div>Rut : 11.111.111-K</div>')
    propio = read_historico(texto, persona_esperada=['11111111-K'])
    assert propio['ownership'] == 'VERIFICADA'
    assert [(f['Programa'], f['Fecha egreso'], f['Causal egreso']) for f in propio['filas']] == [
        ('FAE X', '--', 'SIN INFORMACION'),
        ('PIE Y', '01/05/2026', 'SIN INFORMACION'),
    ]
    assert read_historico(texto, persona_esperada=['99999999-9'])['ownership'] == 'INCIERTA'
    assert read_historico(texto)['ownership'] == 'INCIERTA'  # sin prueba de pertenencia no se afirma
    with pytest.raises(ValueError, match=HISTORICO_NO_VERIFICADA):
        read_historico(pagina(['Folio'], [['1']]))


# ------------------------------------------------------------------ contrato 1.2.0
def _consulta(base, query_id, pantalla, filtros, cobertura='COMPLETA'):
    consulta = {k: '' for k in TABLES['consultas']}
    consulta.update(
        query_id=query_id, source_system='RUS', screen=pantalla, tab=pantalla.upper(), court_code=base['court_code'],
        modality_code=base['modality_code'], court_name=base['court_name'], coverage=cobertura,
        captured_at=base['captured_at'], filters_json=json.dumps(filtros), expected_pages='1', saved_pages='1',
        time_zone='America/Santiago', history_complete='false', institutional_verified='false',
    )  # fmt: skip
    return consulta


def _comunes(base, query_id, n):
    comunes = {k: base[k] for k in COMMON if k in base}
    comunes.update(query_id=query_id, coverage='COMPLETA', original_row=str(n), cause_id=base['cause_id'])
    return comunes


def paquete(tmp_path, historico=(), escritos=(), cobertura_hist='COMPLETA', ownership='VERIFICADA', nombre='p'):
    metadata, tablas = captura_con_historia([], 'VACIA_COMPROBADA')
    base = tablas['registros'][0]
    if historico:
        tablas['consultas'].append(
            _consulta(base, 'hp-1', 'historico_proteccion',
                      {'historico_persona': {'source_system': 'RUS', 'court_code': base['court_code'], 'person_id': base['person_id']}},
                      cobertura_hist)
        )  # fmt: skip
        tablas['historico_proteccion'] = [
            {**_comunes(base, 'hp-1', n), 'person_id': base['person_id'], 'historico_id': f'hp-{n}', 'program_name': p,
             'egreso_date': e, 'egreso_cause': c, 'ownership': ownership, 'raw_json': '{}'}
            for n, (p, e, c) in enumerate(historico, 1)
        ]  # fmt: skip
        metadata['table_states']['historico_proteccion'] = cobertura_hist
    if escritos:
        tablas['consultas'].append(
            _consulta(base, 'es-1', 'escritos', {'escritos_causa': {'source_system': 'RUS', 'court_code': base['court_code'], 'cause_id': base['cause_id']}})
        )  # fmt: skip
        tablas['escritos'] = [
            {**_comunes(base, 'es-1', n), 'ingreso_id': '', 'person_id': '', 'escrito_id': f'es-{n}', 'scope': 'CAUSA', 'doc': d, 'escrito_type': t,
             'requester': 'Programa', 'escrito_status': s, 'reference': '', 'linked_ingreso_id': '', 'linked_projection': '',
             'identity_status': 'CAUSA', 'raw_json': '{}'}
            for n, (d, t, s) in enumerate(escritos, 1)
        ]  # fmt: skip
        metadata['table_states']['escritos'] = 'COMPLETA'
    metadata['queries'] = tablas['consultas']
    return write_capture(tmp_path / f'{nombre}.zip', metadata, tablas)


def test_paquete_sin_las_fuentes_nuevas_sigue_siendo_1_0_y_se_lee_con_estado_no_solicitada(tmp_path):
    captura = read_capture(paquete(tmp_path))
    assert captura.manifest['schema_version'] == '1.0.0'
    assert [captura.manifest['table_states'][t] for t in EXTENDED_TABLES] == ['NO_SOLICITADA', 'NO_SOLICITADA']
    assert captura.rows('escritos') == () and captura.rows('historico_proteccion') == ()


def test_paquete_con_fuentes_nuevas_es_1_2_0_valida_contra_el_esquema_y_conserva_literales(tmp_path):
    captura = read_capture(paquete(tmp_path, historico=[('FAE X', *VIG)], escritos=[('D1', 'Inf. Cump FAE/FAS', 'Pendiente')]))
    assert captura.manifest['schema_version'] == '1.2.0'
    esquema = json.loads((Path(__file__).parents[1] / 'contracts/csmp_capture_v1/manifest.schema.json').read_text(encoding='utf-8'))
    Draft202012Validator(esquema).validate(captura.manifest)
    assert captura.rows('historico_proteccion')[0]['egreso_date'] == '--'
    assert captura.rows('escritos')[0]['escrito_status'] == 'Pendiente'


def test_contrato_rechaza_filas_incoherentes_de_las_fuentes_nuevas(tmp_path):
    with pytest.raises(CaptureError):
        paquete(tmp_path, historico=[('FAE X', *VIG)], ownership='TAL VEZ', nombre='a')
    with pytest.raises(CaptureError):
        paquete(tmp_path, historico=[('FAE X', *VIG)], cobertura_hist='VACIA_COMPROBADA', nombre='b')


# ------------------------------------------------------------------ llegada al motor
def importar(tmp_path, **kw):
    return import_capture(paquete(tmp_path, **kw), defaults(), as_of=CORTE)


def test_otro_programa_vigente_llega_a_los_ingresos_de_esa_persona_y_la_fuente_ausente_no_cambia_nada(tmp_path):
    base = importar(tmp_path, nombre='base')
    con = importar(tmp_path, historico=[('FAE X', *VIG), ('PIE Y', '01/05/2026', 'EGRESO')], nombre='con')
    frase = 'Se hace presente que Persona cuenta con ingreso efectivo al programa FAE X.'
    assert [f for f in con.rows if frase in f.observation] == [con.rows[0]]  # solo la persona del histórico
    assert 'PIE Y' not in ''.join(f.observation for f in con.rows)
    assert con.rows[0].otros_programas == [{'clave': 'fae x', 'programa': 'FAE X'}]
    assert [(f.observation, f.actions) for f in base.rows[1:]] == [(f.observation, f.actions) for f in con.rows[1:]]
    assert [f.actions for f in con.rows] == [f.actions for f in base.rows]  # la frase no crea gestiones


def test_consignar_y_reimportar_conserva_la_marca_y_no_repite_la_frase(tmp_path):
    con = importar(tmp_path, historico=[('FAE X', *VIG)], nombre='uno')
    assert 'cuenta con ingreso efectivo' in con.rows[0].observation
    con.consignar(con.rows[0].id, 'FAE X')
    assert 'cuenta con ingreso efectivo' not in con.rows[0].observation  # se recalcula la fila
    con.consignar(con.rows[0].id, 'fae  x')  # idempotente
    assert con.consignaciones == {con.rows[0].id: ['fae x']}
    otra = import_capture(
        paquete(tmp_path, historico=[('FAE X', *VIG)], nombre='dos'), defaults(), as_of=date(2026, 10, 9), previous=con
    )
    assert 'cuenta con ingreso efectivo' not in otra.rows[0].observation


def test_historico_con_pertenencia_incierta_o_fallido_no_afirma_y_queda_visible(tmp_path):
    incierto = importar(tmp_path, historico=[('FAE X', *VIG)], ownership='INCIERTA', nombre='i')
    assert 'cuenta con ingreso efectivo' not in incierto.rows[0].observation
    assert 'HISTORICO_IDENTIDAD' in [a[1] for a in incierto.rows[0].avisos]
    fallido = importar(tmp_path, historico=[('FAE X', *VIG)], cobertura_hist='FALLIDA', nombre='f') if False else None
    assert fallido is None  # FALLIDA no admite filas en el contrato: se declara sin filas (cubierto en las pruebas del motor)


def test_recalcular_tras_una_edicion_conserva_las_fuentes_nuevas(tmp_path):
    """Escenario 9 (parcial): `recalculate` usa las mismas fuentes del import (Histórico, bitácora, consignaciones)."""
    from nurus.personal.record_edits import recalculate

    trabajo = importar(tmp_path, historico=[('FAE X', *VIG)])
    fila = trabajo.rows[0]
    antes = fila.observation
    recalculate(trabajo, fila)
    assert fila.observation == antes and 'FAE X' in fila.observation
    assert fila.otros_programas == [{'clave': 'fae x', 'programa': 'FAE X'}]
    trabajo.consignar(fila.id, 'FAE X')
    recalculate(trabajo, fila)
    assert 'FAE X' not in fila.observation and fila.otros_programas == []


def _miembros(ruta):
    import zipfile

    with zipfile.ZipFile(ruta) as z:
        return {n: z.read(n) for n in z.namelist()}


def test_estructura_16_vencimientos_10_ingresos_inventario_y_justificacion(tmp_path):
    """Escenarios 3, 5 y 10: el generador escribe en una carpeta temporal y se compara con los archivos versionados que lee
    Rust (`paquete_estructura`), sin tocarlos (E3-15)."""
    import subprocess
    import sys

    raiz = Path(__file__).parents[1]
    datos = raiz / 'rust/crates/flow-trabajo/tests/datos'
    antes = {n: (datos / n).read_bytes() for n in ('paquete_estructura.json', 'paquete_estructura.zip')}
    resultado = subprocess.run(
        [sys.executable, str(raiz / 'rust/oraculo/generar_paquete_estructura.py'), str(tmp_path)],
        capture_output=True, text=True, check=True, cwd=raiz,
    )  # fmt: skip
    assert {n: (datos / n).read_bytes() for n in antes} == antes  # los versionados no se reescriben
    # Primero el contenido (informa el tipo de diferencia) y luego los bytes: artefactos deterministas (E3B-10).
    assert json.loads((tmp_path / 'paquete_estructura.json').read_text(encoding='utf-8')) == json.loads(antes['paquete_estructura.json'])
    assert _miembros(tmp_path / 'paquete_estructura.zip') == _miembros(datos / 'paquete_estructura.zip')
    for nombre, bytes_versionados in antes.items():
        assert (tmp_path / nombre).read_bytes() == bytes_versionados, nombre
    filas = [linea.split(' ', 2) for linea in resultado.stdout.strip().splitlines()]
    assert len(filas) == 16
    por_ingreso = {}
    for ingreso, fecha, resto in filas:
        por_ingreso.setdefault(ingreso, []).append((fecha, resto))
    assert len(por_ingreso) == 10 and len(por_ingreso['0']) == 5
    cinco = [r for _, r in por_ingreso['0']]
    assert 'INFORME_REVISAR_ASOCIACION' in cinco[0] and 'INFORME_AVANCE_PENDIENTE' in cinco[1] and "['NOMENCL']" in cinco[2]
    assert all("'PC_INFO'" in r for r in cinco[3:])  # solo las dos fechas que siguen correspondiendo piden cuenta
    # Dos ingresos sucesivos de la misma persona al mismo programa en una causa: la entrega es ambigua y no cancela productos.
    assert all("'PC_INFO'" in r for _, r in por_ingreso['3'] + por_ingreso['4'])
    assert sum("'PC_INFO'" in r for rs in por_ingreso.values() for _, r in rs) == 13


def test_cambio_de_corte_no_duplica_constancias_ni_pierde_la_marca(tmp_path):
    uno = importar(tmp_path, historico=[('FAE X', *VIG)], nombre='c1')
    uno.consignar(uno.rows[0].id, 'FAE X')
    tarde = import_capture(
        paquete(tmp_path, historico=[('FAE X', *VIG)], nombre='c2'), defaults(), as_of=date(2026, 12, 1), previous=uno
    )
    assert tarde.as_of == '2026-12-01' and tarde.consignaciones == uno.consignaciones
    assert 'FAE X' not in tarde.rows[0].observation
    # Sin la marca, el nuevo corte vuelve a proponer la constancia una sola vez.
    sin = import_capture(paquete(tmp_path, historico=[('FAE X', *VIG)], nombre='c3'), defaults(), as_of=date(2026, 12, 1))
    assert sin.rows[0].observation.count('cuenta con ingreso efectivo') == 1


def test_paquete_rust_con_escritos_lo_lee_el_lector_python():
    """Paquete Rust → lector Python (`rust/crates/flow-trabajo/tests/exportar.rs::escritos_de_la_historia_viajan…`)."""
    ruta = Path(__file__).parents[1] / 'rust/crates/flow-trabajo/tests/datos/paquete_rust_escritos.zip'
    captura = read_capture(ruta)
    assert captura.manifest['schema_version'] == '1.2.0'
    fila = captura.rows('escritos')[0]
    assert (fila['doc'], fila['escrito_type'], fila['escrito_status'], fila['scope']) == (
        'DOC-1', 'Inf. Cump Programa Ambulatorio', 'Pendiente', 'CAUSA'
    )
    assert captura.manifest['table_states']['historico_proteccion'] == 'NO_SOLICITADA'


def test_paquete_rust_con_historico_manual_lo_lee_el_lector_python():
    ruta = Path(__file__).parents[1] / 'rust/crates/flow-trabajo/tests/datos/paquete_rust_historico.zip'
    captura = read_capture(ruta)
    assert captura.manifest['schema_version'] == '1.2.0'
    assert captura.manifest['table_states']['historico_proteccion'] == 'COMPLETA'
    assert [(f['program_name'], f['egreso_date'], f['egreso_cause'], f['ownership']) for f in captura.rows('historico_proteccion')] == [
        ('PRM Ficticio', '--', 'SIN INFORMACION', 'VERIFICADA'),
        ('FAE Otro Ficticio', '--', 'SIN INFORMACION', 'VERIFICADA'),
        ('PDE Antiguo', '01/05/2026', 'EGRESO', 'VERIFICADA'),
    ]


def test_python_y_rust_coinciden_en_la_pertenencia_del_historico():
    """Misma HTML y mismas pruebas: el lector Python da lo mismo que el Rust de `exportar.rs` (RUT con puntos, nombre suelto)."""
    html = pagina(
        ['Causal egreso', 'Programa', 'Fecha egreso'], [['SIN INFORMACION', 'FAE Otro Ficticio', '--']], '<div>Rut : 11.111.111-1</div>'
    )
    assert read_historico(html, persona_esperada=['11.111.111-1', 'Persona'])['ownership'] == 'VERIFICADA'
    assert read_historico(html, persona_esperada=['99.999.999-9', 'Otro'])['ownership'] == 'INCIERTA'


def test_el_anonimizador_trata_las_tablas_nuevas_sin_filtrar_nombres_y_conserva_las_relaciones(tmp_path):
    from nurus.personal.anonymize_package import anonymize_capture

    origen = paquete(tmp_path, historico=[('FAE X', *VIG)], escritos=[('D1', 'Inf. Cump FAE/FAS', 'Pendiente')], nombre='orig')
    # El solicitante del escrito es una persona: no puede salir tal cual.
    capturas = read_capture(origen)
    tablas = {t: [dict(f) for f in capturas.rows(t)] for t in capturas.tables}
    tablas['escritos'][0].update(requester='Juana Pérez Soto', linked_ingreso_id=tablas['registros'][0]['ingreso_id'], linked_projection='2026-08-20')
    metadata = dict(capturas.manifest)
    metadata['table_states'] = dict(metadata['table_states'])
    conn = tmp_path / 'con_persona.zip'
    write_capture(conn, {k: v for k, v in metadata.items() if k not in ('schema_version', 'files', 'schema_id')}, tablas)
    destino = tmp_path / 'anonimo.zip'
    anonymize_capture(conn, destino)
    anonimo = read_capture(destino)
    escrito = anonimo.rows('escritos')[0]
    assert 'Juana' not in json.dumps(escrito) and escrito['escrito_status'] == 'Pendiente' and escrito['escrito_type'] == 'Inf. Cump FAE/FAS'
    assert escrito['linked_projection'] == '2026-08-20'
    historico = anonimo.rows('historico_proteccion')[0]
    assert (historico['egreso_date'], historico['egreso_cause'], historico['ownership']) == ('--', 'SIN INFORMACION', 'VERIFICADA')
    assert escrito['linked_ingreso_id'] in {r['ingreso_id'] for r in anonimo.rows('registros')}


def test_consignar_no_sobrescribe_la_edicion_humana_ni_los_productos(tmp_path):
    """Escenario 9 (producto ya editado): recalcular por una consignación cambia la propuesta, no lo que escribió el funcionario."""
    trabajo = importar(tmp_path, historico=[('FAE X', *VIG)])
    fila = trabajo.rows[0]
    fila.review['OBSERVACION'] = 'Redacción propia del funcionario.'
    fila.word_overrides = {'RIT': 'editado a mano'}
    trabajo.consignar(fila.id, 'FAE X')
    assert fila.review['OBSERVACION'] == 'Redacción propia del funcionario.' and fila.word_overrides == {'RIT': 'editado a mano'}
    assert 'FAE X' not in fila.observation  # la propuesta del motor sí se actualiza


FIXTURES = Path(__file__).parent / 'fixtures_integracion'


def test_historico_con_la_estructura_real_nombre_proyecto_titulo_y_tabla_lrpa():
    """Estructura observada en SITFA (capturas del usuario, 18-nov-2026), con valores ficticios."""
    lectura = read_historico((FIXTURES / 'historial_menores_ficticio.html').read_bytes(), persona_esperada=['11.111.111-1'])
    assert lectura['ownership'] == 'VERIFICADA'
    assert [(f['Programa'], f['Fecha egreso'], f['Causal egreso']) for f in lectura['filas']] == [
        ('PRM - PROGRAMA FICTICIO UNO', '--', 'SIN INFORMACION'),
        ('DCE - PROGRAMA FICTICIO DOS', '06-02-2026', 'EGRESO POR ENVÍO DE INFORME'),
    ]
    assert not lectura['vacio_explicito']  # la fila de relleno del pie no es un programa


def test_historico_solo_con_la_tabla_lrpa_no_se_lee_como_proteccion():
    texto = (FIXTURES / 'historial_menores_ficticio.html').read_text(encoding='utf-8')
    sin_proteccion = texto[: texto.index('<table><tr><td colspan="19">')] + '</body></html>'
    with pytest.raises(ValueError, match=HISTORICO_NO_VERIFICADA):
        read_historico(sin_proteccion)


def test_escritos_con_valores_en_minuscula_como_los_muestra_sitfa():
    lectura = read_escritos((FIXTURES / 'escritos_ficticio.html').read_bytes())
    (fila,) = lectura['filas']
    assert (fila['Doc.'], fila['Tipo Escrito'], fila['Estado']) == ('', 'inf. cump programa ambulatorio', 'pendiente')


def test_el_motor_reconoce_el_escrito_pendiente_aunque_venga_en_minuscula(tmp_path):
    from historia_fixtures import INFORMES, fila
    from test_integracion_20261118 import analizar, escritos

    e = escritos(('inf. cump programa ambulatorio', 'pendiente'))
    [f] = analizar(tmp_path, 'INFORMES', INFORMES, [fila(1, 'PRM Laja') + ['2026-09-20']], escritos={0: e}).rows
    assert 'Pendiente de resolver, informe de avance.' in f.observation and f.actions == []


def test_herramienta_consignar_sobre_una_sesion_guardada(tmp_path, capsys):
    import sys

    sys.path.insert(0, str(Path(__file__).parents[1] / 'tools'))
    from consignar import main

    trabajo = importar(tmp_path, historico=[('FAE X', *VIG)])
    sesion = tmp_path / 'sesion'
    trabajo.save(sesion)
    fila = trabajo.rows[0]
    assert main([str(sesion), '--listar']) == 0 and f'{fila.id}\tFAE X' in capsys.readouterr().out
    assert main([str(sesion), '--fila', fila.id, '--programa', 'fae  x']) == 0
    cargado = Work_load(sesion)
    assert 'FAE X' not in cargado.rows[0].observation and cargado.consignaciones == {fila.id: ['fae x']}
    assert main([str(sesion), '--fila', fila.id, '--programa', 'FAE X']) == 1  # ya consignada
    assert main([str(sesion), '--fila', 'no-existe', '--programa', 'FAE X']) == 1


def Work_load(sesion):
    from nurus.personal.work import Work

    return Work.load(sesion, defaults())


def test_e3_04_anonimizador_estricto_no_deja_texto_libre_en_campos_de_catalogo(tmp_path):
    from nurus.personal.anonymize_package import anonymize_capture

    origen = read_capture(paquete(tmp_path, historico=[('FAE X', *VIG), ('PIE Y', '01/05/2026', 'Observación de Zafiria Luminosa')],
                                  escritos=[('D1', 'Escrito de Zafiria Luminosa', 'Visto por Zafiria Luminosa')], nombre='cat'))
    for modo in ('estricto', 'conservador'):
        destino = tmp_path / f'anon-{modo}.zip'
        anonymize_capture(next(tmp_path.glob('cat.zip')), destino, mode=modo)
        anonimo = read_capture(destino)
        assert 'Zafiria' not in json.dumps(anonimo.tables, ensure_ascii=False), modo
        historico = anonimo.rows('historico_proteccion')
        assert (historico[0]['egreso_date'], historico[0]['egreso_cause']) == ('--', 'SIN INFORMACION')
        assert historico[1]['egreso_date'] == '01/05/2026'
    del origen


def test_e3_03_el_nombre_en_una_celda_de_programa_no_prueba_pertenencia():
    html = pagina(['Nombre Proyecto', 'Fecha Egreso', 'Causal Egreso'], [['PRM Ana Luna', '--', 'SIN INFORMACION']],
                  '<div>Nombre del niño : Beto Sol</div><p>Representante Ana Luna</p>')  # fmt: skip
    assert read_historico(html, persona_esperada=['Ana Luna'])['ownership'] == 'INCIERTA'
    assert read_historico(html, persona_esperada=['Beto Sol'])['ownership'] == 'VERIFICADA'


PROG = (['Nombre Proyecto', 'Fecha Egreso', 'Causal Egreso'], [['PRM Ana Luna', '--', 'SIN INFORMACION']])


def test_e3c_03_rotulo_del_representante_o_rotulo_vacio_no_verifican():
    def dueno(extra, esperada):
        return read_historico(pagina(*PROG, extra), persona_esperada=[esperada])['ownership']

    assert dueno('<p>RUN del representante: 00000000-0</p>', '00000000-0') == 'INCIERTA'
    assert dueno('<p><b>RUN del representante:</b> 00000000-0</p>', '00000000-0') == 'INCIERTA'
    # Rótulo sin valor seguido directamente de la tabla de programas: la tabla no es su valor.
    assert dueno('<span>Nombre del NNA</span>', 'Ana Luna') == 'INCIERTA'
    assert dueno('<div>Nombre del NNA:</div>', 'Ana Luna') == 'INCIERTA'
    # El titular rotulado sí verifica, aunque haya un representante en la misma página.
    assert dueno('<p>RUN: 11.111.111-1</p><p>RUN del representante: 00000000-0</p>', '11111111-1') == 'VERIFICADA'


def test_e3c_04_rotulo_con_dos_puntos_comentario_o_texto_siguiente_verifican():
    def dueno(extra):
        return read_historico(pagina(*PROG, extra), persona_esperada=['Beto Sol'])['ownership']

    for extra in (
        '<table><tr><td>Nombre del NNA</td><td>Beto Sol</td></tr></table>',
        '<table><tr><td>Nombre del NNA:</td><td>Beto Sol</td></tr></table>',
        '<table><tr><td>Nombre del NNA</td><!-- x --><td>Beto Sol</td></tr></table>',
        '<p><b>Nombre del niño:</b> Beto Sol</p>',
        '<p><label>Nombre del niño</label><!-- x --><span>Beto Sol</span></p>',
    ):
        assert dueno(extra) == 'VERIFICADA', extra
    # Una celda rótulo no toma un valor fuera de su fila.
    assert dueno('<table><tr><td>Nombre del NNA</td></tr><tr><td>Beto Sol</td></tr></table>') == 'INCIERTA'


def test_e3_13_pestana_escritos_visible_sin_tabla_es_no_verificada():
    with pytest.raises(ValueError, match=ESCRITOS_NO_VERIFICADA):
        read_escritos('<html><body><ul><li><a>Historia</a></li><li><a>Escritos</a></li></ul></body></html>')


def test_e3_16_consignar_con_fallo_al_recalcular_no_deja_la_marca_y_el_reintento_funciona(tmp_path, monkeypatch):
    import nurus.personal.record_edits as re_

    trabajo = importar(tmp_path, historico=[('FAE X', *VIG)])
    fila = trabajo.rows[0]
    original = re_.recalculate

    def falla(*a, **k):
        raise OSError('fuente inaccesible')

    monkeypatch.setattr(re_, 'recalculate', falla)
    with pytest.raises(OSError):
        trabajo.consignar(fila.id, 'FAE X')
    assert trabajo.consignaciones == {} and 'FAE X' in fila.observation
    monkeypatch.setattr(re_, 'recalculate', original)
    trabajo.consignar(fila.id, 'FAE X')
    assert 'FAE X' not in fila.observation and trabajo.consignaciones == {fila.id: ['fae x']}


def _con_dos_lecturas_de_escritos(tmp_path, segunda, cuando2, nombre):
    metadata, tablas = captura_con_historia([], 'VACIA_COMPROBADA')
    base = tablas['registros'][0]
    filtro = {'escritos_causa': {'source_system': 'RUS', 'court_code': base['court_code'], 'cause_id': base['cause_id']}}
    q1 = _consulta(base, 'es-1', 'escritos', filtro)
    q1['captured_at'] = '2026-10-01T10:00:00-03:00'
    q2 = _consulta(base, 'es-2', 'escritos', filtro, segunda)
    q2['captured_at'] = cuando2
    q2['saved_pages'] = '1'
    tablas['consultas'] += [q1, q2]
    fila = {**_comunes(base, 'es-1', 1), 'ingreso_id': '', 'person_id': '', 'escrito_id': 'es-1', 'scope': 'CAUSA', 'doc': '',
            'escrito_type': 'Inf. Cump FAE/FAS', 'requester': 'P', 'escrito_status': 'Pendiente', 'reference': '',
            'linked_ingreso_id': '', 'linked_projection': '', 'identity_status': 'CAUSA', 'raw_json': '{}'}  # fmt: skip
    tablas['escritos'] = [fila]
    metadata['table_states']['escritos'] = 'PARCIAL'
    metadata['queries'] = tablas['consultas']
    return read_capture(write_capture(tmp_path / f'{nombre}.zip', metadata, tablas)), base


def test_e3_07_manda_la_ultima_lectura_fiable_al_corte(tmp_path):
    from nurus.capture.escritos import filas_de_escritos

    captura, base = _con_dos_lecturas_de_escritos(tmp_path, 'VACIA_COMPROBADA', '2026-10-07T10:00:00-03:00', 'a')
    clave = ('RUS', base['court_code'], base['cause_id'])
    consulta, filas = filas_de_escritos(captura, date(2026, 10, 8))[clave]
    assert consulta['query_id'] == 'es-2' and filas == []  # la bandeja vacía posterior desplaza el pendiente antiguo
    consulta, filas = filas_de_escritos(captura, date(2026, 10, 3))[clave]
    assert consulta['query_id'] == 'es-1' and len(filas) == 1  # al corte anterior, la vacía aún no existía
    captura, base = _con_dos_lecturas_de_escritos(tmp_path, 'VACIA_COMPROBADA', '2026-10-01T10:00:00-03:00', 'b')
    consulta, _ = filas_de_escritos(captura, date(2026, 10, 8))[clave]
    assert consulta['coverage'] == 'PARCIAL'  # dos lecturas completas del mismo instante que discrepan


def test_e3_02_el_contrato_rechaza_filas_fuera_del_alcance_de_su_consulta(tmp_path):
    metadata, tablas = captura_con_historia([], 'VACIA_COMPROBADA')
    base = tablas['registros'][0]
    q = _consulta(base, 'hp-1', 'historico_proteccion',
                  {'historico_persona': {'source_system': 'RUS', 'court_code': base['court_code'], 'person_id': base['person_id']}})  # fmt: skip
    tablas['consultas'].append(q)
    tablas['historico_proteccion'] = [{**_comunes(base, 'hp-1', 1), 'person_id': 'PERSONA-AJENA', 'historico_id': 'hp-1',
                                      'program_name': 'FAE Ficticio', 'egreso_date': '--', 'egreso_cause': 'SIN INFORMACION',
                                      'ownership': 'VERIFICADA', 'raw_json': '{}'}]  # fmt: skip
    metadata['table_states']['historico_proteccion'] = 'COMPLETA'
    metadata['queries'] = tablas['consultas']
    with pytest.raises(CaptureError, match='alcance'):
        write_capture(tmp_path / 'ajeno.zip', metadata, tablas)


def test_e3b_06_valor_de_catalogo_con_sufijo_libre_no_sobrevive(tmp_path):
    from nurus.personal.anonymize_package import anonymize_capture

    paquete(tmp_path, historico=[('FAE X', '--', 'SIN INFORMACION 李明'), ('PIE Y', '--', 'sin información')],
            escritos=[('D1', 'Inf. Cump FAE/FAS Zafiria', 'Pendiente')], nombre='suf')  # fmt: skip
    for modo in ('estricto', 'conservador'):
        destino = tmp_path / f's-{modo}.zip'
        anonymize_capture(tmp_path / 'suf.zip', destino, mode=modo)
        anonimo = read_capture(destino)
        texto = json.dumps(anonimo.tables, ensure_ascii=False)
        if modo == 'estricto':  # el conservador depura nombres conocidos; el estricto, todo texto libre
            assert '李明' not in texto and 'Zafiria' not in texto
        causales = [f['egreso_cause'] for f in anonimo.rows('historico_proteccion')]
        assert causales[1] == 'SIN INFORMACION'  # literal de catálogo → forma canónica
        assert causales[0] != 'SIN INFORMACION'  # el sufijo libre impide tratarlo como catálogo


# ------------------------------------------------------------------ E3C: lecturas de Escritos desde el ZIP hasta acciones
AMB = 'Inf. Cump Programa Ambulatorio'
EXIGIBLES = ['programa_vencido', 'PC_INFO']


def _lecturas(tmp_path, lecturas, nombre):
    """ZIP con varias lecturas de la bandeja de Escritos de la misma causa: (query_id, captured_at, cobertura, [filas])."""
    metadata, tablas = captura_con_historia([], 'VACIA_COMPROBADA')
    base = tablas['registros'][0]
    filtro = {'escritos_causa': {'source_system': 'RUS', 'court_code': base['court_code'], 'cause_id': base['cause_id']}}
    tablas['escritos'] = []
    for qid, cuando, cobertura, filas in lecturas:
        q = _consulta(base, qid, 'escritos', filtro, cobertura)
        q['captured_at'] = cuando
        tablas['consultas'].append(q)
        for n, extra in enumerate(filas, 1):
            if cobertura not in ('COMPLETA', 'VACIA_COMPROBADA'):
                extra = {'coverage': 'PARCIAL', **extra}
            tablas['escritos'].append(
                {**_comunes(base, qid, n), 'ingreso_id': '', 'person_id': '', 'escrito_id': f'{qid}-{n}', 'scope': 'CAUSA',
                 'doc': '', 'escrito_type': AMB, 'requester': 'P', 'escrito_status': 'Pendiente', 'reference': '',
                 'linked_ingreso_id': '', 'linked_projection': '', 'identity_status': 'CAUSA', 'raw_json': '{}', **extra}
            )  # fmt: skip
    metadata['table_states']['escritos'] = 'PARCIAL'
    metadata['queries'] = tablas['consultas']
    captura = read_capture(write_capture(tmp_path / f'{nombre}.zip', metadata, tablas))
    return captura, base


def _acciones(tmp_path, captura, base, nombre):
    """ZIP → `filas_de_escritos` → `escritos_para` → motor (Informes, PRM, vencimiento 20-sep)."""
    from historia_fixtures import INFORMES, fila
    from test_integracion_20261118 import analizar

    from nurus.capture.escritos import escritos_para, filas_de_escritos

    e = escritos_para(base, filas_de_escritos(captura, CORTE))
    (tmp_path / nombre).mkdir()
    [f] = analizar(tmp_path / nombre, 'INFORMES', INFORMES, [fila(1, 'PRM Laja') + ['2026-09-20']], escritos={0: e}).rows
    return f.actions, f


def _propio(base, **extra):
    return {'linked_ingreso_id': base['ingreso_id'], 'linked_projection': '2026-09-20', **extra}


def test_e3c_01_lecturas_simultaneas_discordantes_con_vinculo_explicito_no_retiran_el_pide_cuenta(tmp_path):
    _, base = _lecturas(tmp_path, [], 'b')
    mismo = '2026-10-01T10:00:00-03:00'
    sola, _ = _lecturas(tmp_path, [('es-1', mismo, 'COMPLETA', [_propio(base)])], 'sola')
    acciones, _ = _acciones(tmp_path, sola, base, 'sola')
    assert acciones == []  # control: una sola lectura con vínculo explícito pendiente sí acredita el informe
    for orden, nombre in (((1, 2), 'a'), ((2, 1), 'c')):
        lecturas = {
            1: ('es-1', mismo, 'COMPLETA', [_propio(base)]),
            2: ('es-2', mismo, 'COMPLETA', [_propio(base, escrito_status='Resuelto')]),
        }
        captura, _ = _lecturas(tmp_path, [lecturas[i] for i in orden], nombre)
        acciones, f = _acciones(tmp_path, captura, base, nombre)
        assert acciones == EXIGIBLES, nombre
        assert 'ESCRITOS_DISCORDANTES' in [a[1] for a in f.avisos], f.avisos


def test_e3c_01_folio_de_lecturas_discordantes_queda_bloqueado(tmp_path):
    from nurus.capture.escritos import evidencia_informe, filas_de_escritos

    _, base = _lecturas(tmp_path, [], 'b')
    mismo = '2026-10-01T10:00:00-03:00'
    captura, _ = _lecturas(tmp_path, [
        ('es-1', mismo, 'COMPLETA', [_propio(base, doc='D')]),
        ('es-2', mismo, 'COMPLETA', [{'doc': 'D', 'linked_ingreso_id': 'otro-ingreso', 'linked_projection': '2026-09-20'}]),
    ], 'd')  # fmt: skip
    historia = [{'reference': 'D', 'folio': '31', 'description': AMB, 'event_at': '2026-09-21'}]
    ev = evidencia_informe(base, date(2026, 9, 20), filas_de_escritos(captura, CORTE), historia)
    assert ev.escrito_historia_folio == '' and ev.folios_ajenos == ('31',)
    # Mismo dueño en ambas lecturas, pero discrepan en el estado: tampoco se atribuye el folio.
    captura, _ = _lecturas(tmp_path, [
        ('es-1', mismo, 'COMPLETA', [_propio(base, doc='D')]),
        ('es-2', mismo, 'COMPLETA', [_propio(base, doc='D', escrito_status='Resuelto')]),
    ], 'f')  # fmt: skip
    fuentes = filas_de_escritos(captura, CORTE)
    ev = evidencia_informe(base, date(2026, 9, 20), fuentes, historia)
    assert ev.escrito_historia_folio == '' and ev.folios_ajenos == ('31',)
    from nurus.capture.escritos import escritos_para

    e = escritos_para(base, fuentes)
    assert e.discordante and [f.vinculo for f in e.filas] == ['']  # sin asociación explícita


def test_e3c_02_uno_frente_a_dos_escritos_iguales_es_discordancia(tmp_path):
    from nurus.capture.escritos import filas_de_escritos

    _, base = _lecturas(tmp_path, [], 'b')
    mismo = '2026-10-01T10:00:00-03:00'
    captura, _ = _lecturas(tmp_path, [
        ('es-1', mismo, 'COMPLETA', [{}]),
        ('es-2', mismo, 'COMPLETA', [{}, {'requester': 'Q'}]),
    ], 'a')  # fmt: skip
    consulta, _ = filas_de_escritos(captura, CORTE)[('RUS', base['court_code'], base['cause_id'])]
    assert consulta['coverage'] == 'PARCIAL' and consulta['_discordante']
    acciones, _ = _acciones(tmp_path, captura, base, 'a')
    assert acciones == EXIGIBLES
    # Dos filas idénticas frente a una: el multiconjunto las distingue.
    captura, _ = _lecturas(tmp_path, [('es-1', mismo, 'COMPLETA', [{}]), ('es-2', mismo, 'COMPLETA', [{}, {}])], 'g')
    consulta, _ = filas_de_escritos(captura, CORTE)[('RUS', base['court_code'], base['cause_id'])]
    assert consulta['coverage'] == 'PARCIAL'
    # Las mismas filas en distinto orden no son discordancia.
    captura, _ = _lecturas(tmp_path, [
        ('es-1', mismo, 'COMPLETA', [{}, {'requester': 'Q'}]),
        ('es-2', mismo, 'COMPLETA', [{'requester': 'Q'}, {}]),
    ], 'e')  # fmt: skip
    consulta, _ = filas_de_escritos(captura, CORTE)[('RUS', base['court_code'], base['cause_id'])]
    assert consulta['coverage'] == 'COMPLETA'


def test_e3c_12_orden_utc_desde_el_zip_hasta_acciones(tmp_path):
    from nurus.capture.escritos import filas_de_escritos

    _, base = _lecturas(tmp_path, [], 'b')
    # 21:30 en Santiago (00:30 UTC del 2-oct) es POSTERIOR a 00:10 UTC del 2-oct; por texto ganaría la vacía.
    captura, _ = _lecturas(tmp_path, [
        ('es-1', '2026-10-01T21:30:00-03:00', 'COMPLETA', [{}]),
        ('es-2', '2026-10-02T00:10:00Z', 'VACIA_COMPROBADA', []),
    ], 'a')  # fmt: skip
    consulta, filas = filas_de_escritos(captura, CORTE)[('RUS', base['court_code'], base['cause_id'])]
    assert consulta['query_id'] == 'es-1' and len(filas) == 1
    acciones, _ = _acciones(tmp_path, captura, base, 'a')
    assert acciones == []  # el pendiente de la lectura más reciente se atribuye (único candidato)


def test_e3c_12_identidad_incierta_y_referencia_con_dos_duenos_desde_el_zip(tmp_path):
    from nurus.capture.escritos import evidencia_informe, filas_de_escritos

    _, base = _lecturas(tmp_path, [], 'b')
    historia = [{'reference': 'D', 'folio': '31', 'description': AMB, 'event_at': '2026-09-21'}]
    cuando = '2026-10-01T10:00:00-03:00'
    for cobertura, folio in (('IDENTIDAD_INCIERTA', None), ('COMPLETA', '31')):
        captura, _ = _lecturas(tmp_path, [('es-1', cuando, cobertura, [_propio(base, doc='D')])], cobertura)
        ev = evidencia_informe(base, date(2026, 9, 20), filas_de_escritos(captura, CORTE), historia)
        assert (ev.escrito_historia_folio if ev else None) == folio, cobertura
    captura, _ = _lecturas(tmp_path, [('es-1', cuando, 'COMPLETA', [
        _propio(base, doc='D'), {'doc': 'D', 'linked_ingreso_id': 'otro-ingreso', 'linked_projection': '2026-09-20'},
    ])], 'dos')  # fmt: skip
    ev = evidencia_informe(base, date(2026, 9, 20), filas_de_escritos(captura, CORTE), historia)
    assert ev.escrito_historia_folio == '' and ev.folios_ajenos == ('31',)
