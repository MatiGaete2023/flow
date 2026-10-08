from copy import deepcopy

from nurus.personal.config import Configuration, CORREO_REVISION


def test_email_migration_restores_missing_template_without_overwriting_custom_text(tmp_path):
    conf=Configuration(tmp_path/'config')
    cfg=deepcopy(conf.data)
    cfg['revision_correos']=CORREO_REVISION-1
    cfg['correos']['plantillas'].pop('programa_por_vencer')
    cfg['correos']['plantillas']['programa_espera']['cuerpo']='PLANTILLA PERSONALIZADA DEL USUARIO'
    conf.save(cfg)

    migrated=Configuration(conf.directory)

    assert migrated.data['revision_correos']==CORREO_REVISION
    assert 'programa_por_vencer' in migrated.data['correos']['plantillas']
    assert migrated.data['correos']['plantillas']['programa_por_vencer']['adjunto']=='obligatorio'
    assert migrated.data['correos']['plantillas']['programa_espera']['cuerpo']=='PLANTILLA PERSONALIZADA DEL USUARIO'


def test_email_migration_is_idempotent_after_repair(tmp_path):
    conf=Configuration(tmp_path/'config')
    cfg=deepcopy(conf.data)
    cfg['revision_correos']=CORREO_REVISION-1
    cfg['correos']['plantillas'].pop('programa_vencido')
    conf.save(cfg)

    first=Configuration(conf.directory)
    snapshot=deepcopy(first.data['correos']['plantillas'])
    second=Configuration(conf.directory)

    assert second.data['correos']['plantillas']==snapshot
