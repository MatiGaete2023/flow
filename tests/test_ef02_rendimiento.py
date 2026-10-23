"""EF-02: optimizaciones medidas con el benchmark sintético; el resultado debe ser idéntico al cálculo completo."""

from copy import deepcopy
from datetime import date

from nurus.capture import write_capture
from nurus.capture.fixtures import fixture
from nurus.personal import capture_findings, record_edits
from nurus.personal.capture_input import import_capture
from nurus.personal.config import defaults
from nurus.rus.columns import normalize


def _work(tmp_path, rows=12):
    metadata, tables = fixture(rows)
    return import_capture(write_capture(tmp_path / 'c.zip', metadata, tables), defaults(), as_of=date(2026, 10, 8))


def test_editar_un_registro_equivale_a_recalcular_todos(tmp_path, monkeypatch):
    work = _work(tmp_path)
    reference = deepcopy(work)
    target = work.rows[3].id
    review = {'OBSERVACION': 'Observación ficticia.'}
    seen = []
    original = capture_findings.updated_contexts
    monkeypatch.setattr(
        capture_findings, 'updated_contexts', lambda w, rows: seen.append(len(rows)) or original(w, rows)
    )
    record_edits.apply(work, [target], review=review)
    assert seen == [1]
    monkeypatch.setattr(capture_findings, 'updated_contexts', original)
    record_edits.apply(reference, [target], review=review)
    full = original(reference, reference.rows)
    assert work.capture_context == full
    assert [r.__dict__ for r in work.rows] == [r.__dict__ for r in reference.rows]


def test_deshacer_tras_edicion_parcial_restaura_contextos(tmp_path):
    work = _work(tmp_path)
    original = deepcopy(work.capture_context)
    before = record_edits.apply(work, [work.rows[0].id], review={'OBSERVACION': 'Cambio ficticio.'})
    record_edits.undo(work, before)
    assert work.capture_context == original


def test_normalize_memorizada_conserva_resultado_y_tipos():
    assert normalize('  Fec. Oído  ') == 'fec oido'
    assert normalize(None) == '' and normalize(0) == '' and normalize(12) == '12'
    assert normalize('A-(B).C') == normalize('A-(B).C') == 'a b c'
