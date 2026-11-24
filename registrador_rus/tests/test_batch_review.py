from dataclasses import replace
from pathlib import Path
from tempfile import TemporaryDirectory
import json
import unittest

from registrador_rus.app.batch_review import BatchSelection, SimulationBatchReview
from registrador_rus.app.journal import SimulationJournal, JournalError
from registrador_rus.app.simulation import demo_operations, FakeClock, FakeRus, SimulationController


class BatchReviewTests(unittest.TestCase):
    def test_exact_selection(self):
        ops=demo_operations(); selection=BatchSelection(ops)
        with self.assertRaises(JournalError): SimulationBatchReview.confirm(selection,displayed_digest=selection.digest,confirmed=False)
        review=SimulationBatchReview.confirm(selection,displayed_digest=selection.digest,confirmed=True)
        review.check(BatchSelection(tuple(ops)))
        for changed in (ops[:-1],tuple(reversed(ops)),(replace(ops[0],destination='AL_TRIBUNAL'),)+ops[1:]):
            with self.assertRaises(JournalError): review.check(BatchSelection(changed))

    def test_duplicate_identity(self):
        first=demo_operations()[0]
        for ops in ((first,first),(first,replace(first,operation_id='otro'))):
            with self.assertRaises(JournalError): BatchSelection(ops)

    def test_summary_only_categories(self):
        ops=demo_operations(); selection=BatchSelection(ops); clock=FakeClock()
        with TemporaryDirectory() as tmp:
            j=SimulationJournal(Path(tmp)/'demo.sqlite',clock=clock)
            try:
                c=SimulationController(j,FakeRus(clock,ops)); c.prepare(ops[0]); c.authorize(ops[0],confirmed=True); c.execute(ops[0])
                report=selection.summary(j)
                self.assertEqual(report,dict(simulation_only=True,total=8,states={'COMPROBADA':1,'SIN_PREPARAR':7},destinations={'ADMINISTRATIVA':4,'AL_TRIBUNAL':4},attempts=1))
                output=json.dumps(report)
                for sensitive in (ops[0].operation_id,ops[0].approved_text,ops[0].approved_by,ops[0].remote_identity.ingreso_id,selection.digest):
                    self.assertNotIn(sensitive,output)
                changed=BatchSelection((replace(ops[0],destination='AL_TRIBUNAL'),)+ops[1:])
                with self.assertRaises(JournalError): changed.summary(j)
            finally: j.close()
