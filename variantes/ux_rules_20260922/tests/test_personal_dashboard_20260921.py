from copy import deepcopy
from types import SimpleNamespace
from pathlib import Path

from nurus.personal.app import App
from nurus.personal.config import Configuration, defaults, LEGACY_CORREO_BODIES
from nurus.personal.outputs import prepare_drafts, prepare_required_drafts
from test_personal_flow import exported


def test_program_batch_omits_court_mail_and_keeps_mandatory_cc(tmp_path):
    work=exported(tmp_path)
    work.config['contactos']['AFT EJEMPLO']='programa@example.cl'
    drafts=prepare_required_drafts(work,modalities='todas las modalidades',recipient_scope='programas')
    assert len(drafts)==1
    assert drafts[0].to=='programa@example.cl'
    assert drafts[0].kind=='programa_espera'
    assert drafts[0].cc=='ucc_concepcion@pjud.cl'
    assert Path(drafts[0].attachments[0]).name=='AFT EJEMPLO.xlsx'


def test_court_batch_excludes_program_mails(tmp_path):
    work=exported(tmp_path)
    drafts=prepare_required_drafts(work,modalities='todas las modalidades',recipient_scope='tribunales')
    assert len(drafts)==1 and drafts[0].kind=='espera'
    assert drafts[0].program==''


def test_combined_batch_retains_both_destinations(tmp_path):
    work=exported(tmp_path)
    drafts=prepare_required_drafts(work,modalities='todas las modalidades',recipient_scope='todos')
    assert {d.kind for d in drafts}=={'espera','programa_espera'}


def test_program_mail_does_not_require_court_filter():
    work=SimpleNamespace(rows=[SimpleNamespace(id='a'),SimpleNamespace(id='b')])
    app=SimpleNamespace(_selected_mail_courts=lambda:[])
    assert App._mail_selected_ids(app,work)==['a','b']


def test_manual_template_can_be_addressed_to_program(tmp_path):
    work=exported(tmp_path)
    work.config['contactos']['AFT EJEMPLO']='programa@example.cl'
    drafts=prepare_drafts(work,'especial',recipient_scope='programas',manual_selection=True)
    assert drafts and all(d.to=='programa@example.cl' for d in drafts)


def test_missing_program_address_still_prepares_editable_draft(tmp_path):
    work=exported(tmp_path);work.config['contactos']={}
    draft=prepare_required_drafts(work,modalities='todas las modalidades',recipient_scope='programas')[0]
    assert draft.to=='' and draft.record_ids==[work.rows[0].id]
    assert draft.program=='AFT EJEMPLO' and draft.court=='Jgdo. L. y G. de Laja'


def test_manual_text_migrates_old_default_even_if_revision_was_two(tmp_path):
    cfg=Configuration(tmp_path/'cfg');old=deepcopy(cfg.data);old['revision_correos']=2
    old['correos']['plantillas']['programa_por_vencer']['cuerpo']=LEGACY_CORREO_BODIES['programa_por_vencer']
    cfg.save(old);new=Configuration(cfg.directory)
    assert 'se envía planilla con RIT de causas en las cuales, informes de avances se encuentran pronto a vencer. Se ruega acusar recibo de la información.' in new.data['correos']['plantillas']['programa_por_vencer']['cuerpo']
    assert cfg.path.with_suffix('.bak').is_file()


def test_manual_text_migration_preserves_custom_body(tmp_path):
    cfg=Configuration(tmp_path/'cfg');old=deepcopy(cfg.data);old['revision_correos']=2
    old['correos']['plantillas']['programa_por_vencer']['cuerpo']='Texto personal autorizado.'
    cfg.save(old)
    assert Configuration(cfg.directory).data['correos']['plantillas']['programa_por_vencer']['cuerpo']=='Texto personal autorizado.'


def test_destination_filter_preserves_edited_drafts_when_switching_back():
    from nurus.personal.outputs import Draft, drafts_for_scope
    program=Draft('A programa','Texto editado',recipient_type='programas')
    court=Draft('A tribunal','Otro texto',recipient_type='tribunales')
    source=[program,court]
    assert drafts_for_scope(source,'programas')==[program]
    drafts_for_scope(source,'programas')[0].body='Nueva edición'
    assert drafts_for_scope(source,'tribunales')==[court]
    assert drafts_for_scope(source,'todos')[0].body=='Nueva edición'


def test_program_scope_never_repurposes_court_measures_mail(tmp_path):
    work=exported(tmp_path,'CUMPLIMIENTO')
    program_drafts=prepare_required_drafts(
        work,modalities='todas las modalidades',recipient_scope='programas'
    )
    court_drafts=prepare_required_drafts(
        work,modalities='todas las modalidades',recipient_scope='tribunales'
    )
    assert all(d.kind.startswith('programa_') for d in program_drafts)
    assert {d.kind for d in court_drafts}=={'cumplimiento','medidas'}
    assert all(d.recipient_type=='tribunales' for d in court_drafts)
