"""Una dirección escrita a mano para un programa sin contacto se guarda tras crear su borrador."""

import json
from types import SimpleNamespace

from nurus.personal.app_base import App as BaseApp
from nurus.personal.config import Configuration, contacts_file
from nurus.personal.outputs import Draft, contacts_notice, learned_contacts, resolve_contact

PROGRAMA = 'PRM Programa Ficticio Norte'


def borrador(to, program=PROGRAMA, recipient_type='programas'):
    return Draft('Asunto', 'Cuerpo', to=to, program=program, recipient_type=recipient_type)


def configuracion(tmp_path):
    cfg = Configuration(tmp_path / 'cfg', contacts_directory=tmp_path / 'contactos')
    assert resolve_contact(cfg.data, PROGRAMA) == ''
    return cfg


def test_programa_sin_contacto_aprende_la_direccion(tmp_path):
    cfg = configuracion(tmp_path)
    assert learned_contacts(cfg.data, [borrador('norte@programa.example')]) == {PROGRAMA: 'norte@programa.example'}


def test_no_reemplaza_contactos_ni_aprende_tribunales_ni_conflictos(tmp_path):
    cfg = configuracion(tmp_path)
    existente, direccion = next(iter(cfg.data['contactos'].items()))
    assert not learned_contacts(cfg.data, [borrador('otra@programa.example', program=existente.upper())])
    assert not learned_contacts(cfg.data, [borrador('t@tribunal.example', recipient_type='tribunales')])
    assert not learned_contacts(cfg.data, [borrador('')])
    assert not learned_contacts(cfg.data, [borrador('a@programa.example'), borrador('b@programa.example')])
    assert cfg.data['contactos'][existente] == direccion


def test_guardar_borrador_persiste_el_contacto_en_configuracion_y_directorio(tmp_path):
    cfg = configuracion(tmp_path)
    app = SimpleNamespace(cfg=cfg)
    learned = BaseApp._remember_program_contacts(app, [borrador('norte@programa.example; jefa@programa.example')])
    assert learned == {PROGRAMA: 'norte@programa.example; jefa@programa.example'}
    reopened = Configuration(tmp_path / 'cfg', contacts_directory=tmp_path / 'contactos')
    assert resolve_contact(reopened.data, PROGRAMA) == 'norte@programa.example; jefa@programa.example'
    stored = json.loads(contacts_file(tmp_path / 'contactos').read_text(encoding='utf-8'))
    assert {'Nombre': PROGRAMA, 'Mail': 'norte@programa.example; jefa@programa.example'} in stored
    # Los contactos ficticios de ejemplo no pasan al directorio real.
    assert {e['Nombre'] for e in stored if not e['Nombre'].startswith('Tribunal: ')} == {PROGRAMA}
    assert 'Contacto guardado' in contacts_notice(learned)
    assert BaseApp._remember_program_contacts(app, [borrador('norte@programa.example')]) == {}


def test_lote_solo_informa_los_borradores_creados_en_esta_ejecucion(monkeypatch):
    # Revisión Codex (PR #52): un recibo antiguo no debe hacer aprender la dirección editada de un borrador no guardado.
    from nurus.personal import outputs

    antiguo, nuevo = borrador('editado@programa.example'), borrador('nuevo@programa.example', program='PRM Otro')
    work = SimpleNamespace(receipts={'clave-antigua': {'state': 'created'}})
    monkeypatch.setattr(
        outputs, '_draft_fingerprint', lambda d, digests: 'clave-antigua' if d is antiguo else 'clave-nueva'
    )
    monkeypatch.setattr(outputs, 'create_draft', lambda work, draft, **kw: None)
    result = {'created': 0, 'skipped': 0, 'errors': [], 'processed': 0}

    class Cancelado(Exception):
        pass

    outputs._create_batch(work, [antiguo, nuevo], result, lambda: None, lambda *a: None, Cancelado)
    assert result['created_drafts'] == [nuevo] and result['skipped'] == 1
