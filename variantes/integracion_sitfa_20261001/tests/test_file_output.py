from unittest.mock import Mock
import json
import pytest
from openpyxl import load_workbook

from nurus.services.file_output import write_new_file


def test_failed_writer_never_publishes_partial_output(tmp_path):
    destination = tmp_path / "report.xlsx"

    def fail(path):
        path.write_bytes(b"partial")
        raise OSError("disk failure")

    with pytest.raises(OSError):
        write_new_file(destination, fail)
    assert not destination.exists()
    assert list(tmp_path.iterdir()) == []


def test_destination_created_during_generation_is_preserved(tmp_path):
    destination = tmp_path / "report.xlsx"

    def race(path):
        path.write_bytes(b"generated")
        destination.write_bytes(b"someone else's file")

    with pytest.raises(FileExistsError):
        write_new_file(destination, race)
    assert destination.read_bytes() == b"someone else's file"
    assert list(tmp_path.iterdir()) == [destination]


def test_failure_during_publication_removes_only_owned_partial(tmp_path, monkeypatch):
    destination = tmp_path / "report.xlsx"

    def fail(source, output):
        output.write(b"partial")
        raise OSError("copy failure")

    monkeypatch.setattr("nurus.services.file_output.shutil.copyfileobj", fail)
    with pytest.raises(OSError):
        write_new_file(destination, lambda path: path.write_bytes(b"complete"))
    assert not destination.exists()
    assert list(tmp_path.iterdir()) == []


def test_nomina_treats_user_strings_as_text_not_formulas(tmp_path):
    from nurus.services.communications import attach_snapshot_table
    db = Mock()
    db.get_snapshot.return_value = {
        "batch": {"column_mapping": json.dumps({"rit": "RIT", "nombre": "NOMBRE"})},
        "records": [{"record_id": "id", "decision": "approved", "rus_recorded": True,
                     "values_json": json.dumps({"RIT": "=1+1", "NOMBRE": "+command"})}],
    }
    product = Mock(id="test-product", batch_id="batch", source_snapshot_hash="hash", record_ids=("id",), attachments=[])
    target = attach_snapshot_table(db, product, tmp_path)
    with target.open("rb") as source:
        book = load_workbook(source)
        assert book.active["A2"].data_type == "s"
        assert book.active["A2"].value == "'=1+1"
        assert book.active["D2"].value == "'+command"
        book.close()
