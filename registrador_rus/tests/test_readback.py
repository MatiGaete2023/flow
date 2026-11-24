from datetime import date, datetime, timedelta, timezone
import unittest

from registrador_rus.app.contracts import RegistroFlowV1
from registrador_rus.app.readback import Snapshot, ReadbackError, compare, review_previous, METHOD
from test_contracts import sample

NOW = datetime(2026, 11, 24, 14, tzinfo=timezone.utc)
OP = RegistroFlowV1.from_mapping(sample())
CTX = dict(author='usuario-demo', stage='Etapa ficticia', stage_code='1', server_day=date(2026, 11, 24))
IDENT = dict(tribunal_codigo=OP.remote_identity.tribunal_id, causa_id=OP.remote_identity.causa_id,
             ingreso_id=OP.remote_identity.ingreso_id)


def row(key='1', **changes):
    return dict(IDENT, entry_id=key, texto=OP.approved_text, autor=CTX['author'], tipo='Administrativa',
                etapa=CTX['stage'], fecha='2026-11-24T11:00:00', origen='centro') | changes


def raw(rows=(), after=False, **changes):
    return dict(IDENT, entries=list(rows), row_metadata=[dict(entry_id=r['entry_id'], state=OP.observation_state) for r in rows],
                captured_at=(NOW + timedelta(seconds=1 if after else -1)).isoformat(), stage_code='1',
                alcance_metodo=METHOD, alcance='COMPLETA' if rows else 'VACIA_COMPROBADA', coverage='PARCIAL') | changes


def snap(rows=(), after=False, **changes):
    return Snapshot.from_reader(raw(rows, after, **changes))


class ReadbackTests(unittest.TestCase):
    def result(self, before, after):
        return compare(OP, before, after, attempt_at=NOW, **CTX)

    def test_new_unique_and_prior_different_day(self):
        prior=row('1',fecha='2026-11-23T11:00:00')
        result=self.result(snap([prior]),snap([prior,row('2')],True))
        self.assertEqual((result.state,result.entry.entry_id),('COINCIDENCIA_UNICA','2'))
        self.assertEqual(result.entry.server_timestamp,'2026-11-24T11:00:00')

    def test_previous_same_day_and_ambiguity(self):
        for rows,state in [([row('1')],'REUTILIZABLE'),([row('1'),row('2')],'REVISAR_PREVIAS')]:
            self.assertEqual(review_previous(OP,snap(rows),**CTX).state,state)
            self.assertEqual(self.result(snap(rows),snap(rows+[row('3')],True)).state,'INCIERTA')
        self.assertEqual(self.result(snap(),snap([row('1'),row('2')],True)).state,'AMBIGUA')

    def test_exact_fields(self):
        for changed in (dict(texto='otro'),dict(autor='otro'),dict(tipo='Al Tribunal'),dict(etapa='otra'),dict(fecha='2026-11-23T11:00:00')):
            with self.subTest(changed=changed):
                self.assertEqual(self.result(snap(),snap([row(**changed)],True)).state,'NO_HALLADA')
        self.assertEqual(self.result(snap(),snap([row()],True,row_metadata=[dict(entry_id='1',state='OTRO')])).state,'NO_HALLADA')
        self.assertEqual(self.result(snap(),snap([],True)).state,'NO_HALLADA')

    def test_context_coverage_and_order(self):
        for after in (snap([row()],True,alcance='PARCIAL'),snap([row()],True,alcance_metodo='OTRO'),snap([row()],True,stage_code='2'),
                      snap([],True,ingreso_id='OTRO'),snap([row()],False)):
            self.assertEqual(self.result(snap(),after).state,'INCIERTA')
        self.assertEqual(self.result(snap(alcance='PARCIAL'),snap([row()],True)).state,'INCIERTA')
        prior=row('1',fecha='2026-11-23T11:00:00')
        self.assertEqual(self.result(snap([prior]),snap([row('2')],True)).state,'INCIERTA')
        self.assertEqual(self.result(snap([prior]),snap([row('1'),row('2')],True)).state,'INCIERTA')

    def test_reject_invalid_reader_output(self):
        for data in (raw([row('1'),row('1')]),raw([row()],row_metadata=[]),raw([row(ingreso_id='OTRO')]),
                     raw([row(fecha='imposible')]),raw([row()],captured_at='2026-11-24T11:00:00')):
            with self.assertRaises(ReadbackError): Snapshot.from_reader(data)
