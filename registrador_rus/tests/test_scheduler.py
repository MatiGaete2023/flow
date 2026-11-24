from datetime import datetime, timedelta, timezone
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

from registrador_rus.app.contracts import RegistroFlowV1
from registrador_rus.app.journal import JournalError, SimulationJournal
from registrador_rus.app.scheduler import ScheduleConfig, ScheduleError, evaluate
from test_contracts import sample

NOW = datetime(2026, 11, 24, 14, tzinfo=timezone.utc)


class SchedulerTests(unittest.TestCase):
    def test_defaults_and_invalid(self):
        self.assertTrue(evaluate(ScheduleConfig(), NOW, [NOW] * 400).allowed)
        for kwargs in ({'enabled': True}, {'enabled': False, 'max_attempt_starts': 1},
                       {'enabled': True, 'max_attempt_starts': True}, {'mode': 'fixed', 'fixed_seconds': -1},
                       {'mode': 'bounded_variable', 'min_seconds': 3, 'max_seconds': 2}):
            with self.assertRaises(ScheduleError):
                ScheduleConfig(**kwargs)
        data = dict(schema='ScheduleConfigV1', hourly_limit=dict(enabled=False,max_attempt_starts=None,window_seconds=3600),
                    interval=dict(mode='none',fixed_seconds=None,min_seconds=None,max_seconds=None),
                    max_concurrent_writes=1,stop_on_critical_error=True,require_verified_identity=True,require_readback=True)
        self.assertEqual(ScheduleConfig.from_mapping(data), ScheduleConfig())
        with self.assertRaises(ScheduleError):
            ScheduleConfig.from_mapping(data | {'require_readback': False})

    def test_rolling_window(self):
        config = ScheduleConfig(True, 1)
        start = NOW - timedelta(seconds=3599)
        result = evaluate(config, NOW, [start])
        self.assertEqual((result.reason,result.eligible_at), ('ESPERAR_CUPO', NOW + timedelta(seconds=1)))
        self.assertTrue(evaluate(config, NOW, [NOW - timedelta(hours=1)]).allowed)
        config = ScheduleConfig(True, 2)
        self.assertEqual(evaluate(config, NOW,[NOW-timedelta(minutes=20),NOW-timedelta(minutes=10),NOW]).eligible_at,
                         NOW+timedelta(minutes=50))
        self.assertEqual(evaluate(ScheduleConfig(),NOW,[NOW+timedelta(seconds=1)]).reason,'RELOJ_RETROCEDIO')

    def test_interval_and_priority(self):
        self.assertEqual(ScheduleConfig(mode='fixed',fixed_seconds=5).interval_seconds(),5)
        variable = ScheduleConfig(mode='bounded_variable',min_seconds=3,max_seconds=8)
        self.assertEqual(variable.interval_seconds(lambda a,b:a),3)
        self.assertEqual(variable.interval_seconds(lambda a,b:b),8)
        with self.assertRaises(ScheduleError):
            variable.interval_seconds(lambda a,b:9)
        config = ScheduleConfig(True,1)
        deadline = NOW+timedelta(hours=2)
        self.assertEqual(evaluate(config,NOW,[NOW],not_before=deadline).eligible_at,deadline)
        self.assertEqual(evaluate(config,NOW,[NOW],status='CRITICAL',unresolved=True).reason,'CRITICAL')
        self.assertEqual(evaluate(config,NOW,[NOW],unresolved=True).reason,'INTENTO_SIN_RESOLVER')

    def test_persistent_gate_and_attempt(self):
        with TemporaryDirectory() as tmp:
            path=Path(tmp)/'test.sqlite'
            op=RegistroFlowV1.from_mapping(sample())
            j=SimulationJournal(path,clock=lambda:NOW)
            try:
                j.configure_schedule(ScheduleConfig(True,1,mode='fixed',fixed_seconds=7))
                j.freeze(op);j.mark_ready(op);j.authorize_simulation(op,human_actor='demo',confirmed=True)
                j.control_schedule('PAUSE')
                with self.assertRaisesRegex(JournalError,'PAUSED'):
                    j.reserve_simulated_attempt(op)
                self.assertEqual(j.attempt_count(),0)
            finally:
                j.close()
            j=SimulationJournal(path,clock=lambda:NOW)
            try:
                self.assertEqual(j.schedule_eligibility().reason,'PAUSED')
                self.assertEqual(j.schedule.config().max_attempt_starts,1)
                j.control_schedule('RESUME')
                self.assertEqual(j.reserve_simulated_attempt(op),'ENVIANDO')
                self.assertEqual(j.schedule_eligibility().reason,'INTENTO_SIN_RESOLVER')
                j.control_schedule('CRITICAL')
                with self.assertRaises(ScheduleError):
                    j.control_schedule('RESUME')
                self.assertEqual(j.schedule_eligibility().reason,'CRITICAL')
                self.assertEqual(j.attempt_count(),1)
            finally:
                j.close()

    def test_close_interval_persisted(self):
        with TemporaryDirectory() as tmp:
            path=Path(tmp)/'test.sqlite'
            j=SimulationJournal(path,clock=lambda:NOW)
            try:
                j.configure_schedule(ScheduleConfig(mode='bounded_variable',min_seconds=3,max_seconds=8))
                with j._tx():
                    j.schedule.note_closed(NOW,lambda a,b:5)
                j.configure_schedule(ScheduleConfig())
            finally:
                j.close()
            j=SimulationJournal(path,clock=lambda:NOW)
            try:
                self.assertEqual(j.schedule_eligibility().eligible_at,NOW+timedelta(seconds=5))
            finally:
                j.close()
