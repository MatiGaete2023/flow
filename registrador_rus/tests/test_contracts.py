from copy import deepcopy
from hashlib import sha256
import unittest

from registrador_rus.app.contracts import CAPABILITIES, ContractError, ORIGINS, RegistroFlowV1


def sample():
    text = ' Constancia ficticia.\nUnicode: á y 😀. '
    return dict(schema='RegistroFlowV1', operation_id='op-demo',
                source=dict(tool='FLOW', work_id='trabajo-demo', row_ref='fila-demo'),
                remote_identity=dict(system='SITFA', tribunal_id='T-DEMO', causa_id='C-DEMO', ingreso_id='I-DEMO'),
                origin='ESPERA', destination='ADMINISTRATIVA', observation_state='ESTADO-DEMO',
                approved_text=text, approved_text_sha256=sha256(text.encode()).hexdigest(),
                approved_by='usuario-demo', approved_at_utc='2026-11-23T14:00:00Z', approval_version=1)


class ContractTests(unittest.TestCase):
    def test_four_origins_and_destinations(self):
        for origin in ORIGINS:
            for destination in ('ADMINISTRATIVA', 'AL_TRIBUNAL'):
                with self.subTest(origin=origin, destination=destination):
                    data = sample() | dict(origin=origin, destination=destination)
                    op = RegistroFlowV1.from_mapping(data)
                    self.assertEqual(op.tribunal_checked, destination == 'AL_TRIBUNAL')
                    self.assertEqual(op.cc, int(op.tribunal_checked))
                    self.assertEqual(op.approved_text, data['approved_text'])

    def test_missing_and_extra_fields(self):
        for field in sample():
            with self.subTest(field=field), self.assertRaises(ContractError):
                data = sample(); del data[field]; RegistroFlowV1.from_mapping(data)
        for data in (sample() | {'write_one': True}, sample() | {'source': {'tool': 'FLOW'}}):
            with self.assertRaises(ContractError): RegistroFlowV1.from_mapping(data)
        data = sample(); del data['remote_identity']['ingreso_id']
        with self.assertRaises(ContractError): RegistroFlowV1.from_mapping(data)

    def test_invalid_values(self):
        for field, value in [('destination', None), ('destination', 'Gestión Previa'), ('origin', 'CALENDARIO_91'),
                             ('approved_text', 'modificado'), ('approved_text_sha256', 'a'*64),
                             ('approval_version', True), ('approval_version', 0),
                             ('approved_at_utc', '2026-02-30T00:00:00Z'),
                             ('approved_at_utc', '2026-11-23T14:00:00'), ('approved_text', '\ud800')]:
            with self.subTest(field=field), self.assertRaises(ContractError):
                RegistroFlowV1.from_mapping(sample() | {field: value})
        for field in ('tribunal_id', 'causa_id', 'ingreso_id'):
            data = sample(); data['remote_identity'][field] = ' '
            with self.assertRaises(ContractError): RegistroFlowV1.from_mapping(data)

    def test_stale_approval(self):
        data = sample(); op = RegistroFlowV1.from_mapping(data)
        op.require_same_approval(RegistroFlowV1.from_mapping(deepcopy(data)))
        for changed in (dict(destination='AL_TRIBUNAL'), dict(approval_version=2), dict(observation_state='OTRO')):
            with self.assertRaises(ContractError):
                op.require_same_approval(RegistroFlowV1.from_mapping(data | changed))
        data['remote_identity']['ingreso_id'] = 'I-OTRO'
        with self.assertRaises(ContractError): op.require_same_approval(RegistroFlowV1.from_mapping(data))

    def test_capabilities_disabled(self):
        self.assertFalse(any(CAPABILITIES.values()))
        with self.assertRaises(TypeError): CAPABILITIES['write_one'] = True


if __name__ == '__main__':
    unittest.main()
