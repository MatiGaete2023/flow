"""T-03 con varias audiencias en una celda (RUS V11, «Prox. Auds.» separadas por coma)."""

from datetime import date

import pytest
from openpyxl import Workbook

from nurus.personal.config import defaults
from nurus.personal.motor.utilidades import fechas_varias, lista_fechas_es
from nurus.personal.work import Work


def _cumplimiento(tmp_path, audiencias):
    path = tmp_path / 'cumplimiento_audiencias.xlsx'
    book = Workbook()
    sheet = book.active
    sheet.title = 'Cumplimiento'
    sheet.append(
        [
            'DERIVACION',
            'TRIBUNAL',
            'NOMBRE',
            'RIT',
            'DIAS DE CUMPLIMIENTO',
            'DIAS PARA EGRESAR',
            'FEC.EGRESO PROYECTADO',
            'PROXS. AUDS.',
            'CURADOR',
        ]
    )
    sheet.append(
        [
            'AFT CENTRO',
            'LAJA',
            'Persona Ejemplo',
            'X-1',
            100,
            100,
            date(2027, 1, 27),
            audiencias,
            '11111111-1 Curador Ficticio',
        ]
    )
    book.save(path)
    return path


def _fila(tmp_path, audiencias, config=None):
    work = Work(config or defaults()).analyze(
        _cumplimiento(tmp_path, audiencias), 'CUMPLIMIENTO', as_of=date(2026, 10, 19)
    )
    return work.rows[0]


def test_una_audiencia_conserva_el_texto_singular(tmp_path):
    row = _fila(tmp_path, date(2026, 12, 28))
    assert 'Audiencia fijada para el 28 de diciembre de 2026.' in row.observation
    assert 'COMUN.PROX_AUDIENCIAS' not in row.rules


def test_varias_audiencias_futuras_usan_el_texto_plural_aprobado(tmp_path):
    row = _fila(tmp_path, '05/01/2027, 28/12/2026')
    assert 'Audiencias fijadas para el 28 de diciembre de 2026 y el 5 de enero de 2027.' in row.observation
    assert 'COMUN.PROX_AUDIENCIAS' in row.rules
    assert not row.warnings


def test_tres_audiencias_se_enumeran_en_orden(tmp_path):
    row = _fila(tmp_path, '28/12/2026; 05/01/2027; 10/02/2027')
    assert (
        'Audiencias fijadas para el 28 de diciembre de 2026, el 5 de enero de 2027 y el 10 de febrero de 2027.'
    ) in row.observation


def test_audiencias_pasadas_se_descartan(tmp_path):
    row = _fila(tmp_path, '01/10/2026, 28/12/2026')
    assert 'Audiencia fijada para el 28 de diciembre de 2026.' in row.observation
    assert 'Audiencias fijadas' not in row.observation
    assert 'Audiencia' not in _fila(tmp_path, '01/10/2026, 02/10/2026').observation


def test_fecha_ilegible_queda_como_incidencia_y_no_oculta_las_legibles(tmp_path):
    row = _fila(tmp_path, '28/12/2026, por fijar')
    assert 'Audiencia fijada para el 28 de diciembre de 2026.' in row.observation
    assert 'próxima audiencia con fecha ilegible (G-05)' in row.warnings


def test_desactivar_audiencia_desactiva_tambien_el_plural(tmp_path):
    config = defaults()
    config['desactivadas'] = ['COMUN.PROX_AUDIENCIA']
    assert 'Audiencia' not in _fila(tmp_path, '28/12/2026, 05/01/2027', config).observation


@pytest.mark.parametrize(
    'valor, esperado, ilegibles',
    [
        (None, 0, 0),
        ('', 0, 0),
        ('-', 0, 0),
        ('28/12/2026', 1, 0),
        ('28/12/2026,05/01/2027', 2, 0),
        ('28/12/2026 , x', 1, 1),
    ],
)
def test_fechas_varias(valor, esperado, ilegibles):
    fechas, malas = fechas_varias(valor)
    assert (len(fechas), malas) == (esperado, ilegibles)


def test_lista_fechas_es():
    assert lista_fechas_es([date(2026, 12, 28)]) == 'el 28 de diciembre de 2026'
    assert lista_fechas_es([date(2026, 12, 28), date(2027, 1, 5)]) == (
        'el 28 de diciembre de 2026 y el 5 de enero de 2027'
    )
