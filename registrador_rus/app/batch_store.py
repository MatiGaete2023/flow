"""Persistencia de revisión ficticia del lote; no autoriza intentos individuales."""
from dataclasses import asdict
from datetime import datetime, timezone
from hashlib import sha256
import json
import re
from .contracts import RegistroFlowV1, ContractError


SCHEMA = '''CREATE TABLE IF NOT EXISTS simulated_batch_reviews(
    review_id TEXT PRIMARY KEY, selection_digest TEXT NOT NULL,
    count INTEGER NOT NULL CHECK(count BETWEEN 1 AND 400),
    actor TEXT NOT NULL, confirmed_at TEXT NOT NULL,
    simulated INTEGER NOT NULL CHECK(simulated=1)
);'''


def selection_digest(operations):
    if type(operations) is not tuple or not 1 <= len(operations) <= 400 or any(type(op) is not RegistroFlowV1 for op in operations):
        raise ContractError('LOTE_INVALIDO')
    if len({op.operation_id for op in operations}) != len(operations) or len({op.remote_identity for op in operations}) != len(operations):
        raise ContractError('INGRESO_O_INTENCION_REPETIDA_REVISAR')
    data = json.dumps([asdict(op) for op in operations], ensure_ascii=False, sort_keys=True, separators=(',', ':'))
    return sha256(data.encode('utf-8')).hexdigest()


class SimulationBatchStore:
    """Usa una conexión SQLite del consumidor. Sin payload judicial ni autorización real.

    initialize() se invoca al abrir el diario. Las mutaciones requieren la
    transacción del llamador; la revisión es inmutable y no consume permisos.
    """
    def __init__(self, db):
        self.db = db

    def initialize(self):
        if self.db.in_transaction:
            raise ContractError('INICIALIZACION_FUERA_DE_TRANSACCION')
        self.db.execute(SCHEMA)

    def record(self, review_id, operations, *, displayed_digest, actor, confirmed, now):
        if not self.db.in_transaction:
            raise ContractError('TRANSACCION_REQUERIDA')
        if not isinstance(review_id, str) or not re.fullmatch(r'[a-zA-Z0-9_-]{1,128}', review_id):
            raise ContractError('ID_REVISION_INVALIDO')
        digest = selection_digest(operations)
        if confirmed is not True or displayed_digest != digest or not isinstance(actor, str) or not actor.strip():
            raise ContractError('CONFIRMACION_EXACTA_REQUERIDA')
        if not isinstance(now, datetime) or now.tzinfo is None or now.utcoffset() is None:
            raise ContractError('RELOJ_INVALIDO')
        if self.db.execute('SELECT 1 FROM simulated_batch_reviews WHERE review_id=?', (review_id,)).fetchone():
            raise ContractError('REVISION_YA_REGISTRADA')
        self.db.execute('INSERT INTO simulated_batch_reviews VALUES(?,?,?,?,?,1)',
                        (review_id, digest, len(operations), actor, now.astimezone(timezone.utc).isoformat()))
        return digest

    def require_current(self, review_id, operations):
        digest = selection_digest(operations)
        row = self.db.execute('SELECT selection_digest,count,simulated FROM simulated_batch_reviews WHERE review_id=?', (review_id,)).fetchone()
        if row is None or tuple(row) != (digest, len(operations), 1):
            raise ContractError('LOTE_CAMBIADO_O_REVISION_AUSENTE')
        return True
