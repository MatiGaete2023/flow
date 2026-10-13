from types import SimpleNamespace

from nurus.personal.capture_changes import compare, label, summary


def _row(row_id, rules=(), observation='', values=None):
    return SimpleNamespace(id=row_id, rules=list(rules), observation=observation, values=values or {}, overrides={})


def test_compara_nuevas_fuente_reglas_y_egresadas():
    previous = SimpleNamespace(
        capture_digest='anterior',
        mapping={'rit': 'RIT'},
        rows=[_row('a', ['E1'], values={'RIT': 'X-1'}), _row('b', values={'RIT': 'X-2'})],
    )
    candidate = SimpleNamespace(rows=[_row('a2', ['E2'], values={'RIT': 'X-1'}), _row('c', values={'RIT': 'X-3'})])
    changes = compare(candidate, previous, {'a2': 'a'}, {'a2': ('FECHA',), 'c': ()})
    assert changes['filas']['a2']['tipo'] == 'CAMBIO_FUENTE'
    assert changes['filas']['a2']['reglas_antes'] == ['E1']
    assert changes['filas']['c']['tipo'] == 'NUEVO'
    assert [entry['id_anterior'] for entry in changes['egresados']] == ['b']
    assert summary(changes)['EGRESADOS'] == 1
    assert label(changes, 'a2') == 'Fuente'


def test_sin_anterior_no_inventa_cambios():
    assert compare(SimpleNamespace(rows=[]), SimpleNamespace(capture_digest='', rows=[]), {}, {}) is None
