import sys
from types import ModuleType
import pytest
from nurus import __version__
from nurus.personal.package_check import build_identity


def test_frozen_build_rejects_missing_metadata(monkeypatch):
    monkeypatch.setattr(sys, 'frozen', True, raising=False)
    monkeypatch.setitem(sys.modules, 'nurus.personal._build_meta', None)
    with pytest.raises(ValueError, match='commit y versión'):
        build_identity()


@pytest.mark.parametrize(
    'commit,version', [pytest.param('desarrollo', __version__, id='desarrollo'), ('a' * 40, '0.0.0')]
)
def test_frozen_build_rejects_wrong_origin(monkeypatch, commit, version):
    meta = ModuleType('nurus.personal._build_meta')
    meta.BUILD_COMMIT = commit
    meta.BUILD_VERSION = version
    monkeypatch.setitem(sys.modules, meta.__name__, meta)
    monkeypatch.setattr(sys, 'frozen', True, raising=False)
    with pytest.raises(ValueError):
        build_identity()


def test_frozen_build_exposes_exact_origin(monkeypatch):
    meta = ModuleType('nurus.personal._build_meta')
    meta.BUILD_COMMIT = 'a' * 40
    meta.BUILD_VERSION = __version__
    monkeypatch.setitem(sys.modules, meta.__name__, meta)
    monkeypatch.setattr(sys, 'frozen', True, raising=False)
    assert build_identity() == {'source_commit': 'a' * 40, 'source_version': __version__}
