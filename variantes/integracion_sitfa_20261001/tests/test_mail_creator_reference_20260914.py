from types import SimpleNamespace

from nurus.personal.mail_controls import alcance_modalidades, lista_espanol, selected_court_record_ids


def test_modalities_scope_matches_creator_semantics():
    assert alcance_modalidades(['RES']) == 'la modalidad Cuidado alternativo residencial'
    assert alcance_modalidades(['RES','AMB']) == 'las modalidades Cuidado alternativo residencial e Intervención ambulatoria de reparación'
    assert alcance_modalidades(['RES','AMB','FAE','DCE']) == 'todas las modalidades'
    assert alcance_modalidades([]) == ''


def test_spanish_list_uses_e_before_i_sound():
    assert lista_espanol(['Residencial','Intervención']) == 'Residencial e Intervención'
    assert lista_espanol(['Residencial','DCE']) == 'Residencial y DCE'


def test_selected_courts_filter_work_records():
    rows=[
        SimpleNamespace(id='a',values={'TRIBUNAL':'Juzgado de Familia de Tomé'}),
        SimpleNamespace(id='b',values={'TRIBUNAL':'Jgdo. L. y G. de Laja'}),
        SimpleNamespace(id='c',values={'TRIBUNAL':'Jgdo. L. y G. de Mulchén'}),
    ]
    work=SimpleNamespace(rows=rows,mapping={'tribunal':'TRIBUNAL'})
    assert selected_court_record_ids(work,['TOME','MULCHEN']) == ['a','c']
    assert selected_court_record_ids(work,[]) == []
