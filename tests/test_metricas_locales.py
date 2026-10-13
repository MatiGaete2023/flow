import json
from pathlib import Path
from types import SimpleNamespace

from nurus.personal.review_metrics import record_draft, record_export, record_import, summary


def test_metricas_solo_conteos_y_tope(tmp_path):
    work = SimpleNamespace(
        config={'metricas_locales': True},
        storage_directory=str(tmp_path),
        rows=[],
        capture_changes=None,
    )
    for _ in range(61):
        record_import(work)
    record_export(work)
    record_draft(work)
    path = Path(tmp_path) / 'metricas.json'
    data = json.loads(path.read_text(encoding='utf-8'))
    assert len(data['ciclos']) == 60
    assert data['ciclos'][-1]['exportado'] and data['ciclos'][-1]['primer_borrador']
    assert all('RIT' not in json.dumps(c) for c in data['ciclos'])
    assert len(summary(path)['ciclos']) == 60
