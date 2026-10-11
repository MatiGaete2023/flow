"""Lecturas acotadas por consulta, con identidad fresca antes de guardar."""

from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from types import SimpleNamespace
import os

import pytest

from nurus.personal import outputs
from nurus.personal.outputs import Draft, draft_fingerprint, draft_fingerprint_for_display, draft_preview_scope
from nurus.personal.product_checks import check_products
from test_integral_ui import run_gui


def sample(tmp_path):
    path = tmp_path / 'ficticio.txt'
    path.write_bytes(b'antes')
    draft = Draft('Ficticio', 'Contenido ficticio', to='a@example.test', attachments=[str(path)])
    return path, draft


def count_reads(monkeypatch, target):
    reads = []
    original = Path.read_bytes

    def read(path):
        if path == target:
            reads.append(path)
        return original(path)

    monkeypatch.setattr(Path, 'read_bytes', read)
    return reads


def test_one_attachment_read_for_a_hundred_drafts_per_preflight(tmp_path, monkeypatch):
    path, draft = sample(tmp_path)
    work = SimpleNamespace(
        rows=[], receipts={draft_fingerprint(draft): {'state': 'created'}}, output='', mapping={}, config={}
    )
    drafts = [Draft(str(i), draft.body, to=draft.to, attachments=[str(path)]) for i in range(100)]
    reads = count_reads(monkeypatch, path)
    checks = check_products(work, drafts, [])
    assert len(checks) == 100
    assert len(reads) == 1
    check_products(work, drafts, [])
    assert len(reads) == 2  # una consulta posterior siempre vuelve a leer


def test_nested_scopes_share_only_attachment_digests(tmp_path, monkeypatch):
    path, draft = sample(tmp_path)
    reads = count_reads(monkeypatch, path)
    with draft_preview_scope():
        before = draft_fingerprint_for_display(draft)
        with draft_preview_scope():
            draft.subject += ' editado'
            assert draft_fingerprint_for_display(draft) != before
        draft.body += ' editado'
        assert draft_fingerprint_for_display(draft) != before
    assert len(reads) == 1
    assert outputs._PREVIEW_DIGESTS.get() is None


def test_equal_size_and_mtime_edit_is_seen_by_next_query(tmp_path):
    path, draft = sample(tmp_path)
    stat = path.stat()
    with draft_preview_scope():
        before = draft_fingerprint_for_display(draft)
    path.write_bytes(b'otros')
    os.utime(path, ns=(stat.st_atime_ns, stat.st_mtime_ns))
    assert path.stat().st_size == stat.st_size
    assert path.stat().st_mtime_ns == stat.st_mtime_ns
    with draft_preview_scope():
        assert draft_fingerprint_for_display(draft) != before


def test_saving_identity_ignores_active_display_cache(tmp_path, monkeypatch):
    path, draft = sample(tmp_path)
    work = SimpleNamespace(rows=[], receipts={}, config={})
    products = []
    monkeypatch.setattr('nurus.personal.product_state.require_fresh', lambda *args: None)

    def save(product, **kwargs):
        products.append(product)
        return SimpleNamespace(entry_id='ENTRY-FICTICIA', store_id='STORE-FICTICIA')

    monkeypatch.setattr(outputs, 'save_draft', save)
    with draft_preview_scope():
        old = draft_fingerprint_for_display(draft)
        work.receipts[old] = {'state': 'created'}
        path.write_bytes(b'otros')
        outputs.create_draft(work, draft, confirmed=True)
        assert draft.key != old
        assert products[0].attachments[0][1] == outputs.sha256(b'otros').hexdigest()
        assert work.receipts[draft.key]['state'] == 'created'
        with pytest.raises(ValueError, match='ya se creó'):
            outputs.create_draft(work, draft, confirmed=True)
    assert len(products) == 1


def test_display_without_scope_always_reads_current_bytes(tmp_path):
    path, draft = sample(tmp_path)
    before = draft_fingerprint_for_display(draft)
    path.write_bytes(b'otros')
    assert draft_fingerprint_for_display(draft) != before
    assert draft_fingerprint(draft) == draft_fingerprint_for_display(draft)


def test_failed_read_does_not_leak_scope_into_next_query(tmp_path, monkeypatch):
    path, draft = sample(tmp_path)
    original = Path.read_bytes

    def fail(target):
        if target == path:
            raise OSError('fallo ficticio')
        return original(target)

    with monkeypatch.context() as patch:
        patch.setattr(Path, 'read_bytes', fail)
        with pytest.raises(OSError, match='fallo ficticio'), draft_preview_scope():
            draft_fingerprint_for_display(draft)
    assert outputs._PREVIEW_DIGESTS.get() is None
    assert draft_fingerprint_for_display(draft) == draft_fingerprint(draft)


def test_worker_thread_cannot_reuse_tk_query_cache(tmp_path):
    path, draft = sample(tmp_path)
    with draft_preview_scope():
        old = draft_fingerprint_for_display(draft)
        path.write_bytes(b'otros')
        with ThreadPoolExecutor(max_workers=1) as executor:
            assert executor.submit(draft_fingerprint_for_display, draft).result() != old


@pytest.mark.parametrize('has_receipt', [True, False])
def test_actual_mail_cards_read_shared_attachment_once_per_redraw(tmp_path, has_receipt):
    run_gui(
        tmp_path,
        '''
from types import SimpleNamespace
from unittest.mock import patch
from nurus.personal import outputs
from nurus.personal.outputs import Draft, draft_fingerprint
from nurus.personal.mail_view import DraftCards
attachment=root/'ficticio.txt';attachment.write_bytes(b'antes')
drafts=[Draft('Ficticio '+str(i),'Texto ficticio',to='a@example.test',attachments=[str(attachment)]) for i in range(20)]
receipts={draft_fingerprint(drafts[0]): {'state':'created'}} if HAS_RECEIPT else {}
cards=DraftCards(app)
original=Path.read_bytes;reads=[]
def read(path):
    if path==attachment:reads.append(path)
    return original(path)
with patch.object(Path,'read_bytes',read):
    cards.set_drafts(drafts,receipts)
    assert len(reads)==EXPECTED,len(reads)
    assert outputs._PREVIEW_DIGESTS.get() is None
    attachment.write_bytes(b'otros')
    cards.set_drafts(drafts,receipts)
    assert len(reads)==2*EXPECTED,len(reads)
    assert outputs._PREVIEW_DIGESTS.get() is None
cards.destroy()
'''.replace('HAS_RECEIPT', str(has_receipt)).replace('EXPECTED', '1' if has_receipt else '0'),
    )
