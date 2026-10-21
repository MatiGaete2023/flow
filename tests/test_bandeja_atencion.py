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


def _work(tmp_path, rows):
    return SimpleNamespace(
        rows=rows,
        receipts={},
        output='',
        path=str(tmp_path / 'origen.xlsx'),
        capture_sources={},
        capture_changes=None,
        mapping={},
        config={'correos': {}, 'contactos': {}, 'aliases': {}, 'firma': ''},
    )


def test_bandeja_ordena_por_nivel_de_prioridad_y_solo_muestra_el_mayor(tmp_path):
    def fila(rid, avisos, **extra):
        row = Row(rid, 1, {}, '', [], extra.get('actions', []), extra.get('warnings', []), extra.get('excluded', False))
        row.avisos = avisos
        return row

    rows = [
        fila('incierta', [], warnings=['revisar']),
        fila('nivel3', [(3, 'C8_NO_USO', 'Medida vencida sin Prórroga')]),
        fila('nivel2', [(2, 'T2_MN', 'M.N. posterior'), (3, 'C8_NO_USO', 'otro')]),
        fila('nivel1', [(2, 'R3', 'x'), (1, 'R6_DUPLICIDAD', 'Duplicidad')], warnings=['revisar']),
        fila('excluida', [(1, 'R6_DUPLICIDAD', 'Duplicidad')], excluded=True),
        fila('sin_marca', []),
    ]
    work = _work(tmp_path, rows)
    estados = case_statuses(work, [], [])
    assert (estados['nivel1'].nivel, estados['nivel1'].marca) == (1, 'PRIORITARIO')
    assert (estados['nivel2'].nivel, estados['nivel2'].marca) == (2, 'REQUIERE_ATENCION')
    assert estados['excluida'].nivel == 0 and estados['sin_marca'].nivel == 0
    assert estados['nivel3'].proxima_accion == 'Revisa: Medida vencida sin Prórroga'
    assert [i.row_id for i in attention_items(work, [], [])] == ['nivel1', 'incierta', 'nivel2', 'nivel3']


def test_sesiones_antiguas_sin_avisos_siguen_funcionando(tmp_path):
    row = Row('vieja', 1, {}, '', [], [], [])
    del row.avisos
    assert case_statuses(_work(tmp_path, [row]), [], [])['vieja'].nivel == 0
