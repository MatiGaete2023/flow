import pytest

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
