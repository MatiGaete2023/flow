"""Prueba de usuario 13-oct: una bitácora no leída (límite de 300 lecturas) no se muestra como «sin entradas»."""

import json
from datetime import date

from export_csmp_capture import export_capture
from nurus.capture import read_capture
from nurus.personal.capture_findings import last_center_label, novelties_text
from nurus.personal.capture_input import import_capture
from nurus.personal.config import defaults
from test_reutilizacion_local import bound_lot, diaries


def _labels(tmp_path, manifest):
    lot = bound_lot(tmp_path / 'principal')
    path = export_capture([lot], tmp_path / 'paquete.zip', diaries=[manifest], match_diaries=True)
    work = import_capture(path, defaults(), as_of=date(2026, 10, 13))
    return work, {work.capture_sources[row.id]['cause_id']: row for row in work.rows}


def test_ingreso_sin_lectura_dice_bitacora_no_leida(tmp_path):
    work, rows = _labels(tmp_path, diaries(tmp_path / 'Bitacoras 2026-10-13', 1))
    # La entrada ficticia «Al Tribunal» sin respuesta tiene más de 30 días: aviso CT-05 (19-oct-2026).
    assert last_center_label(work, rows['100']) == '12/08/2026 · 1 aviso'
    assert 'CT-05 · Observación al tribunal del 12/08/2026' in novelties_text(work, rows['100'])
    assert last_center_label(work, rows['101']) == 'Bitácora no leída'
    assert 'no se leyó' in novelties_text(work, rows['101'])


def test_lectura_fallida_dice_bitacora_no_leida(tmp_path):
    manifest = diaries(tmp_path / 'Bitacoras 2026-10-13', 2)
    data = json.loads(manifest.read_text(encoding='utf-8'))
    data['registros'][1].update(estado='PENDIENTE', error='Pendiente: se alcanzó el máximo de lecturas del lote.')
    manifest.write_text(json.dumps(data), encoding='utf-8')
    work, rows = _labels(tmp_path, manifest)
    assert read_capture(work.path).rows('consultas')
    assert last_center_label(work, rows['101']) == 'Bitácora no leída'
    assert last_center_label(work, rows['100']) == '12/08/2026 · 1 aviso'
