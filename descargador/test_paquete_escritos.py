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
