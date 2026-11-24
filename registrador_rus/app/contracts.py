"""RegistroFlowV1: validación local, sin acceso a RUS ni autenticación."""
from dataclasses import dataclass
from datetime import datetime
from hashlib import sha256
from types import MappingProxyType
import re

ORIGINS = frozenset(('ESPERA', 'CUMPLIMIENTO', 'INFORMES_POR_VENCER', 'INFORMES_VENCIDOS'))
DESTINATIONS = frozenset(('ADMINISTRATIVA', 'AL_TRIBUNAL'))
CAPABILITIES = MappingProxyType(dict(read_only=False, prepare_form=False, write_one=False, readback=False))


class ContractError(ValueError):
    """Entrada inválida. Mensajes sin valores de expedientes ni texto."""


def _fields(value, names):
    if type(value) is not dict or set(value) != set(names.split()):
        raise ContractError('Campos ausentes o adicionales')
    return value


def _string(value, *, exact=False):
    if not isinstance(value, str) or not value.strip() or (not exact and value != value.strip()):
        raise ContractError('Cadena requerida o espacios exteriores inválidos')
    try:
        value.encode('utf-8', errors='strict')
    except UnicodeError:
        raise ContractError('Unicode inválido') from None
    if '\x00' in value:
        raise ContractError('Carácter nulo no permitido')
    return value


@dataclass(frozen=True)
class RemoteIdentity:
    system: str
    tribunal_id: str
    causa_id: str
    ingreso_id: str

    def __post_init__(self):
        if self.system != 'SITFA':
            raise ContractError('Sistema fuera de alcance')
        for value in (self.tribunal_id, self.causa_id, self.ingreso_id):
            _string(value)


@dataclass(frozen=True)
class Source:
    tool: str
    work_id: str
    row_ref: str

    def __post_init__(self):
        for value in (self.tool, self.work_id, self.row_ref):
            _string(value)


@dataclass(frozen=True)
class RegistroFlowV1:
    schema: str
    operation_id: str
    source: Source
    remote_identity: RemoteIdentity
    origin: str
    destination: str
    observation_state: str
    approved_text: str
    approved_text_sha256: str
    approved_by: str
    approved_at_utc: str
    approval_version: int

    def __post_init__(self):
        if self.schema != 'RegistroFlowV1':
            raise ContractError('Versión de esquema desconocida')
        if not isinstance(self.source, Source) or not isinstance(self.remote_identity, RemoteIdentity):
            raise ContractError('Procedencia e identidad requeridas')
        for value in (self.operation_id, self.observation_state, self.approved_by):
            _string(value)
        if _string(self.origin) not in ORIGINS or _string(self.destination) not in DESTINATIONS:
            raise ContractError('Origen o destino fuera de alcance')
        text = _string(self.approved_text, exact=True)
        digest = _string(self.approved_text_sha256)
        if not re.fullmatch(r'[0-9a-fA-F]{64}', digest) or sha256(text.encode('utf-8')).hexdigest() != digest.lower():
            raise ContractError('Hash del texto aprobado inválido')
        if type(self.approval_version) is not int or self.approval_version < 1:
            raise ContractError('Versión de aprobación inválida')
        stamp = _string(self.approved_at_utc)
        if not re.fullmatch(r'\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,6})?Z', stamp):
            raise ContractError('Fecha UTC inválida')
        try:
            datetime.fromisoformat(stamp)
        except ValueError:
            raise ContractError('Fecha UTC inválida') from None

    @property
    def tribunal_checked(self):
        return self.destination == 'AL_TRIBUNAL'

    @property
    def cc(self):
        return int(self.tribunal_checked)

    @classmethod
    def from_mapping(cls, value):
        data = dict(_fields(value, 'schema operation_id source remote_identity origin destination observation_state '
                            'approved_text approved_text_sha256 approved_by approved_at_utc approval_version'))
        data['source'] = Source(**_fields(data['source'], 'tool work_id row_ref'))
        data['remote_identity'] = RemoteIdentity(**_fields(data['remote_identity'], 'system tribunal_id causa_id ingreso_id'))
        return cls(**data)

    def require_same_approval(self, current):
        """Comparar con aprobación vigente, previamente obtenida de fuente local autorizada."""
        if not isinstance(current, RegistroFlowV1) or current != self:
            raise ContractError('La aprobación ya no coincide; requiere nueva aprobación')
