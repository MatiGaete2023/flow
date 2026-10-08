from types import SimpleNamespace

import pytest
from openpyxl import Workbook

from nurus.personal.config import defaults
from nurus.personal.mail_controls import selected_court_record_ids
from nurus.personal.modalities import MODALITIES, modality, selected_row
from nurus.personal.outputs import prepare_drafts, prepare_required_drafts
from nurus.personal.work import Work


def residential_work(tmp_path):
    book = Workbook()
    book.active.append(['RIT', 'TRIBUNAL', 'NOMBRE', 'DERIVACION', 'OBSERVACION'])
    for i, code in enumerate(['RTA', 'RTA', 'RTT', 'RTT', 'RTT', 'RTT', 'RTT']):
        book.active.append([f'X-{i}', 'Jgdo. L. y G. de Mulchén', 'Persona de prueba',
                            code + ' - Residencia de prueba', 'Revisado'])
    book.active.append(['X-8', 'LAJA', 'Otra persona', 'REM - Residencia de prueba', 'Revisado'])
    book.active.append(['X-9', 'MULCHEN', 'Otra persona', 'AFT - Programa de prueba', 'Revisado'])
    path = tmp_path/'residencial.xlsx'
    book.save(path)
    return Work.external(path, defaults(), 'CUMPLIMIENTO')


@pytest.mark.parametrize('manual', [False, True])
def test_mulchen_residential_rows_prepare_one_draft_with_res_filter(tmp_path, manual):
    work = residential_work(tmp_path)
    ids = selected_court_record_ids(work, ['MULCHEN'])
    options = dict(selected=ids, modalities=MODALITIES['RES'], modality_keys=['RES'],
                   recipient_scope='tribunales')
    drafts = prepare_drafts(work, 'cumplimiento', manual_selection=manual, **options)
    assert len(drafts) == 1
    assert drafts[0].record_ids == [row.id for row in work.rows[:7]]
    assert drafts[0].to == '; '.join(work.config['correos']['tribunales']['MULCHEN']['para'])
    automatic = prepare_required_drafts(work, **options)
    assert len(automatic) == 1
    assert automatic[0].record_ids == drafts[0].record_ids


@pytest.mark.parametrize('name', ['RTA - Casa de prueba', 'rtt residencia', 'RVA CASA DE PRUEBA',
                                  'REM - Residencia', 'RFA - Residencia', MODALITIES['RES']])
def test_residential_modalities(name):
    assert modality(name) == 'RES'


@pytest.mark.parametrize('key', list(MODALITIES))
def test_explicit_modality_label_is_recognized(key):
    work = SimpleNamespace(mapping={'programa': 'DERIVACION'})
    row = SimpleNamespace(values={'MODALIDAD': MODALITIES[key], 'DERIVACION': 'Nombre sin código'})
    assert selected_row(work, row, [key])
    assert not selected_row(work, row, [other for other in MODALITIES if other != key])


def test_corrected_program_is_used_by_modality_filter():
    work = SimpleNamespace(mapping={'programa': 'DERIVACION'})
    row = SimpleNamespace(values={'DERIVACION': 'AFT - Prueba'}, overrides={'programa': 'RTT - Prueba'})
    assert selected_row(work, row, ['RES'])
    assert not selected_row(work, row, ['AMB'])


@pytest.mark.parametrize('program', ['RZZ - Desconocido', 'Nombre sin código', 'AFT - Residencia', 'FAE - Prueba', 'DCE - Prueba'])
def test_residential_filter_does_not_include_other_or_unknown_modalities(program):
    assert modality(program) != 'RES'
