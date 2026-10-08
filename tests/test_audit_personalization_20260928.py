import pytest

from nurus.personal.config import defaults
from nurus.personal.personalization import save_contact,duplicate_template,archive_template,restore_template


def test_rename_moves_all_aliases_and_does_not_leave_duplicate_contact():
    config=defaults();config['contactos']={'Anterior':'uno@example.cl','Otro':'otro@example.cl'}
    config['aliases']={'A':'Anterior','B':'Anterior','C':'Otro'}
    updated=save_contact(config,'Nuevo','nuevo@example.cl',['A','B','Nuevo alias'],previous='Anterior')
    assert 'Anterior' not in updated['contactos']
    assert updated['aliases']=={'A':'Nuevo','B':'Nuevo','Nuevo alias':'Nuevo','C':'Otro'}
    assert config['contactos']['Anterior']=='uno@example.cl'
    with pytest.raises(ValueError,match='alias'):
        save_contact(updated,'Nuevo','nuevo@example.cl',['C'],previous='Nuevo')


def test_templates_have_independent_copy_archive_and_restore():
    config=defaults()
    duplicated,key=duplicate_template(config,'espera','Copia')
    duplicated['correos']['plantillas'][key]['cuerpo']='Personalizado'
    assert config['correos']['plantillas']['espera']['cuerpo']!='Personalizado'
    archived=archive_template(duplicated,key)
    assert archived['correos']['plantillas'][key]['archivada']
    restored=restore_template(archive_template(config,'espera'),'espera')
    assert not restored['correos']['plantillas']['espera']['archivada']
