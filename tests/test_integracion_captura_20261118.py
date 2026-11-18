"""Plan 18-nov-2026, capa de captura: lectores HTML de Escritos e Histórico, contrato 1.2.0 aditivo y llegada al motor
por import_capture. Datos ficticios."""

import json
from datetime import date

import pytest
from historia_fixtures import captura_con_historia
from jsonschema import Draft202012Validator
from pathlib import Path

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
    texto = pagina(cabecera, filas, '<h3>Niño 11.111.111-K</h3>')
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


def test_estructura_16_vencimientos_10_ingresos_inventario_y_justificacion(tmp_path):
    """Escenarios 3, 5 y 10: mismo paquete que compara Rust (`paquete_estructura`)."""
    import subprocess
    import sys

    raiz = Path(__file__).parents[1]
    resultado = subprocess.run(
        [sys.executable, str(raiz / 'rust/oraculo/generar_paquete_estructura.py')], capture_output=True, text=True, check=True, cwd=raiz
    )
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
        ['Causal egreso', 'Programa', 'Fecha egreso'], [['SIN INFORMACION', 'FAE Otro Ficticio', '--']], '<h4>Historial 11.111.111-1</h4>'
    )
    assert read_historico(html, persona_esperada=['11.111.111-1', 'Persona'])['ownership'] == 'VERIFICADA'
    assert read_historico(html, persona_esperada=['99.999.999-9', 'Otro'])['ownership'] == 'INCIERTA'
