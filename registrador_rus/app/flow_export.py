"""Adaptador puro Work -> RegistroFlowV1; no red, archivos ni permiso de guardado."""
from dataclasses import asdict, dataclass
from hashlib import sha256
import json
import re
from .contracts import RegistroFlowV1, RemoteIdentity, Source, ContractError

ORIGIN_TYPE = {'ESPERA': 'ESPERA', 'CUMPLIMIENTO': 'CUMPLIMIENTO',
               'INFORMES_POR_VENCER': 'INFORMES', 'INFORMES_VENCIDOS': 'INFORMES'}
COLUMNS = ('SITFA_TRIBUNAL_CODIGO', 'SITFA_CAUSA_RUS', 'SITFA_INGRESO_RUS')


def _fail(code):
    raise ContractError(code)


def _digest(data):
    return sha256(json.dumps(data, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode('utf-8')).hexdigest()


@dataclass(frozen=True)
class ReviewCandidate:
    work_id: str
    revision: int
    row_ref: str
    identity: RemoteIdentity
    origin: str
    text: str
    destination: str
    observation_state: str
    source_digest: str

    @property
    def digest(self):
        return _digest(asdict(self))


def candidate(work, row_ref, *, work_id, origin, destination, observation_state):
    """Lee una fila del Asistente derivada de paquete, no una planilla sin procedencia.

    El filtro específico de Informes es declarado por el revisor y debe validarse
    contra la vista real antes de preparar. No se infiere por fechas.
    """
    if origin not in ORIGIN_TYPE or not isinstance(row_ref, str) or not row_ref:
        _fail('ORIGEN_O_FILA_INVALIDA')
    if destination not in ('ADMINISTRATIVA', 'AL_TRIBUNAL') or not isinstance(observation_state, str) or not observation_state.strip() or observation_state != observation_state.strip():
        _fail('DESTINO_O_ESTADO_INVALIDO')
    revision = getattr(work, 'revision', None)
    capture_digest = getattr(work, 'capture_digest', '')
    if type(revision) is not int or revision < 0 or not isinstance(capture_digest, str) or not re.fullmatch('[0-9a-f]{64}', capture_digest):
        _fail('PAQUETE_O_REVISION_NO_VERIFICABLE')
    if not isinstance(work_id, str) or not work_id or work_id != work_id.strip():
        _fail('TRABAJO_INVALIDO')
    rows = [r for r in work.rows if r.id == row_ref]
    if len(rows) != 1 or row_ref.startswith('calendar:'):
        _fail('FILA_NO_UNICA_O_FUERA_DE_ALCANCE')
    row = rows[0]
    if row.excluded or row.warnings:
        _fail('FILA_REQUIERE_REVISION')
    # sync_state es NURUS_DECISIONES serializado, no un indicador de conflicto.
    if getattr(row, 'sync_state', ''):
        try:
            from nurus.personal.record_edits import decode
            decode(row.sync_state)
        except (ImportError, ValueError):
            _fail('SINCRONIZACION_NO_VALIDADA')
    provenance = getattr(work, 'capture_sources', {}).get(row_ref, {})
    if provenance.get('identity_status') != 'VERIFICADA' or provenance.get('record_type') != ORIGIN_TYPE[origin]:
        _fail('PROCEDENCIA_NO_VERIFICADA')
    values = tuple(row.values.get(c) for c in COLUMNS)
    source_values = tuple(provenance.get(k) for k in ('court_code', 'cause_id', 'ingreso_id'))
    if values != source_values or any(not isinstance(v, str) or not re.fullmatch('[0-9]{1,20}', v) or int(v) == 0 for v in values):
        _fail('IDENTIDAD_NO_COINCIDE')
    text = row.review.get('OBSERVACION', row.observation)
    if not isinstance(text, str) or not text.strip():
        _fail('TEXTO_NO_DISPONIBLE')
    return ReviewCandidate(work_id, revision, row_ref, RemoteIdentity('SITFA', *values), origin, text, destination, observation_state, capture_digest)


def approve_candidate(work, frozen, *, displayed_digest, destination, observation_state, actor, stamp, version, operation_id, confirmed):
    """Se exige reconfirmación del contenido actual; actor declarado, no autenticado.

    La UI futura debe mostrar identidad, texto, destino y Estado exactos antes de llamar.
    """
    if type(frozen) is not ReviewCandidate or confirmed is not True or displayed_digest != frozen.digest:
        _fail('CONFIRMACION_EXPLICITA_REQUERIDA')
    current = candidate(work, frozen.row_ref, work_id=frozen.work_id, origin=frozen.origin, destination=destination, observation_state=observation_state)
    if current != frozen:
        _fail('TRABAJO_CAMBIADO_REVISAR')
    return RegistroFlowV1('RegistroFlowV1', operation_id, Source('FLOW', frozen.work_id, frozen.row_ref), frozen.identity,
                          frozen.origin, destination, observation_state, frozen.text, sha256(frozen.text.encode('utf-8')).hexdigest(),
                          actor, stamp, version)
