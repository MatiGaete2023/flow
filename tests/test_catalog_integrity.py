from hashlib import sha256
from pathlib import Path

from nurus.rus.catalog import load_catalog_snapshot


def test_catalog_and_hash_come_from_one_read(tmp_path, monkeypatch):
    source = tmp_path / "catalog.json"
    source.write_bytes(b'{"ESPERA": {}}')
    real_read = Path.read_bytes
    calls = []

    def read_then_change(path):
        result = real_read(path)
        if path == source:
            calls.append(path)
            path.write_bytes(b'{"ESPERA": {"changed": true}}')
        return result

    monkeypatch.setattr(Path, "read_bytes", read_then_change)
    catalog, digest = load_catalog_snapshot(source)
    assert catalog == {"ESPERA": {}}
    assert digest == sha256(b'{"ESPERA": {}}').hexdigest()
    assert len(calls) == 1
