"""P4: diario local de simulación; no adaptador de envío ni conexión a RUS."""
from contextlib import contextmanager
from dataclasses import asdict
from datetime import datetime, timezone
from hashlib import sha256
import json
from pathlib import Path
import sqlite3

from .contracts import RegistroFlowV1
from .scheduler import ScheduleStore
from .readback import ReadbackError
from .readback_store import ReadbackStore, UNRESOLVED, operations_sql
from .journal_protection import check_path, initialize_storage
from .private_demo_directory import validate_private_demo


class JournalError(RuntimeError):
    """Fallo de estado sin valores de expediente ni texto en el mensaje."""


def _payload(op):
    if not isinstance(op, RegistroFlowV1):
        raise JournalError('CONTRATO_INVALIDO')
    return json.dumps(asdict(op), ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def _digest(payload):
    return sha256(payload.encode('utf-8')).hexdigest()


SCHEMA = operations_sql() + '''
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


class _JournalCore:
    """Núcleo transaccional compartido; ámbito fijado por las subclases."""
    _simulated = 1

    def __init__(self, path, *, clock=None, protected_demo=False):
        pilot = self._simulated == 0
        crypt = check_path(path, protected_demo, pilot=pilot)
        self._private_path = Path(path) if protected_demo else None
        self.clock = clock or (lambda: datetime.now(timezone.utc))
        self.db = sqlite3.connect(Path(path), timeout=5, isolation_level=None)
        self.db.row_factory = sqlite3.Row
        try:
            self.storage = initialize_storage(self.db, protected_demo, crypt, pilot=pilot)
            self.db.execute('PRAGMA foreign_keys=ON')
            if self.storage.protected:
                self.db.execute('PRAGMA temp_store=MEMORY')
            if self.db.execute('PRAGMA journal_mode=WAL').fetchone()[0] != 'wal':
                raise JournalError('WAL_NO_DISPONIBLE')
            self.db.execute('PRAGMA synchronous=FULL')
            self.db.executescript(SCHEMA if self._simulated else SCHEMA.replace('CHECK(simulated=1)','CHECK(simulated=0)'))
            self.schedule = ScheduleStore(self.db)
            self.readback = ReadbackStore(self.db, storage=self.storage, simulated=self._simulated)
            if self._private_path is not None:
                validate_private_demo(self._private_path)
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
        if self._private_path is not None:
            validate_private_demo(self._private_path)
        self.db.execute('BEGIN IMMEDIATE')
        try:
            yield
            if self._private_path is not None:
                validate_private_demo(self._private_path)
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
        self.storage.require_operation(current)
        stored = self.storage.unseal('payload',op_id,row['payload'])
        if stored != _payload(current) or row['digest'] != self.storage.digest('approval',stored):
            raise JournalError('APROBACION_CAMBIADA')
        return row

    def freeze(self, op):
        payload = _payload(op)
        self.storage.require_operation(op)
        ident = json.dumps(asdict(op.remote_identity), sort_keys=True, separators=(',', ':'))
        # Conservador hasta P6: una intención equivalente exige revisión, no reenvío automático.
        intent = self.storage.digest('intent',json.dumps([ident, op.approved_text_sha256.lower(), op.destination, op.observation_state],
                                    ensure_ascii=False, separators=(',', ':')))
        with self._tx():
            row = self.db.execute('SELECT * FROM operations WHERE operation_id=?', (op.operation_id,)).fetchone()
            if row:
                if self.storage.unseal('payload',op.operation_id,row['payload']) != payload or row['digest'] != self.storage.digest('approval',payload):
                    raise JournalError('ID_REUTILIZADO_CON_CAMBIOS')
                return row['state']
            if self.db.execute('SELECT 1 FROM operations WHERE intent_key=? AND state != ? LIMIT 1', (intent, 'OMITIDA')).fetchone():
                raise JournalError('INTENCION_EQUIVALENTE_REVISAR')
            self.db.execute('INSERT INTO operations VALUES(?,?,?,?,?,?)',
                            (op.operation_id, self.storage.seal('payload',op.operation_id,payload),
                             self.storage.digest('approval',payload),self.storage.digest('identity',ident),intent,'PREPARADA'))
            self._audit(op.operation_id, 'CONGELADA')
            return 'PREPARADA'

    def mark_ready(self, current):
        """El controlador futuro debe verificar formulario; aquí solo transición ficticia."""
        with self._tx():
            row = self._require_current(current.operation_id, current)
            if row['state'] != 'PREPARADA':
                raise JournalError('ESTADO_NO_PREPARADO')
            self.db.execute('UPDATE operations SET state=? WHERE operation_id=?', ('LISTA', current.operation_id))
            self._audit(current.operation_id, 'LISTA_SIMULADA' if self._simulated else 'LISTA_PILOTO')

    def _authorize(self, current, *, human_actor, confirmed):
        if confirmed is not True or not isinstance(human_actor, str) or not human_actor.strip():
            raise JournalError('CONFIRMACION_HUMANA_REQUERIDA')
        with self._tx():
            row = self._require_current(current.operation_id, current)
            if row['state'] != 'LISTA':
                raise JournalError('ESTADO_NO_LISTO')
            if self.db.execute('SELECT 1 FROM authorizations WHERE operation_id=?', (current.operation_id,)).fetchone():
                raise JournalError('AUTORIZACION_YA_REGISTRADA')
            self.db.execute('INSERT INTO authorizations VALUES(?,?,?,?,0)',
                            (current.operation_id,row['digest'],self.storage.seal('actor',current.operation_id,human_actor),self._now()))
            self._audit(current.operation_id, 'AUTORIZACION_SIMULADA' if self._simulated else 'AUTORIZACION_PILOTO')

    def _reserve_attempt(self, current, *, expected_evidence_digest=None, record_reservation=None):
        """Una sola reserva global sin resolver; persiste antes de devolver control."""
        with self._tx():
            row = self._require_current(current.operation_id, current)
            if self.db.execute('SELECT 1 FROM write_attempts WHERE operation_id=?', (current.operation_id,)).fetchone():
                raise JournalError('INTENTO_PREVIO_SOLO_LECTURA')
            if row['state'] != 'LISTA':
                raise JournalError('ESTADO_NO_LISTO')
            if self.db.execute(f"SELECT 1 FROM operations WHERE state IN {UNRESOLVED} LIMIT 1").fetchone():
                raise JournalError('INTENTO_SIN_RESOLVER')
            auth = self.db.execute('SELECT * FROM authorizations WHERE operation_id=?', (current.operation_id,)).fetchone()
            if auth is None or auth['consumed'] or auth['digest'] != row['digest']:
                raise JournalError('AUTORIZACION_REQUERIDA')
            if self.storage.protected and not self.storage.unseal('actor',current.operation_id,auth['human_actor']).strip():
                raise JournalError('AUTORIZACION_INVALIDA')
            now = self._now()
            eligibility = self.schedule.eligibility(now)
            if not eligibility.allowed:
                raise JournalError(eligibility.reason)
            try:
                self.readback.require_recent(current.operation_id, row['digest'], now)
                if expected_evidence_digest is not None:
                    evidence=self.db.execute('SELECT evidence_digest FROM readback_baselines WHERE operation_id=?',(current.operation_id,)).fetchone()
                    if evidence is None or evidence[0]!=expected_evidence_digest:
                        raise ReadbackError('LECTURA_PREVIA_CAMBIADA')
            except ReadbackError as exc:
                raise JournalError(str(exc)) from None
            self.db.execute('INSERT INTO write_attempts VALUES(?,?,?)', (current.operation_id, now,self._simulated))
            self.db.execute('UPDATE authorizations SET consumed=1 WHERE operation_id=?', (current.operation_id,))
            self.db.execute('UPDATE operations SET state=? WHERE operation_id=?', ('ENVIANDO', current.operation_id))
            self._audit(current.operation_id, 'RESERVA_SIMULADA' if self._simulated else 'RESERVA_PILOTO')
            if record_reservation is not None:
                record_reservation(row,now)
        return 'ENVIANDO'

    def bind_baseline(self, current, raw, **context):
        with self._tx():
            row = self._require_current(current.operation_id, current)
            if row['state'] != 'LISTA':
                raise JournalError('ESTADO_NO_LISTO')
            try:
                result = self.readback.bind(current, row['digest'], raw, now=self._now(), **context)
            except ReadbackError:
                self.db.execute('DELETE FROM readback_baselines WHERE operation_id=?', (current.operation_id,))
                self._audit(current.operation_id, 'BASELINE_INVALIDADA')
                return 'INCIERTA'
            if result.state in ('REUTILIZABLE', 'REVISAR_PREVIAS'):
                self.db.execute("UPDATE operations SET state='REVISAR_PREVIAS' WHERE operation_id=?", (current.operation_id,))
            self._audit(current.operation_id, result.reason)
            return result.state

    def _resolve(self, current, raw):
        with self._tx():
            row = self._require_current(current.operation_id, current)
            if row['state'] == 'AMBIGUA':
                raise JournalError('AMBIGUEDAD_REQUIERE_REVISION_HUMANA')
            if row['state'] not in ('ENVIANDO', 'INCIERTA', 'NO_HALLADA'):
                raise JournalError('ESTADO_NO_RECUPERABLE')
            now = self._now()
            try:
                state, reason = self.readback.resolve(current, row, raw, now=now)
            except ReadbackError:
                state, reason = 'INCIERTA', 'EVIDENCIA_INSUFICIENTE'
            self.db.execute('UPDATE operations SET state=? WHERE operation_id=?', (state, current.operation_id))
            self._audit(current.operation_id, reason)
            if state in ('COMPROBADA', 'PENDIENTE_EXCEL'):
                self.schedule.note_closed(now)
            return state

    def configure_schedule(self, config):
        with self._tx():
            self.schedule.configure(config, self._now())

    def control_schedule(self, action):
        with self._tx():
            self.schedule.control(action, self._now())

    def schedule_eligibility(self):
        with self._tx():
            return self.schedule.eligibility(self._now(), unresolved=bool(self.recover_unresolved()))

    def recover_unresolved(self):
        """Solo lista trabajos que requieren lectura; no modifica estado ni llama a adaptadores."""
        rows = self.db.execute(f"SELECT operation_id FROM operations WHERE state IN {UNRESOLVED} ORDER BY operation_id")
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


class SimulationJournal(_JournalCore):
    """Cada reserva es ficticia; el registro de actor no autentica al funcionario."""
    def authorize_simulation(self,current,*,human_actor,confirmed):
        return self._authorize(current,human_actor=human_actor,confirmed=confirmed)

    def reserve_simulated_attempt(self,current):
        return self._reserve_attempt(current)

    def resolve_simulated(self,current,raw):
        return self._resolve(current,raw)
