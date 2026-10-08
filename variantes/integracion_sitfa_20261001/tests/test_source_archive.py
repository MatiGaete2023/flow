from datetime import date
from hashlib import sha256
import sqlite3

import pandas as pd
import pytest
from openpyxl import Workbook

from nurus.rus import Mode, evaluate_batch, read_workbook
from nurus.services.workflow import WorkController
from nurus.storage.database import Database


def source_file(tmp_path):
    path = tmp_path / "original.xlsx"
    book = Workbook()
    sheet = book.active
    sheet.title = "Espera"
    sheet.append(["RIT", "TRIBUNAL", "DERIVACION", "T ESPERA"])
    sheet.append(["X-1", "Juzgado de Laja", "PRM Prueba", 45])
    other = book.create_sheet("OB")
    other["A1"] = "=1+2"
    other.column_dimensions["A"].width = 27
    book.save(path)
    return path


def test_original_bytes_survive_source_change_removal_and_reopen(tmp_path):
    path = source_file(tmp_path)
    original = path.read_bytes()
    db = Database(tmp_path / "db.sqlite3")
    controller = WorkController(db)
    batch = controller.analyze(path, Mode.ESPERA, as_of=date(2026, 9, 8))
    snapshot = controller.approve_batch(batch.batch_id)
    path.write_bytes(b"changed externally")
    assert db.get_original_workbook(batch.batch_id) == original
    path.unlink()
    reopened = Database(db.path)
    content = reopened.get_original_workbook(batch.batch_id, snapshot_hash=snapshot)
    assert content == original
    assert sha256(content).hexdigest() == batch.workbook_sha256
    assert batch.evaluations[0].source.row_number == 2


def test_parse_and_storage_use_same_bytes_when_external_file_changes(tmp_path, monkeypatch):
    path = source_file(tmp_path)
    original = path.read_bytes()
    real_excel_file = pd.ExcelFile

    def replace_external(*args, **kwargs):
        path.write_bytes(b"external replacement")
        return real_excel_file(*args, **kwargs)

    monkeypatch.setattr(pd, "ExcelFile", replace_external)
    db = Database(tmp_path / "db.sqlite3")
    batch = WorkController(db).analyze(path, Mode.ESPERA)
    assert db.get_original_workbook(batch.batch_id) == original
    assert batch.evaluations[0].values["RIT"] == "X-1"


def test_duplicate_import_reuses_original_blob(tmp_path):
    path = source_file(tmp_path)
    db = Database(tmp_path / "db.sqlite3")
    controller = WorkController(db)
    first = controller.analyze(path, Mode.ESPERA)
    second = controller.analyze(path, Mode.ESPERA)
    assert first.batch_id != second.batch_id
    assert db.get_original_workbook(first.batch_id) == db.get_original_workbook(second.batch_id)
    with db.connect() as conn:
        assert conn.execute("SELECT COUNT(*) FROM workbook_sources").fetchone()[0] == 1


def test_wrong_bytes_leave_no_batch_or_source(tmp_path):
    path = source_file(tmp_path)
    result = evaluate_batch(read_workbook(path, Mode.ESPERA))
    db = Database(tmp_path / "db.sqlite3")
    with pytest.raises(ValueError, match="hash"):
        db.save_evaluation_batch(result, source_bytes=b"unrelated")
    assert db.get_batch(result.batch_id) is None
    with db.connect() as conn:
        assert conn.execute("SELECT COUNT(*) FROM workbook_sources").fetchone()[0] == 0


def test_record_failure_rolls_back_source_and_batch(tmp_path):
    path = source_file(tmp_path)
    db = Database(tmp_path / "db.sqlite3")
    with db.connect() as conn:
        conn.execute("CREATE TRIGGER fail_record BEFORE INSERT ON review_records BEGIN SELECT RAISE(ABORT, 'injected'); END")
    with pytest.raises(sqlite3.IntegrityError, match="injected"):
        WorkController(db).analyze(path, Mode.ESPERA)
    with db.connect() as conn:
        assert conn.execute("SELECT COUNT(*) FROM workbook_sources").fetchone()[0] == 0
        assert conn.execute("SELECT COUNT(*) FROM batches").fetchone()[0] == 0


@pytest.mark.parametrize("statement", [
    "UPDATE workbook_sources SET content=content",
    "DELETE FROM workbook_sources",
])
def test_originals_reject_update_and_delete(tmp_path, statement):
    path = source_file(tmp_path)
    db = Database(tmp_path / "db.sqlite3")
    WorkController(db).analyze(path, Mode.ESPERA)
    with pytest.raises(sqlite3.IntegrityError, match="inmutable"):
        with db.connect() as conn:
            conn.execute(statement)


def test_corrupted_blob_is_detected(tmp_path):
    path = source_file(tmp_path)
    db = Database(tmp_path / "db.sqlite3")
    batch = WorkController(db).analyze(path, Mode.ESPERA)
    with db.connect() as conn:
        conn.execute("DROP TRIGGER source_no_update")
        conn.execute("UPDATE workbook_sources SET content=?,size=3", (b"bad",))
    with pytest.raises(ValueError, match="hash"):
        db.get_original_workbook(batch.batch_id)


def test_missing_legacy_bytes_are_not_read_from_external_path(tmp_path):
    path = source_file(tmp_path)
    result = evaluate_batch(read_workbook(path, Mode.ESPERA))
    db = Database(tmp_path / "db.sqlite3")
    db.save_evaluation_batch(result)
    with pytest.raises(ValueError, match="vuelve a importarlo"):
        db.get_original_workbook(result.batch_id)
    assert path.exists()


def test_v4_migration_backs_up_and_preserves_existing_data(tmp_path):
    db = Database(tmp_path / "db.sqlite3")
    with db.connect() as conn:
        template_count = conn.execute("SELECT COUNT(*) FROM templates").fetchone()[0]
        conn.execute("PRAGMA user_version=4")
    migrated = Database(db.path)
    assert ".pre-v8-" in migrated.migration_backup.name
    with sqlite3.connect(migrated.migration_backup) as backup:
        assert backup.execute("PRAGMA user_version").fetchone()[0] == 4
        assert backup.execute("SELECT COUNT(*) FROM templates").fetchone()[0] == template_count
    with migrated.connect() as conn:
        assert conn.execute("PRAGMA user_version").fetchone()[0] == 8
    assert Database(db.path).migration_backup is None

def test_snapshot_bytes_hash_and_repr(tmp_path):
    path = source_file(tmp_path)
    batch = read_workbook(path, Mode.ESPERA)
    assert batch.source_bytes == path.read_bytes()
    assert sha256(batch.source_bytes).hexdigest() == batch.workbook_sha256
    assert "source_bytes=" not in repr(batch)
