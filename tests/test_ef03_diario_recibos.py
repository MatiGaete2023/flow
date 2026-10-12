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
    assert len(full) == 2  # uno previo al primer efecto (E3 #1) y uno al final; nunca por borrador
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


def test_bin_danado_impide_el_primer_efecto_de_outlook(tmp_path, monkeypatch):
    # E3 WP-92 #1: el guardado completo previo al lote verifica la copia de recuperación.
    work, folder = _session(tmp_path)
    (folder / (work.source_hash + '.bin')).write_bytes(b'copia ficticia alterada')
    calls = []
    monkeypatch.setattr(outputs, 'save_draft', lambda *a, **k: calls.append(1))
    with pytest.raises(ValueError, match='recuperación está dañado'):
        create_drafts(work, _drafts(2))
    assert calls == []


def test_ediciones_en_memoria_quedan_en_disco_antes_del_lote(tmp_path, monkeypatch):
    work, folder = _session(tmp_path)
    work.rows[0].review['OBSERVACION'] = 'Edición ficticia sin autoguardado.'

    def cut(*a, **k):
        raise KeyboardInterrupt  # corte durante el primer efecto de Outlook

    monkeypatch.setattr(outputs, 'save_draft', cut)
    with pytest.raises(KeyboardInterrupt):
        create_drafts(work, _drafts(1))
    assert Work.load(folder, work.config).rows[0].review['OBSERVACION'] == 'Edición ficticia sin autoguardado.'


def test_error_no_os_en_el_guardado_final_se_informa_sin_perder_el_resumen(tmp_path, monkeypatch):
    work, folder = _session(tmp_path)
    monkeypatch.setattr(outputs, 'save_draft', lambda *a, **k: SimpleNamespace(entry_id='e', store_id='s'))
    original = Work.save
    calls = []

    def save(self, *a, **k):
        calls.append(1)
        if len(calls) > 1:  # el guardado inicial pasa; el final falla
            raise ValueError('falla ficticia')
        return original(self, *a, **k)

    monkeypatch.setattr(Work, 'save', save)
    result = create_drafts(work, _drafts(2))
    assert result['created'] == 2 and any('diario' in e for e in result['errors'])
    monkeypatch.setattr(Work, 'save', original)
    loaded = Work.load(folder, work.config)
    assert sorted(r['state'] for r in loaded.receipts.values() if r.get('kind') == 'draft') == ['created'] * 2


def test_lote_sin_cambios_netos_no_deja_diario_residual(tmp_path, monkeypatch):
    # E3 WP-92 #2: todos fallan de forma no incierta y el guardado final sale por el atajo de huella.
    work, folder = _session(tmp_path)

    def fail(*a, **k):
        raise RuntimeError('Outlook ficticio no disponible')

    monkeypatch.setattr(outputs, 'save_draft', fail)
    create_drafts(work, _drafts(2))
    assert not (folder / RECEIPT_JOURNAL).exists()


def test_diario_bloqueado_no_hace_fallar_un_guardado_ya_escrito(tmp_path, monkeypatch):
    # E3 WP-92 #3: en Windows el unlink puede fallar; la trabajo.json nueva vuelve obsoleto el diario.
    work, folder = _session(tmp_path)
    work.receipts['k'] = {'kind': 'draft', 'state': 'created'}
    work.save_receipts()
    original = Path.unlink

    def locked(self, *a, **k):
        if self.name == RECEIPT_JOURNAL:
            raise PermissionError('bloqueo ficticio')
        return original(self, *a, **k)

    monkeypatch.setattr(Path, 'unlink', locked)
    work.receipts['k2'] = {'kind': 'draft', 'state': 'created'}
    work.save(folder)
    monkeypatch.setattr(Path, 'unlink', original)
    assert set(Work.load(folder, work.config).receipts) >= {'k', 'k2'}


@pytest.mark.parametrize('damage', ['permiso', 'truncado', 'estructura'])
def test_diario_existente_ilegible_bloquea_repetir_borradores(tmp_path, monkeypatch, damage):
    # Revisión externa H2: un diario que existe pero no se puede leer no equivale a «sin diario».
    work, folder = _session(tmp_path)
    calls = []
    monkeypatch.setattr(
        outputs, 'save_draft', lambda *a, **k: calls.append(1) or SimpleNamespace(entry_id='e', store_id='s')
    )
    token = outputs._RECEIPT_JOURNAL.set(True)  # corte: el recibo solo quedó en el diario
    try:
        create_draft(work, _drafts(1)[0], confirmed=True)
    finally:
        outputs._RECEIPT_JOURNAL.reset(token)
    journal = folder / RECEIPT_JOURNAL
    if damage == 'truncado':
        journal.write_text(journal.read_text(encoding='utf-8')[:20], encoding='utf-8')
    elif damage == 'estructura':
        journal.write_text('{"version": 1, "receipts": []}', encoding='utf-8')
    original = Path.read_text

    def locked(self, *a, **k):
        if damage == 'permiso' and self.name == RECEIPT_JOURNAL:
            raise PermissionError('bloqueo ficticio')
        return original(self, *a, **k)

    monkeypatch.setattr(Path, 'read_text', locked)
    recovered = Work.load(folder, work.config)
    with pytest.raises(ValueError, match='diario de recibos'):
        create_drafts(recovered, _drafts(1))
    with pytest.raises(ValueError, match='diario de recibos'):
        create_draft(recovered, _drafts(1)[0], confirmed=True)
    recovered.save(folder)  # un guardado completo no destruye la evidencia
    assert journal.exists() and calls == [1]


def test_diario_obsoleto_valido_sigue_ignorandose_sin_bloquear(tmp_path, monkeypatch):
    work, folder = _session(tmp_path)
    (folder / RECEIPT_JOURNAL).write_text(
        json.dumps({'version': 1, 'base': '0' * 64, 'receipts': {}}), encoding='utf-8'
    )
    loaded = Work.load(folder, work.config)
    monkeypatch.setattr(outputs, 'save_draft', lambda *a, **k: SimpleNamespace(entry_id='e', store_id='s'))
    assert create_drafts(loaded, _drafts(1))['created'] == 1


def test_otra_instancia_no_borra_un_diario_ilegible(tmp_path, monkeypatch):
    # E3 WP-94 C1: guardar otro Work en la misma carpeta no destruye la evidencia ni levanta el bloqueo.
    work, folder = _session(tmp_path)
    monkeypatch.setattr(outputs, 'save_draft', lambda *a, **k: SimpleNamespace(entry_id='e', store_id='s'))
    token = outputs._RECEIPT_JOURNAL.set(True)
    try:
        create_draft(work, _drafts(1)[0], confirmed=True)
    finally:
        outputs._RECEIPT_JOURNAL.reset(token)
    journal = folder / RECEIPT_JOURNAL
    journal.write_text('{"truncado', encoding='utf-8')
    (tmp_path / 'otra').mkdir()
    other = exported(tmp_path / 'otra')
    other.save(folder)
    assert journal.exists()
    with pytest.raises(ValueError, match='diario de recibos'):
        create_drafts(other, _drafts(1))
    with pytest.raises(ValueError, match='diario de recibos'):
        create_drafts(Work.load(folder, work.config), _drafts(1))


@pytest.mark.parametrize(
    'contenido',
    [
        {'version': 1, 'base': None, 'receipts': {}},
        {'version': 1, 'receipts': {}},
        {'version': 1, 'base': '0' * 64, 'receipts': {'k': 'no es un recibo'}},
    ],
)
def test_diario_con_base_o_recibos_invalidos_bloquea(tmp_path, contenido):
    # E3 WP-94 C5.
    work, folder = _session(tmp_path)
    (folder / RECEIPT_JOURNAL).write_text(json.dumps(contenido), encoding='utf-8')
    with pytest.raises(ValueError, match='diario de recibos'):
        create_drafts(Work.load(folder, work.config), _drafts(1))
