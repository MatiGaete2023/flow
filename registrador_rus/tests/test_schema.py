"""Pruebas de desarrollo: jsonschema es opcional para ejecución del módulo."""
import json
from pathlib import Path
import unittest

from jsonschema import Draft202012Validator, FormatChecker
from test_contracts import sample
from registrador_rus.app.contracts import RegistroFlowV1, ContractError


class SchemaTests(unittest.TestCase):
    def test_schema_and_examples(self):
        path = Path(__file__).resolve().parents[1] / 'contracts' / 'registro_flow_v1.schema.json'
        schema = json.loads(path.read_text(encoding='utf-8'))
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator(schema, format_checker=FormatChecker())
        self.assertTrue(validator.is_valid(sample()))
        for value in (sample() | {'destination': 'ESTADO-DEMO'}, sample() | {'write_one': True},
                      sample() | {'approval_version': False}, sample() | {'approved_text_sha256': 'marcador'},
                      sample() | {'approved_at_utc': '2026-02-30T14:00:00Z'}):
            self.assertFalse(validator.is_valid(value))
            with self.assertRaises(ContractError): RegistroFlowV1.from_mapping(value)
        data = sample(); del data['remote_identity']['ingreso_id']
        self.assertFalse(validator.is_valid(data))
        with self.assertRaises(ContractError): RegistroFlowV1.from_mapping(data)

    def test_hash_requires_runtime_validation(self):
        path = Path(__file__).resolve().parents[1] / 'contracts' / 'registro_flow_v1.schema.json'
        validator = Draft202012Validator(json.loads(path.read_text(encoding='utf-8')))
        data = sample() | {'approved_text': 'Texto ficticio modificado'}
        self.assertTrue(validator.is_valid(data))
        with self.assertRaises(ContractError): RegistroFlowV1.from_mapping(data)
