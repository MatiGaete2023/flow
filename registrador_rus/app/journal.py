"""P4: diario local de simulación; no adaptador de envío ni conexión a RUS."""
from contextlib import contextmanager
from dataclasses import asdict
from datetime import datetime, timezone
from hashlib import sha256
import json
from pathlib import Path
import sqlite3

from .contracts import RegistroFlowV1


class JournalError(RuntimeError):
    """Fallo de estado sin valores de expediente ni texto en el mensaje."""


def _payload(op):
    if not isinstance(op, RegistroFlowV1):
        raise JournalError('CONTRATO_INVALIDO')
    return json.dumps(asdict(op), ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def _digest(payload):
    return sha256(payload.encode('utf-8')).hexdigest()


SCHEMA = '''
CREATE TABLE IF NOT EXISTS operations(
    operation_id TEXT PRIMARY KEY, payload TEXT NOT NULL, digest TEXT NOT NULL,
    identity_key TEXT NOT NULL, intent_key TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('PREPARADA','LISTA','ENVIANDO','INCIERTA','OMITIDA'))
);
CREATE INDEX IF NOT EXISTS intent_index ON operations(intent_key);
CREATE TABLE IF NOT EXISTS authorizations(
    operation_id TEXT PRIMARY KEY REFERENCES operations(operation_id),
    digest TEXT NOT NULL, human_actor TEXT NOT NULL, authorized_at TEXT NOT NULL,
    consumed INTEGER NOT NULL DEFAULT 0 CHECK(consumed IN (0,1))
);
CREATE TABLE IF NOT EXISTS write_attempts(
    operation_id TEXT PRIMARY KEY REFERENCES operations(operation_id),
    started_at TEXT NOT NULL, simulated INTEGER NOT NULL CHECK(simulated=1)
);
CREATE TABLE IF NOT EXISTS audit(
    id INTEGER PRIMARY KEY, operation_id TEXT NOT NULL REFERENCES operations(operation_id),
    action TEXT NOT NULL, recorded_at TEXT NOT NULL
);
'''


class SimulationJournal:
    """Cada reserva es ficticia; el registro de actor no autentica al funcionario."""

    def __init__(self, path, *, clock=None):
        self.clock = clock or (lambda: datetime.now(timezone.utc))
        self.db = sqlite3.connect(Path(path), timeout=5, isolation_level=None)
        self.db.row_factory = sqlite3.Row
        try:
            self.db.execute('PRAGMA foreign_keys=ON')
            if self.db.execute('PRAGMA journal_mode=WAL').fetchone()[0] != 'wal':
                raise JournalError('WAL_NO_DISPONIBLE')
            self.db.execute('PRAGMA synchronous=FULL')
            self.db.executescript(SCHEMA)
        except BaseException:
            self.db.close()
            raise

    def close(self):
        self.db.close()

    def _now(self):
        dt = self.clock()
        if not isinstance(dt, datetime) or dt.tzinfo is None or dt.utcoffset() is None:
            raise JournalError('RELOJ_INVALIDO')
        return dt.astimezone(timezone.utc).isoformat(timespec='microseconds')

    @contextmanager
    def _tx(self):
        self.db.execute('BEGIN IMMEDIATE')
        try:
            yield
            self.db.execute('COMMIT')
        except BaseException:
            if self.db.in_transaction:
                self.db.execute('ROLLBACK')
            raise

    def _audit(self, op_id, action):
        self.db.execute('INSERT INTO audit(operation_id,action,recorded_at) VALUES(?,?,?)', (op_id, action, self._now()))

    def _row(self, op_id):
        row = self.db.execute('SELECT * FROM operations WHERE operation_id=?', (op_id,)).fetchone()
        if row is None:
            raise JournalError('OPERACION_DESCONOCIDA')
        return row

    def _require_current(self, op_id, current):
        row = self._row(op_id)
        if row['payload'] != _payload(current) or row['digest'] != _digest(row['payload']):
            raise JournalError('APROBACION_CAMBIADA')
        return row

    def freeze(self, op):
        payload = _payload(op)
        ident = json.dumps(asdict(op.remote_identity), sort_keys=True, separators=(',', ':'))
        # Conservador hasta P6: una intención equivalente exige revisión, no reenvío automático.
        intent = _digest(json.dumps([ident, op.approved_text_sha256.lower(), op.destination, op.observation_state],
                                    ensure_ascii=False, separators=(',', ':')))
        with self._tx():
            row = self.db.execute('SELECT * FROM operations WHERE operation_id=?', (op.operation_id,)).fetchone()
            if row:
                if row['payload'] != payload or row['digest'] != _digest(payload):
                    raise JournalError('ID_REUTILIZADO_CON_CAMBIOS')
                return row['state']
            if self.db.execute('SELECT 1 FROM operations WHERE intent_key=? AND state != ? LIMIT 1', (intent, 'OMITIDA')).fetchone():
                raise JournalError('INTENCION_EQUIVALENTE_REVISAR')
            self.db.execute('INSERT INTO operations VALUES(?,?,?,?,?,?)',
                            (op.operation_id, payload, _digest(payload), _digest(ident), intent, 'PREPARADA'))
            self._audit(op.operation_id, 'CONGELADA')
            return 'PREPARADA'

    def mark_ready(self, current):
        """El controlador futuro debe verificar formulario; aquí solo transición ficticia."""
        with self._tx():
            row = self._require_current(current.operation_id, current)
            if row['state'] != 'PREPARADA':
                raise JournalError('ESTADO_NO_PREPARADO')
            self.db.execute('UPDATE operations SET state=? WHERE operation_id=?', ('LISTA', current.operation_id))
            self._audit(current.operation_id, 'LISTA_SIMULADA')

    def authorize_simulation(self, current, *, human_actor, confirmed):
        if confirmed is not True or not isinstance(human_actor, str) or not human_actor.strip():
            raise JournalError('CONFIRMACION_HUMANA_REQUERIDA')
        with self._tx():
            row = self._require_current(current.operation_id, current)
            if row['state'] != 'LISTA':
                raise JournalError('ESTADO_NO_LISTO')
            if self.db.execute('SELECT 1 FROM authorizations WHERE operation_id=?', (current.operation_id,)).fetchone():
                raise JournalError('AUTORIZACION_YA_REGISTRADA')
            self.db.execute('INSERT INTO authorizations VALUES(?,?,?,?,0)',
                            (current.operation_id, row['digest'], human_actor, self._now()))
            self._audit(current.operation_id, 'AUTORIZACION_SIMULADA')

    def reserve_simulated_attempt(self, current):
        """Una sola reserva global sin resolver; persiste antes de devolver control."""
        with self._tx():
            row = self._require_current(current.operation_id, current)
            if self.db.execute('SELECT 1 FROM write_attempts WHERE operation_id=?', (current.operation_id,)).fetchone():
                raise JournalError('INTENTO_PREVIO_SOLO_LECTURA')
            if row['state'] != 'LISTA':
                raise JournalError('ESTADO_NO_LISTO')
            if self.db.execute("SELECT 1 FROM operations WHERE state IN ('ENVIANDO','INCIERTA') LIMIT 1").fetchone():
                raise JournalError('INTENTO_SIN_RESOLVER')
            auth = self.db.execute('SELECT * FROM authorizations WHERE operation_id=?', (current.operation_id,)).fetchone()
            if auth is None or auth['consumed'] or auth['digest'] != row['digest']:
                raise JournalError('AUTORIZACION_REQUERIDA')
            self.db.execute('INSERT INTO write_attempts VALUES(?,?,1)', (current.operation_id, self._now()))
            self.db.execute('UPDATE authorizations SET consumed=1 WHERE operation_id=?', (current.operation_id,))
            self.db.execute('UPDATE operations SET state=? WHERE operation_id=?', ('ENVIANDO', current.operation_id))
            self._audit(current.operation_id, 'RESERVA_SIMULADA')
        return 'ENVIANDO'

    def recover_unresolved(self):
        """Solo lista trabajos que requieren lectura; no modifica estado ni llama a adaptadores."""
        rows = self.db.execute("SELECT operation_id FROM operations WHERE state IN ('ENVIANDO','INCIERTA') ORDER BY operation_id")
        return tuple(row[0] for row in rows)

    def mark_uncertain(self, op_id):
        with self._tx():
            if self._row(op_id)['state'] != 'ENVIANDO':
                raise JournalError('ESTADO_NO_ENVIANDO')
            self.db.execute('UPDATE operations SET state=? WHERE operation_id=?', ('INCIERTA', op_id))
            self._audit(op_id, 'INCIERTA_SOLO_LECTURA')

    def omit(self, op_id):
        with self._tx():
            if self._row(op_id)['state'] not in ('PREPARADA', 'LISTA'):
                raise JournalError('OMISION_NO_PERMITIDA')
            self.db.execute('UPDATE operations SET state=? WHERE operation_id=?', ('OMITIDA', op_id))
            self._audit(op_id, 'OMITIDA')

    def state(self, op_id):
        return self._row(op_id)['state']

    def attempt_count(self):
        return self.db.execute('SELECT COUNT(*) FROM write_attempts').fetchone()[0]
