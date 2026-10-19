"""WA-5: avisos OB-01..OB-05 de órdenes de búsqueda («Búsqueda Lit», menú 94); valores exclusivamente ficticios."""

from datetime import date
import json

from nurus.capture import write_capture
from nurus.capture.fixtures import fixture
from nurus.personal.capture_input import import_capture
from nurus.personal.config import defaults
from nurus.personal.ordenes_busqueda import avisos_orden

CORTE = date(2026, 10, 19)
# Encabezados tal como los entrega el Excel BusquedaLit (23 columnas).
BASE = {
    'Tipo Causa': 'X', 'Rol Causa': '1', 'Era Causa': '2026', 'Tribunal': 'Juzgado ficticio', 'Rut': 'ID-FICTICIO',
    'Nombre': 'Persona Ficticia', 'Tipo litigante': 'NIÑO', 'Sexo': 'F', 'Tipo O.B.': 'Carabineros',
    'Nomenclatura': 'Orden ficticia', 'Resultado': '', 'Estado O.B.': 'Activa', 'Derivado a': 'Unidad ficticia',
    'Fecha trámite': '01-07-2026', 'Duración Medida': '', 'Estado envío': 'Realizada', 'Fecha Envío': '02/07/2026',
    'E-mail Envío': 'buzon@ejemplo.test', 'Fec. Nacimiento': '10/10/2015', 'Edad': '--', 'Nacionalidad': '1',
    'Prox.Auds.': '-', 'Curador': 'Curador Ficticio',
}


def _codigos(**cambios):
    return [a.split(':')[0] for a in avisos_orden({**BASE, **cambios}, CORTE)]


def test_ob01_activa_sin_resultado_mas_de_60_dias():
    assert _codigos() == ['OB-01']
    assert _codigos(**{'Fecha trámite': '01/09/2026'}) == []
    assert _codigos(**{'Resultado': 'Ubicado'}) == []


def test_ob02_mayor_de_edad():
    assert 'OB-02' in _codigos(**{'Fec. Nacimiento': '23/05/2007'})
    assert 'OB-02' not in _codigos(**{'Fec. Nacimiento': '23/12/2008'})


def test_ob03_envio_no_realizado():
    assert 'OB-03' in _codigos(**{'Estado envío': 'Pendiente'})
    assert 'OB-03' in _codigos(**{'Estado envío': ''})


def test_ob04_nna_sin_curador():
    assert 'OB-04' in _codigos(**{'Curador': ''})
    assert 'OB-04' not in _codigos(**{'Curador': '', 'Tipo litigante': 'ADULTO'})


def test_ob05_audiencia_futura_incluso_con_varias_fechas():
    avisos = avisos_orden({**BASE, 'Prox.Auds.': '01/10/2026, 05/01/2027, 28/12/2026'}, CORTE)
    assert 'OB-05: audiencia fijada para el 28 de diciembre de 2026 con orden de búsqueda activa.' in avisos


def test_ordenes_no_activas_o_sin_columnas_no_avisan():
    assert avisos_orden({**BASE, 'Estado O.B.': 'Cumplida', 'Curador': ''}, CORTE) == []
    assert avisos_orden({'Nombre': 'Persona Ficticia'}, CORTE) == []
    assert _codigos(**{'Curador': ''}) == ['OB-01', 'OB-04']


def test_importacion_de_paquete_marca_las_filas_de_busqueda_lit(tmp_path):
    metadata, tables = fixture(count=1)
    query = {**tables['consultas'][0], 'query_id': 'qlit', 'screen': 'litigantes', 'tab': ''}
    tables['consultas'].append(query)
    metadata['queries'] = tables['consultas']
    record = tables['registros'][0]
    otra = {**record, 'record_id': 'lit1', 'query_id': 'qlit', 'record_type': 'OTRO', 'identity_status': 'INCIERTA',
            'ingreso_id': '', 'cause_id': '', 'normalized_json': json.dumps(BASE, ensure_ascii=False),
            'raw_json': json.dumps(BASE, ensure_ascii=False), 'original_row': '9'}
    tables['registros'].append(otra)
    work = import_capture(write_capture(tmp_path / 'paquete.zip', metadata, tables), defaults(), as_of=CORTE)
    filas = {row.id: row for row in work.rows}
    assert any(w.startswith('OB-01:') for w in filas['lit1'].warnings)
    assert not any(w.startswith('OB-') for w in filas[record['record_id']].warnings)
    assert filas['lit1'].observation == ''
