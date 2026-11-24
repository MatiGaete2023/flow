"""Prueba focalizada del lector existente con HTML ficticio, no captura RUS."""
from datetime import timedelta
from html import escape
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

from nurus.bitacora_html import HEADERS, read_snapshot
from registrador_rus.app.readback import Snapshot
from registrador_rus.app.journal import SimulationJournal
from registrador_rus.app.simulation import demo_operations, FakeClock


def params(op):
    i = op.remote_identity
    return dict(COD_Tribunal=i.tribunal_id, CRR_IdCausa=i.causa_id, ID_Ingreso=i.ingreso_id,
                COD_Etapa='1', TIP_Consulta='1', tipo_popUp='11')


def hidden(name, value):
    return '<input type="hidden" name="' + escape(name, quote=True) + '" value="' + escape(value, quote=True) + '">'


def fixture(op, *, entries=(), partial=False):
    context = params(op) | dict(RIT_Causa='FICTICIO', GLS_Nombre='Persona ficticia', GLS_Centro='Centro ficticio')
    header = '<tr>' + ''.join('<th>' + escape(h) + '</th>' for h in HEADERS) + '</tr>'
    rows = []
    for entry_id in entries:
        fields = ('Administrativa' if not op.cc else 'Al Tribunal', 'Etapa ficticia', '24/11/2026 09:00:01',
                  'actor-ficticio', 'Vista abreviada', '---', '', '---')
        cells = ''.join('<td>' + escape(t) + '</td>' for t in fields)
        metadata = ''.join(hidden(k + entry_id, v) for k,v in (
            ('HOBS_Centro_', op.approved_text), ('HOBS_Tribunal_', ''), ('HCOD_Estado_', op.observation_state),
            ('HFUN_Tribunal_', ''), ('HFLG_Envio_', str(op.cc))))
        rows.append('<tr>' + cells + '<td><img id="' + entry_id + '">' + metadata + '</td></tr>')
    return ('<html><body><form method="post" name="InformesPpalForm" action="/SITFAWEB/InformesDAction.do">'
            + ''.join(hidden(k,v) for k,v in context.items())
            + '<table id="TablaInforme">' + header + ''.join(rows) + '</table>'
            + ('<a rel="next">Siguiente</a>' if partial else '') + '</form></body></html>')


class ReaderIntegrationTests(unittest.TestCase):
    def test_real_parser_to_receipt_and_partial_recovery(self):
        for op in demo_operations(2):
            clock = FakeClock()
            with TemporaryDirectory() as tmp:
                j = SimulationJournal(Path(tmp)/'demo.sqlite', clock=clock)
                try:
                    j.freeze(op); j.mark_ready(op); j.authorize_simulation(op,human_actor='actor-ficticio',confirmed=True)
                    before = read_snapshot(fixture(op), params(op), captured_at=clock().isoformat())
                    self.assertTrue(Snapshot.from_reader(before).complete)
                    self.assertEqual(j.bind_baseline(op,before,author='actor-ficticio',stage='Etapa ficticia',
                        stage_code='1',server_day=clock().date()),'SIN_PREVIA')
                    self.assertEqual(j.reserve_simulated_attempt(op),'ENVIANDO')
                    clock.tick()
                    after = read_snapshot(fixture(op,entries=('900001',),partial=True),params(op),captured_at=clock().isoformat())
                    self.assertFalse(Snapshot.from_reader(after).complete)
                    self.assertEqual(j.resolve_simulated(op,after),'INCIERTA')
                    clock.tick()
                    after = read_snapshot(fixture(op,entries=('900001',)),params(op),captured_at=clock().isoformat())
                    self.assertEqual(Snapshot.from_reader(after).entries[0].text,op.approved_text)
                    self.assertEqual(j.resolve_simulated(op,after),'COMPROBADA')
                    self.assertEqual(j.attempt_count(),1)
                finally: j.close()

    def test_reader_rejects_wrong_identity(self):
        op = demo_operations(1)[0]
        with self.assertRaises(ValueError):
            read_snapshot(fixture(op),params(op)|{'ID_Ingreso':'999999'},captured_at=FakeClock()().isoformat())
