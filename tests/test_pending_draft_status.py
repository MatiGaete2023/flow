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


@pytest.mark.parametrize('edit', ['subject', 'body', 'attachment'])
def test_row_indicator_does_not_reuse_saved_key_after_draft_edit(work, tmp_path, edit):
    from nurus.personal.ux_support import product_indicators
    from nurus.personal.work import Row

    row = Row('r1', 2, {}, '', [], [], [], False, {})
    attachment = tmp_path / 'ficticio.txt'
    attachment.write_text('antes', encoding='utf-8')
    draft = Draft(
        'Ficticio', 'Contenido ficticio', to='a@example.test', record_ids=[row.id], attachments=[str(attachment)]
    )
    draft.key = draft_fingerprint(draft)
    work.receipts[draft.key] = {'kind': 'draft', 'state': 'created'}
    assert 'Correo ✓' in product_indicators(work, row, [draft])
    if edit == 'attachment':
        attachment.write_text('otros', encoding='utf-8')
    else:
        setattr(draft, edit, getattr(draft, edit) + ' editado')
    assert 'Correo ✓' not in product_indicators(work, row, [draft])
    assert 'Correo preparado' in product_indicators(work, row, [draft])


def test_row_indicator_stays_prepared_after_recipient_becomes_invalid(work):
    from nurus.personal.ux_support import product_indicators
    from nurus.personal.work import Row

    row = Row('r1', 2, {}, '', [], [], [], False, {})
    draft = Draft('Ficticio', 'Contenido ficticio', to='a@example.test', record_ids=[row.id])
    draft.key = draft_fingerprint(draft)
    work.receipts[draft.key] = {'kind': 'draft', 'state': 'created'}
    draft.to = 'correo inválido'
    indicators = product_indicators(work, row, [draft])
    assert 'Correo ✓' not in indicators
    assert 'Correo preparado' in indicators


def test_row_selection_does_not_rehash_unchanged_attachments(tmp_path, monkeypatch):
    # C1 (revisión integral): recorrer la tabla no relee los adjuntos de cada borrador en cada selección.
    from pathlib import Path
    from types import SimpleNamespace
    from nurus.personal.outputs import Draft, draft_fingerprint
    from nurus.personal.ux_support import product_indicators
    from nurus.personal.work import Row

    adjunto = tmp_path / 'nomina.xlsx'
    adjunto.write_bytes(b'x' * 200_000)
    fila = Row('r1', 2, {}, '', [], [], [], False, {})
    borradores = [
        Draft(f'S{i}', 'b', to='a@example.test', record_ids=['r1'], attachments=[str(adjunto)]) for i in range(5)
    ]
    for borrador in borradores:
        borrador.key = draft_fingerprint(borrador)
    work = SimpleNamespace(receipts={borradores[0].key: {'state': 'created'}}, output='')
    lecturas = []
    original = Path.read_bytes
    monkeypatch.setattr(Path, 'read_bytes', lambda p: (lecturas.append(p), original(p))[1])
    for _ in range(10):
        product_indicators(work, fila, borradores)
    assert len(lecturas) <= 1
    from nurus.personal.ux_support import _mail_state

    assert _mail_state(work, 'r1', borradores) == 'created'
    # Un adjunto editado (otro tamaño y mtime) cambia la huella mostrada: ya no figura como guardado.
    adjunto.write_bytes(b'y' * 200_001)
    assert _mail_state(work, 'r1', borradores) == 'prepared'
