"""Revisión de un lote Work en dos fases, sin red ni autorización operativa."""
from dataclasses import asdict, dataclass
from hashlib import sha256
import json

from .contracts import ContractError
from .flow_export import ReviewCandidate, candidate, approve_candidate
from .flow_work import load_verified_capture, verify_record
from .batch_store import selection_digest


def _digest(data):
    return sha256(json.dumps(data, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode('utf-8')).hexdigest()


@dataclass(frozen=True)
class ReviewRequest:
    row_ref: str
    origin: str
    destination: str
    observation_state: str


@dataclass(frozen=True)
class WorkBatchReview:
    candidates: tuple[ReviewCandidate, ...]

    def __post_init__(self):
        items = self.candidates
        if type(items) is not tuple or not 1 <= len(items) <= 400 or any(type(c) is not ReviewCandidate for c in items):
            raise ContractError('LOTE_INVALIDO')
        if len({c.row_ref for c in items}) != len(items) or len({c.identity for c in items}) != len(items):
            raise ContractError('INGRESO_O_FILA_REPETIDA_REVISAR')
        if len({(c.work_id,c.revision,c.source_digest) for c in items}) != 1:
            raise ContractError('LOTE_DE_DISTINTAS_FUENTES')

    @property
    def digest(self):
        return _digest([asdict(c) for c in self.candidates])


def build_review(work, requests, *, work_id, report_catalog=None):
    if type(requests) is not tuple or not 1 <= len(requests) <= 400 or any(type(r) is not ReviewRequest for r in requests):
        raise ContractError('SOLICITUD_DE_LOTE_INVALIDA')
    capture = load_verified_capture(work)
    items = []
    for request in requests:
        verify_record(work,capture,request.row_ref,request.origin,report_catalog)
        items.append(candidate(work,request.row_ref,work_id=work_id,origin=request.origin,
                               destination=request.destination,observation_state=request.observation_state))
    return WorkBatchReview(tuple(items))


def confirm_review(work, frozen, requests, *, displayed_digest, confirmed, actor, stamp, version, report_catalog=None):
    """Confirmación del contenido de revisión, no permiso de registrar.
    
    El consumidor debe mostrar el conjunto íntegro y pasar solicitudes vigentes;
    no puede reutilizar la selección congelada en lugar de la selección actual.
    """
    if type(frozen) is not WorkBatchReview or confirmed is not True or displayed_digest != frozen.digest:
        raise ContractError('CONFIRMACION_EXACTA_REQUERIDA')
    current = build_review(work,requests,work_id=frozen.candidates[0].work_id,report_catalog=report_catalog)
    if current != frozen:
        raise ContractError('LOTE_CAMBIADO_REVISAR')
    result = []
    for c in current.candidates:
        # ID estable para mismo contenido/versión, sin reintento implícito.
        operation_id = _digest([current.digest,c.row_ref,version])
        result.append(approve_candidate(work,c,displayed_digest=c.digest,destination=c.destination,
            observation_state=c.observation_state,actor=actor,stamp=stamp,version=version,
            operation_id=operation_id,confirmed=True))
    operations = tuple(result)
    selection_digest(operations)
    return operations
