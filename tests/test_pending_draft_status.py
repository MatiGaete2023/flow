"""Los pendientes conservan los mismos avisos de seguridad que Resultados."""

from types import SimpleNamespace

import pytest

from nurus.personal.current_tools import pending
from nurus.personal.outputs import Draft, draft_fingerprint
from nurus.personal.product_checks import check_products


@pytest.fixture
def work():
    return SimpleNamespace(rows=[], receipts={}, output='', mapping={}, config={})


@pytest.mark.parametrize('state', ['saving', 'uncertain'])
def test_uncertain_receipt_remains_an_actionable_outlook_check(work, state):
    draft = Draft('Ficticio', 'Contenido ficticio', to='a@example.test')
    work.receipts[draft_fingerprint(draft)] = {'kind': 'draft', 'state': state}
    checks = check_products(work, [draft], [])
    assert 'Comprobar en Outlook antes de repetir' in checks[0].issues
    assert ('Correo', draft.subject, 'Comprobar en Outlook antes de repetir') in pending(work, [draft], [])
    assert not any('guardar borrador' in task[2] for task in pending(work, [draft], []))


@pytest.mark.parametrize('address', ['correo inválido', 'a@example.test;mal'])
def test_invalid_recipient_becomes_a_pending_issue_instead_of_crashing(work, address):
    draft = Draft('Ficticio', 'Contenido ficticio', to=address)
    issues = check_products(work, [draft], [])[0].issues
    tasks = pending(work, [draft], [])
    assert issues
    assert all(any(task[2] == issue for task in tasks) for issue in issues)


def test_missing_attachment_remains_pending_even_with_matching_receipt(work, tmp_path):
    draft = Draft('Ficticio', 'Contenido ficticio', to='a@example.test', attachments=[str(tmp_path / 'ausente.txt')])
    work.receipts[draft_fingerprint(draft)] = {'kind': 'draft', 'state': 'created'}
    tasks = pending(work, [draft], [])
    assert any('Adjunto no disponible' in task[2] for task in tasks)


@pytest.mark.parametrize('field', ['subject', 'body'])
def test_incomplete_content_remains_pending(work, field):
    draft = Draft('Ficticio', 'Contenido ficticio', to='a@example.test')
    setattr(draft, field, '')
    issues = check_products(work, [draft], [])[0].issues
    tasks = pending(work, [draft], [])
    assert all(any(task[2] == issue for task in tasks) for issue in issues)


def test_confirmed_receipt_has_no_pending_outlook_action(work):
    draft = Draft('Ficticio', 'Contenido ficticio', to='a@example.test')
    work.receipts[draft_fingerprint(draft)] = {'kind': 'draft', 'state': 'created'}
    assert pending(work, [draft], []) == []


def test_draft_without_receipt_requires_review_and_explicit_save(work):
    draft = Draft('Ficticio', 'Contenido ficticio', to='a@example.test')
    assert ('Correo', draft.subject, 'Revisar y guardar borrador') in pending(work, [draft], [])


def test_no_receipts_do_not_require_reading_attachment_bytes(work, tmp_path, monkeypatch):
    import nurus.personal.product_checks as checks

    attachment = tmp_path / 'ficticio.txt'
    attachment.write_text('contenido ficticio', encoding='utf-8')
    draft = Draft('Ficticio', 'Contenido ficticio', to='a@example.test', attachments=[str(attachment)])
    monkeypatch.setattr(checks, 'draft_fingerprint', lambda draft: pytest.fail('No hay recibos que buscar'))
    assert ('Correo', draft.subject, 'Revisar y guardar borrador') in pending(work, [draft], [])
