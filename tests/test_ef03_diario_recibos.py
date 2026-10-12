"""EF-03: diario de recibos en lotes de borradores; persistencia antes/después de cada efecto de Outlook."""

import json
from pathlib import Path
from types import SimpleNamespace

import pytest

from test_personal_flow import exported
from nurus.personal import outputs
from nurus.personal.outputs import Draft, create_draft, create_drafts
from nurus.adapters.outlook import DraftSaveUncertain
from nurus.personal.work import RECEIPT_JOURNAL, Work


def _session(tmp_path):
    work = exported(tmp_path)
    work.save(tmp_path / 'sesion')
    return work, tmp_path / 'sesion'


def _drafts(n):
    return [Draft(f'Asunto ficticio {i}', 'Texto ficticio.') for i in range(n)]


def test_lote_guarda_sesion_completa_una_vez_y_recibos_en_diario(tmp_path, monkeypatch):
    work, folder = _session(tmp_path)
    full, journal = [], []
    original_save, original_receipts = Work.save, Work.save_receipts
    monkeypatch.setattr(Work, 'save', lambda self, *a, **k: full.append(1) or original_save(self, *a, **k))
    monkeypatch.setattr(Work, 'save_receipts', lambda self: journal.append(1) or original_receipts(self))
    monkeypatch.setattr(outputs, 'save_draft', lambda *a, **k: SimpleNamespace(entry_id='e', store_id='s'))
    result = create_drafts(work, _drafts(3))
    assert result['created'] == 3 and not result['errors']
    assert len(journal) == 6  # antes y después de cada efecto
    assert len(full) == 1  # un solo guardado completo al final
    assert not (folder / RECEIPT_JOURNAL).exists()
    loaded = Work.load(folder, work.config)
    assert sorted(r['state'] for r in loaded.receipts.values() if r.get('kind') == 'draft') == ['created'] * 3


@pytest.mark.parametrize('outcome', ['created', 'uncertain'])
def test_corte_despues_de_outlook_recupera_el_recibo_desde_el_diario(tmp_path, monkeypatch, outcome):
    work, folder = _session(tmp_path)
    before = (folder / 'trabajo.json').read_bytes()

    def save(*a, **k):
        if outcome == 'uncertain':
            raise DraftSaveUncertain('incierto ficticio')
        return SimpleNamespace(entry_id='e', store_id='s')

    monkeypatch.setattr(outputs, 'save_draft', save)
    token = outputs._RECEIPT_JOURNAL.set(True)  # dentro de un lote, sin llegar al guardado final (corte)
    try:
        if outcome == 'uncertain':
            with pytest.raises(DraftSaveUncertain):
                create_draft(work, _drafts(1)[0], confirmed=True)
        else:
            create_draft(work, _drafts(1)[0], confirmed=True)
    finally:
        outputs._RECEIPT_JOURNAL.reset(token)
    assert (folder / 'trabajo.json').read_bytes() == before  # sin guardado completo
    loaded = Work.load(folder, work.config)
    states = [r['state'] for r in loaded.receipts.values() if r.get('kind') == 'draft']
    assert states == [outcome]


def test_diario_de_otra_version_se_ignora(tmp_path):
    work, folder = _session(tmp_path)
    (folder / RECEIPT_JOURNAL).write_text(
        json.dumps({'version': 1, 'base': '0' * 64, 'receipts': {'x': {'kind': 'draft', 'state': 'created'}}}),
        encoding='utf-8',
    )
    assert 'x' not in Work.load(folder, work.config).receipts
    (folder / RECEIPT_JOURNAL).write_text('no es JSON', encoding='utf-8')
    assert 'x' not in Work.load(folder, work.config).receipts


def test_guardado_completo_retira_el_diario(tmp_path):
    work, folder = _session(tmp_path)
    work.receipts['k'] = {'kind': 'draft', 'state': 'saving'}
    work.save_receipts()
    assert (folder / RECEIPT_JOURNAL).is_file()
    work.save(folder)
    assert not (folder / RECEIPT_JOURNAL).exists()
    assert Work.load(folder, work.config).receipts['k']['state'] == 'saving'


def test_fallo_no_incierto_retira_el_recibo_tambien_del_diario(tmp_path, monkeypatch):
    work, folder = _session(tmp_path)

    def fail(*a, **k):
        raise RuntimeError('Outlook ficticio no disponible')

    monkeypatch.setattr(outputs, 'save_draft', fail)
    result = create_drafts(work, _drafts(1))
    assert result['created'] == 0 and len(result['errors']) == 1
    assert not [r for r in Work.load(folder, work.config).receipts.values() if r.get('kind') == 'draft']


def test_cada_adjunto_se_lee_una_vez_por_borrador(tmp_path, monkeypatch):
    work, _ = _session(tmp_path)
    attachment = tmp_path / 'nomina-ficticia.xlsx'
    attachment.write_bytes(b'contenido ficticio')
    reads = []
    original = Path.read_bytes

    def counting(self):
        if self.name == attachment.name:
            reads.append(1)
        return original(self)

    monkeypatch.setattr(Path, 'read_bytes', counting)
    monkeypatch.setattr(outputs, 'save_draft', lambda *a, **k: SimpleNamespace(entry_id='e', store_id='s'))
    draft = Draft('Con adjunto ficticio', 'Texto.', attachments=[str(attachment)])
    result = create_drafts(work, [draft])
    assert result['created'] == 1 and not result['errors']
    assert len(reads) == 1  # antes: 3 (clave del lote, clave del borrador y producto)
