"""WA-4: avisos solo mostrar N-01..N-04 (decisión del usuario 19-oct-2026); datos ficticios."""

from datetime import date
from types import SimpleNamespace

from openpyxl import Workbook

from nurus.personal.config import defaults
from nurus.personal.contexto_excel import avisos_fila, marcar_paquete
from nurus.personal.work import Work

CORTE = date(2026, 10, 19)
COLS = {'nacimiento': 'NAC', 'egreso_proy': 'EGR', 'vencimiento': 'VENC', 'prox_aud': 'AUD'}


def test_n01_medida_mas_alla_de_la_mayoria():
    avisos = avisos_fila({'NAC': date(2009, 3, 1), 'EGR': date(2027, 6, 1)}, COLS, 'CUMPLIMIENTO', CORTE)
    assert avisos == [
        'N-01: la medida se proyecta hasta el 1 de junio de 2027, posterior a la mayoría de edad (1 de marzo de 2027).'
    ]
    assert avisos_fila({'NAC': date(2010, 3, 1), 'EGR': date(2027, 6, 1)}, COLS, 'CUMPLIMIENTO', CORTE) == []
    # Ya mayor de edad: lo cubre E-02/C-01.
    assert avisos_fila({'NAC': date(2008, 3, 1), 'EGR': date(2027, 6, 1)}, COLS, 'CUMPLIMIENTO', CORTE) == []


def test_n02_informe_vence_despues_de_la_audiencia():
    valores = {'VENC': date(2026, 11, 20), 'AUD': '05/11/2026, 01/03/2027'}
    assert avisos_fila(valores, COLS, 'INFORMES', CORTE) == [
        'N-02: audiencia el 5 de noviembre de 2026 y el informe vence el 20 de noviembre de 2026; '
        'se sugiere solicitarlo antes de la audiencia.'
    ]
    assert avisos_fila({'VENC': date(2026, 12, 20), 'AUD': '05/11/2026'}, COLS, 'INFORMES', CORTE) == []
    assert avisos_fila({'VENC': date(2026, 11, 1), 'AUD': '05/11/2026'}, COLS, 'INFORMES', CORTE) == []
    hoja2 = avisos_fila({'AUD': '05/11/2026'}, COLS, 'CUMPLIMIENTO', CORTE, fecha_hoja2=date(2026, 11, 20))
    assert hoja2 and hoja2[0].startswith('N-02:')


def test_n02_aparece_en_avisos_sin_cambiar_la_observacion(tmp_path):
    path = tmp_path / 'informes.xlsx'
    book = Workbook()
    sheet = book.active
    sheet.append(['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'NOMBRE CENTRO', 'FECHA VENCIMIENTO', 'PROXS. AUDS.'])
    sheet.append(['X-1-2026', 'Laja', 'Persona Ficticia', '1-9', 'PRM FICTICIO', date(2026, 11, 10), date(2026, 11, 5)])
    book.save(path)
    row = Work(defaults()).analyze(path, 'INFORMES', as_of=CORTE).rows[0]
    assert any(w.startswith('N-02:') for w in row.warnings)
    assert row.rules == ['INFORMES.I02_POR_VENCER_GENERAL', 'COMUN.PROX_AUDIENCIA']


def _record(record_type, ingreso, status='VERIFICADA'):
    return {
        'source_system': 'RUS',
        'court_code': 'T1',
        'cause_id': 'c1',
        'ingreso_id': ingreso,
        'record_type': record_type,
        'identity_status': status,
    }


def _identity(row):
    return tuple(row.get(k, '') for k in ('source_system', 'court_code', 'cause_id', 'ingreso_id'))


def _marcar(provenance):
    rows = [SimpleNamespace(id=key, warnings=[]) for key in provenance]
    marcar_paquete(rows, provenance, _identity)
    return {row.id: row.warnings for row in rows}


def test_n03_varios_ingresos_de_una_causa():
    avisos = _marcar({'a': _record('ESPERA', 'i1'), 'b': _record('CUMPLIMIENTO', 'i2')})
    assert all(
        w == ['N-03: la causa tiene 2 ingresos en este trabajo; considerar un solo correo o proyecto.']
        for w in avisos.values()
    )
    assert _marcar({'a': _record('ESPERA', 'i1'), 'b': _record('CUMPLIMIENTO', 'i2', 'INCIERTA')}) == {'a': [], 'b': []}


def test_n04_egresado_con_informe_pendiente():
    avisos = _marcar({'a': _record('EGRESADOS', 'i1'), 'b': _record('INFORMES', 'i1')})
    assert avisos == {
        'a': ['N-04: medida egresada que sigue con informe pendiente en RUS; revisar.'],
        'b': ['N-04: medida egresada que sigue con informe pendiente en RUS; revisar.'],
    }
