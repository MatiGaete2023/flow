"""Plan 18-nov-2026: bandeja de Escritos de la causa, leída de la misma respuesta de Historia, llega al ZIP (alcance CAUSA)
con estados diferenciados; sin la pestaña el paquete no cambia (sigue 1.0/1.1) y no se inventa una bandeja vacía."""

import json
import threading

from lectura_historia import LectorHistoria
from nurus.capture import read_capture
from nurus.personal.capture_input import import_capture
from nurus.personal.config import defaults
from datetime import date
from test_lectura_historia import LISTADO, PERFIL, QUERY, SELECCION, VENTANA, FakeBridge
from test_paquete_historia import paquete, validar

ESCRITOS_VACIO = (
    b'<table><thead><tr><th>Doc.</th><th>Tipo Escrito</th><th>Solicitante</th><th>Estado</th></tr></thead>'
    b'<tbody></tbody></table>'
)
ESCRITOS_CON_FILAS = (
    b'<table><thead><tr><th>Estado</th><th>Solicitante</th><th>Doc.</th><th>Tipo Escrito</th></tr></thead><tbody>'
    b'<tr><td>Pendiente</td><td>Programa Ficticio</td><td>DOC-1</td><td>Inf. Cump Programa Ambulatorio</td></tr>'
    b'<tr><td>Resuelto</td><td>Programa Ficticio</td><td>DOC-2</td><td>Otro escrito</td></tr></tbody></table>'
)
ESCRITOS_ILEGIBLE = b'<table><thead><tr><th>Tipo Escrito</th><th>Estado</th></tr></thead><tbody><tr><td>x</td></tr></tbody></table>'


def con_escritos(bloque):
    assert VENTANA.count(b'</body>') == 1
    return VENTANA.replace(b'</body>', bloque + b'</body>')


def historia(tmp_path, cuerpo):
    carpeta = tmp_path / 'lote-historia'
    carpeta.mkdir()
    lector = LectorHistoria(FakeBridge(cuerpo), lambda *a: None, threading.Event(), pause=0)
    lector.abrir(carpeta)
    lector.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    lector.cerrar()
    return carpeta / 'historia' / 'historias.json'


def test_escritos_con_filas_llega_al_paquete_con_literales_de_la_misma_fila(tmp_path):
    ruta = paquete(tmp_path, 'con', historia=historia(tmp_path, con_escritos(ESCRITOS_CON_FILAS)))
    captura = read_capture(ruta)
    assert captura.manifest['schema_version'] == '1.2.0'
    assert captura.manifest['table_states']['escritos'] == 'COMPLETA'
    filas = captura.rows('escritos')
    assert [(f['doc'], f['escrito_type'], f['escrito_status']) for f in filas] == [
        ('DOC-1', 'Inf. Cump Programa Ambulatorio', 'Pendiente'),
        ('DOC-2', 'Otro escrito', 'Resuelto'),
    ]
    assert all(f['scope'] == 'CAUSA' and f['ingreso_id'] == '' and f['cause_id'] == '100' for f in filas)
    consulta = next(q for q in captura.manifest['queries'] if q['screen'] == 'escritos')
    assert json.loads(consulta['filters_json'])['escritos_causa']['cause_id'] == '100'
    validar(ruta)


def test_escritos_con_cabecera_y_cero_filas_es_vacia_comprobada(tmp_path):
    captura = read_capture(paquete(tmp_path, 'vacio', historia=historia(tmp_path, con_escritos(ESCRITOS_VACIO))))
    assert captura.manifest['table_states']['escritos'] == 'VACIA_COMPROBADA' and not captura.rows('escritos')


def test_pestana_presente_pero_ilegible_es_fallida_con_incidencia_y_no_vacia(tmp_path):
    captura = read_capture(paquete(tmp_path, 'mala', historia=historia(tmp_path, con_escritos(ESCRITOS_ILEGIBLE))))
    assert captura.manifest['table_states']['escritos'] == 'FALLIDA' and not captura.rows('escritos')
    assert any(i['code'] == 'ESCRITOS_NO_VERIFICADA' for i in captura.rows('incidencias'))
    # La historia de la misma respuesta sigue leyéndose: un fallo de Escritos no la invalida.
    assert len(captura.rows('historia_tramites')) == 3


def test_sin_la_pestana_el_paquete_no_cambia_y_no_hay_consulta_de_escritos(tmp_path):
    captura = read_capture(paquete(tmp_path, 'ausente', historia=historia(tmp_path, VENTANA)))
    assert captura.manifest['schema_version'] in ('1.0.0', '1.1.0')
    assert 'escritos' not in captura.manifest['table_states'] or captura.manifest['table_states']['escritos'] == 'NO_SOLICITADA'
    assert all(q['screen'] != 'escritos' for q in captura.manifest['queries'])


def test_el_asistente_importa_el_paquete_con_escritos_sin_cambiar_las_reglas_originales(tmp_path):
    base = import_capture(paquete(tmp_path, 'a', historia=historia(tmp_path / 'x', VENTANA) if (tmp_path / 'x').mkdir() is None else None), defaults(), as_of=date(2099, 10, 14))
    otro = tmp_path / 'y'
    otro.mkdir()
    con = import_capture(paquete(tmp_path, 'b', historia=historia(otro, con_escritos(ESCRITOS_CON_FILAS))), defaults(), as_of=date(2099, 10, 14))
    assert [(r.observation, r.actions) for r in con.rows] == [(r.observation, r.actions) for r in base.rows]


# ------------------------------------------------------------------ Ingreso Histórico Protección (captura manual)
from historico_manual import capturar  # noqa: E402

HISTORICO_HTML = (
    '<html><body><h4>Historial 11.111.111-1</h4><table><thead><tr><th>Programa</th><th>Fecha egreso</th><th>Causal egreso</th></tr>'
    '</thead><tbody><tr><td>PRM Ficticio</td><td>--</td><td>SIN INFORMACION</td></tr>'
    '<tr><td>FAE Otro Ficticio</td><td>--</td><td>SIN INFORMACION</td></tr>'
    '<tr><td>PDE Antiguo</td><td>01/05/2026</td><td>EGRESO</td></tr></tbody></table></body></html>'
)


def _persona(tmp_path):
    base = read_capture(paquete(tmp_path, 'base'))
    registro = base.rows('registros')[0]
    return {'tribunal': registro['court_code'], 'tribunal_nombre': registro['court_name'],
            'person_id': registro['person_id'] or 'persona-ficticia-1', 'rut': '11.111.111-1', 'nombre': 'Persona'}  # fmt: skip


def test_historico_manual_llega_al_paquete_con_pertenencia_y_sin_consultas_por_rut(tmp_path):
    persona = _persona(tmp_path)
    carpeta = tmp_path / 'hist'
    capturar(carpeta, HISTORICO_HTML, persona, capturada='2099-10-13T12:00:00+00:00')
    captura = read_capture(paquete(tmp_path, 'con-hist', historico=carpeta))
    assert captura.manifest['schema_version'] == '1.2.0'
    filas = captura.rows('historico_proteccion')
    assert [(f['program_name'], f['egreso_date'], f['egreso_cause'], f['ownership']) for f in filas] == [
        ('PRM Ficticio', '--', 'SIN INFORMACION', 'VERIFICADA'),
        ('FAE Otro Ficticio', '--', 'SIN INFORMACION', 'VERIFICADA'),
        ('PDE Antiguo', '01/05/2026', 'EGRESO', 'VERIFICADA'),
    ]
    assert captura.manifest['table_states']['historico_proteccion'] == 'COMPLETA'


def test_historico_sin_prueba_de_pertenencia_queda_incierto_y_la_fuente_ilegible_fallida(tmp_path):
    persona = _persona(tmp_path)
    incierta = capturar(tmp_path / 'a', HISTORICO_HTML, {**persona, 'rut': '99.999.999-9', 'nombre': 'Otro'})
    assert incierta['ownership'] == 'INCIERTA'
    mala = capturar(tmp_path / 'b', '<html><body><p>nada</p></body></html>', persona)
    assert mala['estado'] == 'FALLIDA' and mala['filas'] == []
    captura = read_capture(paquete(tmp_path, 'mal', historico=tmp_path / 'b'))
    assert captura.manifest['table_states']['historico_proteccion'] == 'FALLIDA'
    assert any(i['code'] == 'HISTORICO_NO_VERIFICADA' for i in captura.rows('incidencias'))


def test_la_direccion_de_la_ventana_prueba_o_desmiente_la_pertenencia(tmp_path):
    from historico_manual import rut_de_url

    url = 'https://ejemplo.invalido/PopUpHistorial.jsp?tipo_popUp=77&RUT_Litigante=11111111&RUT_DV=1'
    assert rut_de_url(url) == '111111111' and rut_de_url('https://x/?a=1') == ''
    base = {'tribunal': '113', 'person_id': 'p1', 'nombre': '', 'rut': '11.111.111-1'}
    sin_texto = HISTORICO_HTML.replace('Historial 11.111.111-1', 'Historial')
    assert capturar(tmp_path / 'a', sin_texto, base)['ownership'] == 'INCIERTA'  # el texto no trae el RUT
    assert capturar(tmp_path / 'b', sin_texto, base, url=url)['ownership'] == 'VERIFICADA'
    otra = url.replace('RUT_Litigante=11111111', 'RUT_Litigante=22222222')
    assert capturar(tmp_path / 'c', HISTORICO_HTML, base, url=otra)['ownership'] == 'INCIERTA'  # pide otro RUT
    assert 'RUT_Litigante' not in (tmp_path / 'b' / 'historico.json').read_text(encoding='utf-8')  # no se guarda la dirección


def test_cli_de_captura_manual(tmp_path, capsys):
    from historico_manual import main

    html = tmp_path / 'vista.html'
    html.write_text(HISTORICO_HTML, encoding='utf-8')
    codigo = main(['--carpeta', str(tmp_path / 'lote'), '--html', str(html), '--tribunal', '113', '--person-id', 'p1', '--rut', '11.111.111-1'])
    assert codigo == 0 and 'pertenencia VERIFICADA' in capsys.readouterr().out


def test_extracto_anonimo_de_escritos_no_emite_valores(tmp_path, capsys):
    from extraer_estructura_escritos import main

    html = tmp_path / 'e.html'
    html.write_text(
        '<html><body><table><tr><th>Doc.</th><th>Tipo Escrito</th><th>Solicitante</th><th>Estado</th></tr>'
        '<tr><td><a href="/Doc.do?id=123456&clave=QWxhZGRpbjpvcGVuIHNlc2FtZQ%3D%3D" onclick="verDoc(\'987654\',\'x\')"><img src="a.gif"></a></td>'
        '<td>inf. cump programa ambulatorio</td><td>tercerista</td><td>pendiente</td></tr></table></body></html>',
        encoding='utf-8',
    )
    assert main([str(html)]) == 0
    salida = capsys.readouterr().out
    for valor in ('123456', '987654', 'QWxhZGRpbjpvcGVuIHNlc2FtZQ', 'tercerista', 'inf. cump', 'Doc.do'):
        assert valor not in salida, valor
    assert '"funcion": "verDoc"' in salida and 'opaco(' in salida and '"nombre": "clave"' in salida
