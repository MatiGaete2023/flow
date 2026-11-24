"""P6b: evidencia y cierre transaccional exclusivos de simulación."""
from datetime import date, timedelta
from hashlib import sha256
import json

from .readback import Snapshot, ReadbackError, compare, review_previous, _utc, METHOD
from .journal_protection import PlainStorage

STATES = ('PREPARADA', 'LISTA', 'ENVIANDO', 'INCIERTA', 'NO_HALLADA', 'AMBIGUA',
          'REVISAR_PREVIAS', 'PENDIENTE_EXCEL', 'COMPROBADA', 'OMITIDA')
UNRESOLVED = "('ENVIANDO','INCIERTA','NO_HALLADA','AMBIGUA')"
MAX_BASELINE_AGE = timedelta(minutes=5)  # Conservador, técnico y solo de simulación.


def operations_sql(name='operations'):
    assert name in ('operations', 'operations_v2')
    return f'''CREATE TABLE IF NOT EXISTS {name}(
    operation_id TEXT PRIMARY KEY, payload TEXT NOT NULL, digest TEXT NOT NULL,
    identity_key TEXT NOT NULL, intent_key TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN {STATES})
);'''


def migrate_operations(db):
    """Conserva intentos, autorizaciones y auditoría al ampliar estados de P4."""
    db.execute('PRAGMA foreign_keys=OFF')
    try:
        db.execute('BEGIN IMMEDIATE')
        sql = db.execute("SELECT sql FROM sqlite_master WHERE type='table' AND name='operations'").fetchone()[0]
        if 'PENDIENTE_EXCEL' not in sql:
            db.execute(operations_sql('operations_v2'))
            db.execute('INSERT INTO operations_v2 SELECT * FROM operations')
            db.execute('DROP TABLE operations')
            db.execute('ALTER TABLE operations_v2 RENAME TO operations')
            db.execute('CREATE INDEX intent_index ON operations(intent_key)')
        if db.execute('PRAGMA foreign_key_check').fetchone():
            raise ReadbackError('DIARIO_INCONSISTENTE')
        db.execute('COMMIT')
    except BaseException:
        if db.in_transaction:
            db.execute('ROLLBACK')
        raise
    finally:
        db.execute('PRAGMA foreign_keys=ON')


SCHEMA = '''
CREATE TABLE IF NOT EXISTS readback_baselines(
    operation_id TEXT PRIMARY KEY REFERENCES operations(operation_id),
    approval_digest TEXT NOT NULL, evidence TEXT NOT NULL, evidence_digest TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS readback_receipts(
    operation_id TEXT PRIMARY KEY REFERENCES write_attempts(operation_id),
    identity_key TEXT NOT NULL, entry_id TEXT NOT NULL, server_timestamp TEXT NOT NULL,
    destination TEXT NOT NULL, approval_digest TEXT NOT NULL, captured_at TEXT NOT NULL,
    simulated INTEGER NOT NULL CHECK(simulated=1), UNIQUE(identity_key, entry_id)
);
'''


def _canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':'))


def _digest(value):
    return sha256(value.encode('utf-8')).hexdigest()


def _snapshot_data(snap):
    ident = dict(tribunal_codigo=snap.identity.tribunal_id, causa_id=snap.identity.causa_id,
                 ingreso_id=snap.identity.ingreso_id)
    return dict(ident, captured_at=snap.captured_at.isoformat(), stage_code=snap.stage_code,
                entries=[dict(ident, entry_id=r.entry_id, texto=r.text, autor=r.author, tipo=r.kind,
                              etapa=r.stage, fecha=r.server_timestamp, origen='centro') for r in snap.entries],
                row_metadata=[dict(entry_id=r.entry_id, state=r.state) for r in snap.entries],
                alcance_metodo=METHOD,
                alcance=('COMPLETA' if snap.entries else 'VACIA_COMPROBADA') if snap.complete else 'PARCIAL')


class ReadbackStore:
    def __init__(self, db, *, storage=None, simulated=1):
        if type(simulated) is not int or simulated not in (0,1):
            raise ReadbackError('AMBITO_DIARIO_INVALIDO')
        self.simulated=simulated
        self.db = db
        self.storage = storage if storage is not None else PlainStorage()
        migrate_operations(db)
        db.executescript(SCHEMA if simulated else SCHEMA.replace('CHECK(simulated=1)','CHECK(simulated=0)'))

    def bind(self, op, digest, raw, *, now, author, stage, stage_code, server_day, require_excel=False):
        if not self.db.in_transaction:
            raise ReadbackError('TRANSACCION_REQUERIDA')
        if (any(not isinstance(x, str) or not x.strip() for x in (author, stage, stage_code)) or
                type(server_day) is not date or type(require_excel) is not bool):
            raise ReadbackError('CONTEXTO_INVALIDO')
        before = Snapshot.from_reader(raw)
        if not timedelta(0) <= _utc(now) - before.captured_at <= MAX_BASELINE_AGE:
            raise ReadbackError('BASE_NO_RECIENTE')
        context = dict(author=author, stage=stage, stage_code=stage_code, server_day=server_day)
        result = review_previous(op, before, **context)
        if result.state == 'SIN_PREVIA':
            context['server_day'] = server_day.isoformat()
            evidence = _canonical(dict(snapshot=_snapshot_data(before), context=context, require_excel=require_excel))
            self.db.execute('''INSERT INTO readback_baselines VALUES(?,?,?,?)
                ON CONFLICT(operation_id) DO UPDATE SET approval_digest=excluded.approval_digest,
                evidence=excluded.evidence,evidence_digest=excluded.evidence_digest''',
                (op.operation_id,digest,self.storage.seal('baseline',op.operation_id,evidence),
                 self.storage.digest('baseline',evidence)))
        else:
            self.db.execute('DELETE FROM readback_baselines WHERE operation_id=?', (op.operation_id,))
        return result

    def baseline(self, operation_id, digest):
        row = self.db.execute('SELECT * FROM readback_baselines WHERE operation_id=?', (operation_id,)).fetchone()
        if row is None or row['approval_digest'] != digest:
            raise ReadbackError('BASELINE_REQUERIDA_O_INVALIDA')
        plain=self.storage.unseal('baseline',operation_id,row['evidence'])
        if self.storage.digest('baseline',plain)!=row['evidence_digest']:
            raise ReadbackError('BASELINE_REQUERIDA_O_INVALIDA')
        evidence=json.loads(plain)
        snap = Snapshot.from_reader(evidence['snapshot'])
        context = evidence['context']
        context['server_day'] = date.fromisoformat(context['server_day'])
        return snap, context, evidence['require_excel']

    def require_recent(self, operation_id, digest, now):
        before, _, _ = self.baseline(operation_id, digest)
        if not timedelta(0) <= _utc(now) - before.captured_at <= MAX_BASELINE_AGE:
            raise ReadbackError('BASE_NO_RECIENTE')

    def resolve(self, op, row, raw, *, now):
        if not self.db.in_transaction:
            raise ReadbackError('TRANSACCION_REQUERIDA')
        before, context, require_excel = self.baseline(op.operation_id, row['digest'])
        attempt = self.db.execute('SELECT started_at FROM write_attempts WHERE operation_id=? AND simulated=?',
                                  (op.operation_id,self.simulated)).fetchone()
        if attempt is None:
            raise ReadbackError('INTENTO_NO_EXISTE')
        after = Snapshot.from_reader(raw)
        if after.captured_at > _utc(now):
            raise ReadbackError('CAPTURA_FUTURA')
        result = compare(op, before, after, attempt_at=attempt['started_at'], **context)
        if result.state != 'COINCIDENCIA_UNICA':
            return result.state, result.reason
        entry = result.entry
        entry_key=self.storage.entry_key(row['identity_key'],entry.entry_id)
        if self.db.execute('SELECT 1 FROM readback_receipts WHERE identity_key=? AND entry_id=?',
                           (row['identity_key'], entry_key)).fetchone():
            return 'AMBIGUA', 'ENTRADA_YA_ATRIBUIDA'
        self.db.execute('INSERT INTO readback_receipts VALUES(?,?,?,?,?,?,?,?)',
                        (op.operation_id, row['identity_key'], entry_key, entry.server_timestamp,
                         op.destination, row['digest'], after.captured_at.isoformat(),self.simulated))
        return ('PENDIENTE_EXCEL' if require_excel else 'COMPROBADA'), result.reason
