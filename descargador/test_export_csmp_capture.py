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


class CalendarioAsociadoTests(unittest.TestCase):
    @staticmethod
    def registro(rid, cause, ingreso, nombre='Persona Ficticia', tipo='CUMPLIMIENTO', programa='AFT FICTICIO'):
        normalizado = {'RIT': 'X-1-2026', 'NOMBRE': nombre, 'DERIVACION': programa}
        return {
            'record_id': rid,
            'record_type': tipo,
            'identity_status': 'VERIFICADA' if cause else 'INCIERTA',
            'court_code': '113',
            'cause_id': cause,
            'ingreso_id': ingreso,
            'person_id': '',
            'program_id': '',
            'query_id': 'q' + rid,
            'source_sha256': 'h',
            'original_row': rid,
            'normalized_json': json.dumps(normalizado),
        }

    @staticmethod
    def calendario(rid, nombre='Persona Ficticia', programa='AFT FICTICIO'):
        normalizado = {
            'RIT': 'X-1-2026',
            'NOMBRE': nombre,
            'NOMBRE CENTRO': programa,
            'FECHA VENCIMIENTO': '2026-11-20',
        }
        return {
            'query_id': 'q' + rid,
            'source_sha256': 'h',
            'original_row': rid,
            'court_code': '113',
            'cause_id': '',
            'ingreso_id': '',
            'person_id': '',
            'program_id': '',
            'association': 'NO_ASOCIADO',
            'normalized_json': json.dumps(normalizado),
        }

    def test_el_calendario_se_asocia_solo_si_hay_un_unico_ingreso_con_esos_datos(self):
        from export_csmp_capture import asociar_calendarios

        data = {
            'registros': [
                self.registro('1', '500', '900'),
                self.registro('2', '501', '901', nombre='Otra Persona'),
                self.registro('3', '502', '902', nombre='Empatada'),
                self.registro('4', '503', '903', nombre='Empatada'),
                self.registro('c1', '', '', tipo='INFORMES'),
                self.registro('c2', '', '', nombre='Otra Persona', tipo='INFORMES'),
                self.registro('c3', '', '', nombre='Empatada', tipo='INFORMES'),
                self.registro('c4', '', '', nombre='Desconocida', tipo='INFORMES'),
            ],
            'calendarios': [
                self.calendario('c1'),
                self.calendario('c2', nombre='Otra Persona'),
                self.calendario('c3', nombre='Empatada'),
                self.calendario('c4', nombre='Desconocida'),
            ],
        }
        resueltos = asociar_calendarios(data)
        self.assertEqual(sorted(resueltos), ['c1', 'c2'])
        asociados = {c['query_id']: (c['association'], c['cause_id'], c['ingreso_id']) for c in data['calendarios']}
        self.assertEqual(asociados['qc1'], ('VERIFICADA', '500', '900'))
        self.assertEqual(asociados['qc2'], ('VERIFICADA', '501', '901'))
        self.assertEqual(asociados['qc3'][0], 'NO_ASOCIADO')
        self.assertEqual(asociados['qc4'][0], 'NO_ASOCIADO')
        registro = next(r for r in data['registros'] if r['record_id'] == 'c1')
        self.assertEqual((registro['identity_status'], registro['cause_id']), ('VERIFICADA', '500'))
