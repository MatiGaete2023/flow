"""Intercambio autónomo de fixtures verificadas, sin Chrome ni Asistente."""

import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'src'))
from nurus.capture import read_capture
from export_csmp_capture import export_capture
from resultados import load_lot
from test_resultados import fixture
import motor as m


class CaptureExportTests(unittest.TestCase):
    def test_primary_originals_untouched_reexport_deterministic_and_partial(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            folder = fixture(root / 'lot')
            before = {p.name: p.read_bytes() for p in folder.iterdir()}
            lot = load_lot(folder)
            a = export_capture([lot], root / 'a.zip')
            b = export_capture([lot], root / 'b.zip')
            self.assertEqual(a.read_bytes(), b.read_bytes())
            capture = read_capture(a)
            self.assertEqual(len(capture.rows('registros')), 2)
            self.assertTrue(all(r['identity_status'] == 'INCIERTA' for r in capture.rows('registros')))
            self.assertEqual(capture.manifest['table_states']['bitacoras'], 'NO_SOLICITADA')
            self.assertEqual({p.name: p.read_bytes() for p in folder.iterdir()}, before)
            # Una consulta planificada no intentada sigue explícitamente faltante.
            summary = json.loads((folder / 'resumen.json').read_text())
            summary['estado'] = 'INCOMPLETA'
            summary['plan'].append({**summary['plan'][0], 'tab': 'Cumplimiento'})
            (folder / 'resumen.json').write_text(json.dumps(summary))
            partial = read_capture(export_capture([load_lot(folder, partial=True)], root / 'partial.zip'))
            self.assertEqual(partial.manifest['table_states']['consultas'], 'PARCIAL')
            self.assertIn('NO_CONSULTADA', [q['coverage'] for q in partial.rows('consultas')])

    def test_four_lots_and_changed_source_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            lots = [
                load_lot(fixture(root / f'lot{i}', mode=mode))
                for i, mode in enumerate(('Espera', 'Cumplimiento', 'Informes'))
            ]
            capture = read_capture(export_capture(lots, root / 'combined.zip', include_originals=True))
            self.assertEqual(
                {r['record_type'] for r in capture.rows('registros')}, {'ESPERA', 'CUMPLIMIENTO', 'INFORMES'}
            )
            (lots[0].folder / 'pagina.xls').write_bytes(b'alterado')
            with self.assertRaises(m.PocError):
                export_capture(lots, root / 'bad.zip')
            self.assertFalse((root / 'bad.zip').exists())

    def test_egresados_tab_is_typed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            capture = read_capture(export_capture([load_lot(fixture(root / 'eg', mode='Egresados'))], root / 'e.zip'))
            self.assertEqual({r['record_type'] for r in capture.rows('registros')}, {'EGRESADOS'})


if __name__ == '__main__':
    unittest.main()
