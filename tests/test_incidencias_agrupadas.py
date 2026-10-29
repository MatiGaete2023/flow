"""La evidencia de la captura agrupa incidencias idénticas con su total (prueba del usuario 29-oct-2026)."""

from types import SimpleNamespace

from nurus.personal.capture_products import evidence_text


def incidencia(code, message):
    return {'code': code, 'severity': 'ADVERTENCIA', 'message': message, 'record_id': '', 'query_id': 'q'}


def test_incidencias_identicas_se_agrupan_y_las_distintas_siguen_visibles():
    work = SimpleNamespace(
        capture_digest='a' * 64,
        capture_id='captura-x',
        as_of='2026-10-29',
        ruleset_version='r',
        capture_manifest={'table_states': {'registros': 'COMPLETA'}},
        capture_incidents=[incidencia('IDENTIDAD_INCIERTA', 'Sin ingreso remoto.')] * 600
        + [incidencia('BITACORAS_REUTILIZADAS', 'Bitácoras conservadas: 119.')],
    )
    text = evidence_text(work)
    assert 'IDENTIDAD_INCIERTA · ADVERTENCIA · Sin ingreso remoto. (×600)' in text
    assert text.count('IDENTIDAD_INCIERTA') == 1
    assert 'BITACORAS_REUTILIZADAS · ADVERTENCIA · Bitácoras conservadas: 119.' in text
    assert '(×1)' not in text
