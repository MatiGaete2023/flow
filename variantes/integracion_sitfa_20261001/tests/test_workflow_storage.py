from __future__ import annotations

import hashlib
from datetime import date

import pytest
from openpyxl import Workbook

from nurus.domain.models import Template
from nurus.rus import Mode
from nurus.services.workflow import WorkController
from nurus.storage.database import Database


def _book(path, sheet_name, headers, rows):
    book = Workbook()
    sheet = book.active
    sheet.title = sheet_name
    sheet.append(headers)
    for row in rows:
        sheet.append(row)
    book.save(path)


def test_controller_persists_two_traceable_rows_and_freezes_approval(tmp_path):
    path = tmp_path / "espera.xlsx"
    _book(
        path,
        "Espera",
        ["DERIVACION", "TRIBUNAL", "NOMBRE", "RIT", "T ESPERA"],
        [
            ["PRM Norte", "Juzgado de Laja", "Ana Pérez", "C-1", 45],
            ["PRM Sur", "Juzgado de Tomé", "Luis Soto", "C-2", 10],
        ],
    )
    db = Database(tmp_path / "nurus.sqlite3")
    controller = WorkController(db)

    batch = controller.analyze(path, Mode.ESPERA, as_of=date(2026, 9, 8))
    rows = controller.rows_from_batch(batch)

    assert len(rows) == 2
    assert [row.source_row for row in rows] == [2, 3]
    assert len({row.record_id for row in rows}) == 2
    assert all(row.decision == "pending" for row in rows)

    persisted = db.get_batch(batch.batch_id)
    assert persisted is not None
    assert persisted["source_hash"] == hashlib.sha256(path.read_bytes()).hexdigest()
    assert persisted["source_hash"] != "manual"
    assert persisted["status"] == "review"

    # Una sola aprobación de lote acepta todas las propuestas sin incidencias.
    snapshot_hash = controller.approve_batch(batch.batch_id)

    assert len(snapshot_hash) == 64
    approved = db.get_batch(batch.batch_id)
    assert approved["status"] == "approved"
    assert approved["snapshot_hash"] == snapshot_hash
    stored_rows = controller.load_rows(batch.batch_id)
    assert all(row.decision == "approved" for row in stored_rows)


def test_blocked_row_cannot_be_approved_but_can_be_explicitly_excluded(tmp_path):
    path = tmp_path / "cumplimiento.xlsx"
    _book(
        path,
        "Cumplimiento",
        ["DERIVACION", "TRIBUNAL", "RIT", "DIAS DE CUMPLIMIENTO", "DIAS PARA EGRESAR"],
        [["PRM Centro", "Tribunal desconocido", "C-3", 100, 100]],
    )
    db = Database(tmp_path / "nurus.sqlite3")
    controller = WorkController(db)
    batch = controller.analyze(path, Mode.CUMPLIMIENTO, as_of=date(2026, 9, 8))
    row = controller.rows_from_batch(batch)[0]

    assert row.status == "blocked"
    assert row.decision == "blocked"
    assert any("G-06" in issue for issue in row.issues)

    with pytest.raises(ValueError):
        controller.approve_record(batch.batch_id, row.record_id)
    with pytest.raises(ValueError, match="filas bloqueadas"):
        controller.approve_batch(batch.batch_id)

    controller.exclude_record(batch.batch_id, row.record_id, reason="Revisión manual: tribunal no reconocido")
    controller.document_missing_cross_sheet_exception(
        batch.batch_id,
        responsible="Revisor de prueba",
        reason="La planilla de prueba no contiene hoja de cruce.",
    )
    snapshot_hash = controller.approve_batch(batch.batch_id)
    assert len(snapshot_hash) == 64


def test_blocked_exception_does_not_force_clicking_clean_rows_one_by_one(tmp_path):
    path = tmp_path / "cumplimiento-mixto.xlsx"
    _book(
        path,
        "Cumplimiento",
        ["DERIVACION", "TRIBUNAL", "RIT", "DIAS DE CUMPLIMIENTO", "DIAS PARA EGRESAR"],
        [
            ["PRM Centro", "Juzgado de Laja", "C-10", 100, 100],
            ["PRM Centro", "Tribunal desconocido", "C-11", 100, 100],
            ["PRM Centro", "Juzgado de Laja", "C-12", 100, 100],
        ],
    )
    db = Database(tmp_path / "nurus.sqlite3")
    controller = WorkController(db)
    batch = controller.analyze(path, Mode.CUMPLIMIENTO, as_of=date(2026, 9, 8))
    rows = controller.rows_from_batch(batch)

    blocked = next(row for row in rows if row.decision == "blocked")
    clean = [row for row in rows if row.decision == "pending"]
    assert len(clean) == 2

    with pytest.raises(ValueError, match="2 filas sin incidencias se aprobarán juntas"):
        controller.approve_batch(batch.batch_id)

    # El fallo por la excepción no altera todavía las filas normales.
    assert [row.decision for row in controller.load_rows(batch.batch_id)].count("pending") == 2

    controller.exclude_record(
        batch.batch_id,
        blocked.record_id,
        reason="Revisión manual: tribunal no reconocido",
    )
    controller.document_missing_cross_sheet_exception(
        batch.batch_id,
        responsible="Revisor de prueba",
        reason="La planilla de prueba no contiene hoja de cruce.",
    )
    snapshot_hash = controller.approve_batch(batch.batch_id)
    assert len(snapshot_hash) == 64

    final_rows = controller.load_rows(batch.batch_id)
    assert [row.decision for row in final_rows].count("approved") == 2
    assert [row.decision for row in final_rows].count("excluded") == 1
    assert all(row.decision != "pending" for row in final_rows)


def test_foreign_keys_are_enabled_on_every_database_connection(tmp_path):
    db = Database(tmp_path / "nurus.sqlite3")
    assert db.foreign_keys_enabled() is True
    with db.connect() as conn:
        assert conn.execute("PRAGMA foreign_keys").fetchone()[0] == 1


def test_template_versions_are_immutable_and_preserved(tmp_path):
    db = Database(tmp_path / "nurus.sqlite3")
    original = db.list_templates()[0]
    saved = db.save_template(
        Template(
            id=original.id,
            name=original.name,
            kind=original.kind,
            subject=original.subject,
            body=original.body + "\nNueva revisión",
            allowed_variables=original.allowed_variables,
            status=original.status,
            version=original.version,
        )
    )

    versions = db.list_template_versions(original.id)
    assert saved.version == original.version + 1
    assert [item.version for item in versions] == [original.version, saved.version]
    assert versions[0].body == original.body
    assert versions[-1].body.endswith("Nueva revisión")


def test_edit_or_exclusion_requires_a_reason(tmp_path):
    path = tmp_path / "espera.xlsx"
    _book(
        path,
        "Espera",
        ["DERIVACION", "TRIBUNAL", "RIT", "T ESPERA"],
        [["PRM Norte", "Juzgado de Laja", "C-1", 45]],
    )
    db = Database(tmp_path / "nurus.sqlite3")
    controller = WorkController(db)
    batch = controller.analyze(path, Mode.ESPERA, as_of=date(2026, 9, 8))
    row = controller.rows_from_batch(batch)[0]

    with pytest.raises(ValueError):
        controller.approve_record(
            batch.batch_id,
            row.record_id,
            observation=row.observation + " Editada.",
        )
    with pytest.raises(ValueError):
        controller.exclude_record(batch.batch_id, row.record_id, reason="")

