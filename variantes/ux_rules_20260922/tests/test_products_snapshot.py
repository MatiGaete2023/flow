from __future__ import annotations

from datetime import date

import pytest
from openpyxl import Workbook

from nurus.rus import Mode
from nurus.services.products import ProductBuildError, approve_product, persist_approved_product, prepare_from_snapshot
from nurus.services.workflow import WorkController
from nurus.storage.database import Database


def _write(path, rows):
    book = Workbook()
    sheet = book.active
    sheet.title = "Espera"
    sheet.append(["DERIVACION", "TRIBUNAL", "NOMBRE", "RIT", "T ESPERA"])
    for row in rows:
        sheet.append(row)
    book.save(path)


def _approved_batch(db, path):
    controller = WorkController(db)
    batch = controller.analyze(path, Mode.ESPERA, as_of=date(2026, 9, 8))
    updates = [
        {
            "record_id": row["record_id"],
            "observation": row["edited_observation"],
            "review_date": "2026-09-08",
            "tt": "",
            "workload": "",
            "resolution": "",
        }
        for row in db.list_review_records(batch.batch_id)
    ]
    db.apply_review_import(
        batch.batch_id,
        source_name="constancia.xlsx",
        source_bytes=b"constancia completa",
        responsible="Revisora CSMP",
        updates=updates,
    )
    controller.approve_batch(batch.batch_id, require_review_import=True)
    return batch


def test_product_uses_frozen_snapshot_even_if_source_file_changes(tmp_path):
    path = tmp_path / "espera.xlsx"
    _write(
        path,
        [
            ["PRM Norte", "Juzgado de Laja", "Ana Pérez", "C-1", 45],
            ["PRM Norte", "Juzgado de Laja", "Luis Soto", "C-2", 50],
        ],
    )
    db = Database(tmp_path / "nurus.sqlite3")
    batch = _approved_batch(db, path)

    product_before = prepare_from_snapshot(db, batch.batch_id, "email-ingreso")
    assert product_before.batch_id == batch.batch_id
    assert "C-1" in product_before.rendered_body
    assert "C-2" in product_before.rendered_body
    assert product_before.recipient == ""
    assert product_before.warnings

    _write(path, [["ALTERADO", "Juzgado de Tomé", "Otro Nombre", "X-999", 999]])
    product_after = prepare_from_snapshot(db, batch.batch_id, "email-ingreso")

    assert product_after.rendered_body == product_before.rendered_body
    assert "X-999" not in product_after.rendered_body
    assert product_after.context == product_before.context

    approve_product(product_after)
    persist_approved_product(db, product_after)
    stored = db.list_products()[0]
    stored_batch = db.get_batch(batch.batch_id)
    assert stored["source_snapshot_hash"] == stored_batch["snapshot_hash"]
    assert stored["template_version"] == product_after.template.version


def test_mixed_tribunals_are_not_grouped_silently(tmp_path):
    path = tmp_path / "espera.xlsx"
    _write(
        path,
        [
            ["PRM Norte", "Juzgado de Laja", "Ana Pérez", "C-1", 45],
            ["PRM Norte", "Juzgado de Tomé", "Luis Soto", "C-2", 65],
        ],
    )
    db = Database(tmp_path / "nurus.sqlite3")
    batch = _approved_batch(db, path)

    with pytest.raises(ProductBuildError, match="más de un tribunal"):
        prepare_from_snapshot(db, batch.batch_id, "email-ingreso")
