from types import SimpleNamespace

from nurus.personal.attention import attention_items, case_statuses
from nurus.personal.work import Row


def test_bandeja_prioriza_aviso_y_no_modifica_recibos(tmp_path):
    rows = [
        Row('aviso', 1, {}, '', ['ESPERA.E1'], [], ['aviso']),
        Row('accion', 2, {}, '', [], ['programa_espera'], []),
    ]
    work = SimpleNamespace(
        rows=rows,
        receipts={},
        output='',
        path=str(tmp_path / 'origen.xlsx'),
        capture_sources={},
        capture_changes=None,
        mapping={},
        config={'correos': {}, 'contactos': {}, 'aliases': {}, 'firma': ''},
    )
    before = dict(work.receipts)
    statuses = case_statuses(work, [], [])
    assert statuses['aviso'].estado == 'REQUIERE_REVISION'
    assert statuses['accion'].estado == 'POR_PREPARAR'
    assert [item.row_id for item in attention_items(work, [], [])] == ['aviso', 'accion']
    assert work.receipts == before
