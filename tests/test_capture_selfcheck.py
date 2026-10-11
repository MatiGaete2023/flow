"""La prueba distribuida debe recorrer productores reales y fallar ante pérdida."""

from pathlib import Path
import json
import os
import subprocess
import sys

import pytest


def test_portable_pipeline_without_ui_network_or_downloader(tmp_path):
    root = Path(__file__).parents[1]
    script = '''
import json,socket,sys
from pathlib import Path
sys.path.insert(0,sys.argv[1])
def forbidden(*args,**kwargs):raise AssertionError('Acceso externo prohibido')
socket.socket.connect=forbidden
socket.create_connection=forbidden
from nurus.adapters import outlook
outlook.save_draft=forbidden
from nurus.personal.capture_selfcheck import check_pipeline
result=check_pipeline(Path(sys.argv[2])/'productos')
assert result['ok'] and result['institutional_reads']==result['institutional_writes']==result['outlook_saved']==0
assert not any(n in sys.modules for n in ('nurus.personal.app','nurus.personal.download_link','export_csmp_capture'))
print(json.dumps(result))
'''
    # Una carpeta nueva evita depender de imports accidentales desde el repositorio.
    env = {k: v for k, v in os.environ.items() if k != 'DISPLAY'}
    result = subprocess.run(
        [sys.executable, '-c', script, str(root / 'src'), str(tmp_path)],
        cwd=tmp_path,
        env=env,
        capture_output=True,
        text=True,
        encoding='utf-8',
        timeout=60,
    )
    assert result.returncode == 0, result.stderr
    proof = json.loads(result.stdout)
    assert proof['rows'] == proof['modalities'] == 4 and proof['modes'] == 3
    assert proof['request_items'] == 8 and proof['word_projects'] == 1 and proof['rosters'] > 0
    assert all(len(digest) == 64 for digest in proof['product_sha256'].values())


def test_distribution_check_detects_missing_human_text(tmp_path, monkeypatch):
    from nurus.personal.capture_selfcheck import check_pipeline
    from nurus.personal.work import Work

    original = Work.export

    def discard_review(work, *args, **kwargs):
        for row in work.rows:
            row.review.pop('OBSERVACION', None)
        return original(work, *args, **kwargs)

    monkeypatch.setattr(Work, 'export', discard_review)
    with pytest.raises(ValueError, match='texto humano'):
        check_pipeline(tmp_path / 'productos')


def test_shared_contract_check_covers_empty_full_and_partial(tmp_path):
    from nurus.capture.selfcheck import check_contract
    from nurus.capture import read_capture

    assert check_contract(tmp_path)
    assert not read_capture(tmp_path / 'Captura_verificacion.zip').rows('registros')
    assert len(read_capture(tmp_path / 'Captura_completa.zip').rows('bitacoras')) == 4
    assert read_capture(tmp_path / 'Captura_parcial.zip').manifest['table_states']['bitacoras'] == 'NO_CONSULTADA'
