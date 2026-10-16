from copy import deepcopy

from nurus.personal.config import Configuration, CORREO_REVISION


def test_email_migration_restores_missing_template_without_overwriting_custom_text(tmp_path):
    conf = Configuration(tmp_path / 'config')
    cfg = deepcopy(conf.data)
    cfg['revision_correos'] = CORREO_REVISION - 1
    cfg['correos']['plantillas'].pop('programa_por_vencer')
    cfg['correos']['plantillas']['programa_espera']['cuerpo'] = 'PLANTILLA PERSONALIZADA DEL USUARIO'
    conf.save(cfg)

    migrated = Configuration(conf.directory)

    assert migrated.data['revision_correos'] == CORREO_REVISION
    assert 'programa_por_vencer' in migrated.data['correos']['plantillas']
    assert migrated.data['correos']['plantillas']['programa_por_vencer']['adjunto'] == 'obligatorio'
    assert migrated.data['correos']['plantillas']['programa_espera']['cuerpo'] == 'PLANTILLA PERSONALIZADA DEL USUARIO'


def test_email_migration_is_idempotent_after_repair(tmp_path):
    conf = Configuration(tmp_path / 'config')
    cfg = deepcopy(conf.data)
    cfg['revision_correos'] = CORREO_REVISION - 1
    cfg['correos']['plantillas'].pop('programa_vencido')
    conf.save(cfg)

    first = Configuration(conf.directory)
    snapshot = deepcopy(first.data['correos']['plantillas'])
    second = Configuration(conf.directory)

    assert second.data['correos']['plantillas'] == snapshot


def test_missing_text_key_is_added_from_base_without_touching_others(tmp_path, monkeypatch):
    import json
    from nurus.personal import config as config_module

    monkeypatch.setattr(config_module, 'user_directory', lambda: tmp_path)
    first = config_module.Configuration(tmp_path)
    data = json.loads(first.path.read_text(encoding='utf-8'))
    scope = next(s for s in data['textos'] if not s.startswith('_'))
    removed, kept = list(data['textos'][scope])[:2]
    base_text = data['textos'][scope][removed]['texto']
    del data['textos'][scope][removed]
    data['textos'][scope][kept]['texto'] = data['textos'][scope][kept]['texto'] + ' '
    first.path.write_text(json.dumps(data, ensure_ascii=False), encoding='utf-8')
    loaded = config_module.Configuration(tmp_path)
    assert loaded.data['textos'][scope][removed]['texto'] == base_text
    assert loaded.data['textos'][scope][kept]['texto'] == data['textos'][scope][kept]['texto']


def test_rev1_texts_reach_current_base_through_rev2(tmp_path, monkeypatch):
    # F7 (revisión integral): un texto rev1 («se sugiere remitir…») de una clave que cambió en rev3 llega al texto vigente.
    import json
    from nurus.personal import config as config_module
    from nurus.personal.config import LEGACY_TEXTS_REV2, defaults

    monkeypatch.setattr(config_module, 'user_directory', lambda: tmp_path)
    first = config_module.Configuration(tmp_path)
    data = json.loads(first.path.read_text(encoding='utf-8'))
    scope, key = 'ESPERA', 'E05_SOLO_CORREO'
    rev2 = LEGACY_TEXTS_REV2[scope + '.' + key]
    rev1 = rev2.replace('Se remite correo electrónico', 'Se sugiere remitir correo electrónico').replace(
        'se remite correo electrónico', 'se sugiere remitir correo electrónico'
    )
    assert rev1 != rev2
    data['textos'][scope][key]['texto'] = rev1
    data['revision_textos'] = 1
    first.path.write_text(json.dumps(data, ensure_ascii=False), encoding='utf-8')
    loaded = config_module.Configuration(tmp_path)
    assert loaded.data['textos'][scope][key]['texto'] == defaults()['textos'][scope][key]['texto']
