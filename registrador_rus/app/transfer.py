"""Intercambio en memoria para revisión local. No guarda archivos ni autoriza intentos."""
from dataclasses import asdict, dataclass
from hashlib import sha256
import json
import re

from .contracts import ContractError, RegistroFlowV1
from .batch_store import selection_digest

MAX_BYTES = 8 * 1024 * 1024


def _json(data):
    return json.dumps(data, ensure_ascii=False, sort_keys=True, separators=(',', ':'), allow_nan=False).encode('utf-8')


def _unique(pairs):
    result = {}
    for k,v in pairs:
        if k in result:
            raise ContractError('CLAVE_DUPLICADA')
        result[k] = v
    return result


def _invalid_constant(_):
    raise ContractError('JSON_INVALIDO')


@dataclass(frozen=True)
class PreviewBatch:
    operations: tuple[RegistroFlowV1, ...]
    source_digest: str
    revision: int

    def __post_init__(self):
        selection_digest(self.operations)
        if (not isinstance(self.source_digest,str) or not re.fullmatch('[0-9a-f]{64}',self.source_digest)
                or type(self.revision) is not int or self.revision < 0):
            raise ContractError('FUENTE_INVALIDA')
        if (len({o.source.work_id for o in self.operations}) != 1 or
                len({o.source.row_ref for o in self.operations}) != len(self.operations)):
            raise ContractError('PROCEDENCIA_DE_LOTE_INVALIDA')

    def encode(self):
        body = dict(schema='FlowBatchPreviewV1',preview_only=True,source_digest=self.source_digest,
                    revision=self.revision,operations=[asdict(o) for o in self.operations])
        data = _json(dict(body=body,sha256=sha256(_json(body)).hexdigest()))
        if len(data) > MAX_BYTES:
            raise ContractError('LOTE_EXCEDE_TRANSPORTE')
        return data

    @classmethod
    def decode(cls, data):
        if type(data) is not bytes or not 0 < len(data) <= MAX_BYTES:
            raise ContractError('LOTE_EXCEDE_TRANSPORTE')
        try:
            envelope = json.loads(data.decode('utf-8'),object_pairs_hook=_unique,parse_constant=_invalid_constant)
            if type(envelope) is not dict or set(envelope) != {'body','sha256'}:
                raise ContractError('SOBRE_INVALIDO')
            body = envelope['body']
            if (type(body) is not dict or set(body) != {'schema','preview_only','source_digest','revision','operations'}
                    or body['schema'] != 'FlowBatchPreviewV1' or body['preview_only'] is not True):
                raise ContractError('SOBRE_INVALIDO')
            if envelope['sha256'] != sha256(_json(body)).hexdigest():
                raise ContractError('HUELLA_DE_LOTE_INVALIDA')
            rows = body['operations']
            if type(rows) is not list or not 1 <= len(rows) <= 400:
                raise ContractError('LOTE_INVALIDO')
            return cls(tuple(RegistroFlowV1.from_mapping(row) for row in rows),body['source_digest'],body['revision'])
        except ContractError:
            raise
        except (ValueError,TypeError,UnicodeError,RecursionError,OverflowError):
            raise ContractError('JSON_INVALIDO') from None
