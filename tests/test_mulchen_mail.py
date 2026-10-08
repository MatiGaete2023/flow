from copy import deepcopy
from types import SimpleNamespace

import pytest
from openpyxl import Workbook

from nurus.personal.config import defaults
from nurus.personal.courts import court_contact
from nurus.personal.mail_controls import selected_court_record_ids
from nurus.personal.outputs import prepare_drafts, prepare_required_drafts
from nurus.personal.work import Work


VARIANTS = [
    'Mulchén', 'MULCHEN', 'MULCHËN', 'mulche\u0301n',
    'Juzgado de Letras y Garantía de Mulchén',
    'Jgdo. L. y G. de Mulchén                          ',
    'MULCHËN — Garantía y Letras, Juzgado',
]


@pytest.mark.parametrize('selected', VARIANTS)
def test_court_filter_normalizes_selection_as_well_as_excel_values(selected):
    rows = [SimpleNamespace(id=str(i), values={'TRIBUNAL': name})
            for i, name in enumerate(VARIANTS + ['LAJA', 'TOME'])]
    work = SimpleNamespace(rows=rows, mapping={'tribunal': 'TRIBUNAL'})
    assert selected_court_record_ids(work, [selected]) == [str(i) for i in range(len(VARIANTS))]
    rows[-1].overrides = {'tribunal': 'MULCHËN'}
    assert rows[-1].id in selected_court_record_ids(work, [selected])


@pytest.mark.parametrize('config_key', VARIANTS)
@pytest.mark.parametrize('manual', [False, True])
def test_excel_variants_make_one_addressed_draft_for_automatic_and_manual_selection(tmp_path, config_key, manual):
    book = Workbook()
    book.active.append(['RIT', 'TRIBUNAL', 'NOMBRE', 'DERIVACION', 'OBSERVACION'])
    for i, name in enumerate(VARIANTS + ['LAJA']):
        book.active.append([f'X-{i}', name, 'Persona de prueba', 'AFT PRUEBA', 'Revisado'])
    path = tmp_path/'revisado.xlsx'
    book.save(path)
    config = defaults()
    contact = config['correos']['tribunales'].pop('MULCHEN')
    contact['para'] = ['destino-personalizado@example.test']
    config['correos']['tribunales'][config_key] = contact
    work = Work.external(path, config, 'CUMPLIMIENTO')
    ids = selected_court_record_ids(work, [config_key])
    options = dict(selected=ids, modalities='todas las modalidades', recipient_scope='tribunales')
    drafts = prepare_drafts(work, 'cumplimiento', manual_selection=manual, **options)
    assert len(drafts) == 1
    assert drafts[0].record_ids == ids
    assert len(ids) == len(VARIANTS)
    assert drafts[0].to == 'destino-personalizado@example.test'
    assert drafts[0].court == contact['nombre']
    automatic = prepare_required_drafts(work, **options)
    assert len(automatic) == 1
    assert automatic[0].record_ids == ids
    assert automatic[0].to == drafts[0].to


def test_court_lookup_preserves_configuration_and_does_not_guess_unknown_recipient():
    config = defaults()
    before = deepcopy(config)
    for name in VARIANTS:
        assert court_contact(config, name) == config['correos']['tribunales']['MULCHEN']
    assert court_contact(config, 'Otro tribunal') == {'nombre': 'Otro tribunal', 'para': []}
    assert config == before


def test_conflicting_legacy_aliases_require_resolution():
    config = defaults()
    contact = config['correos']['tribunales'].pop('MULCHEN')
    config['correos']['tribunales']['Mulchén'] = contact
    config['correos']['tribunales']['MULCHËN'] = {**contact, 'para': ['otro@example.test']}
    with pytest.raises(ValueError, match='contradictorios'):
        court_contact(config, 'Mulchen')
