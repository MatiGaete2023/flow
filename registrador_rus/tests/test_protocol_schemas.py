import json
from pathlib import Path
import unittest
from jsonschema import Draft202012Validator
from registrador_rus.app.scheduler import ScheduleConfig, ScheduleError
from registrador_rus.app.bridge import Mailbox
from registrador_rus.app.simulation import demo_operations

ROOT = Path(__file__).resolve().parents[1] / 'contracts'


def validator(name):
    schema = json.loads((ROOT / name).read_text(encoding='utf-8'))
    Draft202012Validator.check_schema(schema)
    return Draft202012Validator(schema)


class ProtocolSchemas(unittest.TestCase):
    def test_scheduler(self):
        v = validator('scheduler_v1.schema.json')
        data = dict(schema='ScheduleConfigV1', hourly_limit=dict(enabled=False,max_attempt_starts=None,window_seconds=3600),
                    interval=dict(mode='none',fixed_seconds=None,min_seconds=None,max_seconds=None),
                    max_concurrent_writes=1,stop_on_critical_error=True,require_verified_identity=True,require_readback=True)
        self.assertTrue(v.is_valid(data)); self.assertEqual(ScheduleConfig.from_mapping(data),ScheduleConfig())
        for change in (dict(require_readback=False),dict(hourly_limit=dict(enabled=True,max_attempt_starts=None,window_seconds=3600)),
                       dict(interval=dict(mode='fixed',fixed_seconds=None,min_seconds=None,max_seconds=None))):
            bad=data|change; self.assertFalse(v.is_valid(bad))
            with self.assertRaises(ScheduleError): ScheduleConfig.from_mapping(bad)
        cross=data|dict(interval=dict(mode='bounded_variable',fixed_seconds=None,min_seconds=8,max_seconds=3))
        self.assertTrue(v.is_valid(cross))
        with self.assertRaises(ScheduleError): ScheduleConfig.from_mapping(cross)

    def test_bridge(self):
        v = validator('bridge_commands_v1.schema.json')
        op=demo_operations()[0]
        for command in ('get_capabilities','inspect_current','locate_exact_row','open_bitacora','prepare','readback'):
            box=Mailbox(); box.submit(command,**({} if command=='get_capabilities' else dict(frozen=op,current=op)))
            msg=box.poll(dict(schema='BridgePollV1',session_id=box.session_id))['command']
            self.assertTrue(v.is_valid(msg))
            self.assertFalse(v.is_valid(msg|dict(command='write_one')))
            self.assertFalse(v.is_valid(msg|dict(simulation_only=False)))
            self.assertFalse(v.is_valid(msg|dict(extra=True)))
