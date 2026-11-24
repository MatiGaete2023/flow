"""P5: política de ritmo local; no navega ni realiza escrituras remotas."""
from dataclasses import asdict, dataclass
from datetime import datetime, timedelta, timezone
import json
from random import randint


class ScheduleError(ValueError):
    pass


def _integer(value, minimum=0):
    return type(value) is int and minimum <= value <= 2_147_483_647


@dataclass(frozen=True)
class ScheduleConfig:
    enabled: bool = False
    max_attempt_starts: int | None = None
    mode: str = 'none'
    fixed_seconds: int | None = None
    min_seconds: int | None = None
    max_seconds: int | None = None

    def __post_init__(self):
        if type(self.enabled) is not bool or (self.enabled and not _integer(self.max_attempt_starts, 1)):
            raise ScheduleError('CUPO_INVALIDO')
        if not self.enabled and self.max_attempt_starts is not None:
            raise ScheduleError('CUPO_DESACTIVADO_DEBE_SER_NULL')
        a, b, c = self.fixed_seconds, self.min_seconds, self.max_seconds
        valid = ((self.mode == 'none' and a is b is c is None) or
                 (self.mode == 'fixed' and _integer(a) and b is c is None) or
                 (self.mode == 'bounded_variable' and a is None and _integer(b) and _integer(c) and b <= c))
        if not valid:
            raise ScheduleError('INTERVALO_INVALIDO')

    @classmethod
    def from_mapping(cls, data):
        if type(data) is not dict or set(data) != {'schema', 'hourly_limit', 'interval', 'max_concurrent_writes',
                                                  'stop_on_critical_error', 'require_verified_identity', 'require_readback'}:
            raise ScheduleError('CONFIGURACION_INVALIDA')
        if (data['schema'] != 'ScheduleConfigV1' or type(data['max_concurrent_writes']) is not int or
                data['max_concurrent_writes'] != 1 or any(data[k] is not True for k in
                ('stop_on_critical_error', 'require_verified_identity', 'require_readback'))):
            raise ScheduleError('CONTROLES_OBLIGATORIOS')
        h, i = data['hourly_limit'], data['interval']
        if (type(h) is not dict or set(h) != {'enabled', 'max_attempt_starts', 'window_seconds'} or
                type(h['window_seconds']) is not int or h['window_seconds'] != 3600 or type(i) is not dict or
                set(i) != {'mode', 'fixed_seconds', 'min_seconds', 'max_seconds'}):
            raise ScheduleError('CONFIGURACION_INVALIDA')
        return cls(h['enabled'], h['max_attempt_starts'], i['mode'], i['fixed_seconds'], i['min_seconds'], i['max_seconds'])

    def interval_seconds(self, choose=randint):
        if self.mode == 'none':
            return 0
        if self.mode == 'fixed':
            return self.fixed_seconds
        seconds = choose(self.min_seconds, self.max_seconds)
        if not _integer(seconds) or not self.min_seconds <= seconds <= self.max_seconds:
            raise ScheduleError('INTERVALO_FUERA_DE_RANGO')
        return seconds


def _stamp(value):
    if isinstance(value, str):
        value = datetime.fromisoformat(value)
    if not isinstance(value, datetime) or value.tzinfo is None or value.utcoffset() is None:
        raise ScheduleError('RELOJ_INVALIDO')
    return value.astimezone(timezone.utc)


@dataclass(frozen=True)
class Eligibility:
    reason: str
    eligible_at: datetime | None = None

    @property
    def allowed(self):
        return self.reason == 'AHORA'


def evaluate(config, now, attempts, *, status='RUNNING', not_before=None, unresolved=False):
    """Ventana (ahora-3600, ahora]; cuenta todos los intentos persistidos."""
    if status != 'RUNNING':
        return Eligibility(status)
    if unresolved:
        return Eligibility('INTENTO_SIN_RESOLVER')
    now = _stamp(now)
    starts = sorted(_stamp(t) for t in attempts)
    if starts and starts[-1] > now:
        return Eligibility('RELOJ_RETROCEDIO')
    deadlines = []
    if config.enabled:
        window = [t for t in starts if t > now - timedelta(hours=1)]
        if len(window) >= config.max_attempt_starts:
            deadlines.append((window[-config.max_attempt_starts] + timedelta(hours=1), 'ESPERAR_CUPO'))
    if not_before is not None and _stamp(not_before) > now:
        deadlines.append((_stamp(not_before), 'ESPERAR_INTERVALO'))
    if deadlines:
        deadline, reason = max(deadlines)
        return Eligibility(reason, deadline)
    return Eligibility('AHORA', now)


SCHEMA = '''
CREATE TABLE IF NOT EXISTS batch_settings(
    singleton INTEGER PRIMARY KEY CHECK(singleton=1), config TEXT NOT NULL,
    version INTEGER NOT NULL, status TEXT NOT NULL CHECK(status IN ('RUNNING','PAUSED','STOPPED','CRITICAL')),
    not_before TEXT
);
CREATE TABLE IF NOT EXISTS schedule_audit(
    id INTEGER PRIMARY KEY, action TEXT NOT NULL, version INTEGER NOT NULL,
    config TEXT NOT NULL, recorded_at TEXT NOT NULL
);
'''


def _json(config):
    return json.dumps(asdict(config), sort_keys=True, separators=(',', ':'))


class ScheduleStore:
    """Usa la conexión del diario; las mutaciones requieren su BEGIN IMMEDIATE."""
    def __init__(self, db):
        self.db = db
        db.executescript(SCHEMA)
        db.execute("INSERT OR IGNORE INTO batch_settings VALUES(1,?,1,'RUNNING',NULL)", (_json(ScheduleConfig()),))

    def _row(self):
        return self.db.execute('SELECT * FROM batch_settings WHERE singleton=1').fetchone()

    def config(self):
        return ScheduleConfig(**json.loads(self._row()['config']))

    def _require_tx(self):
        if not self.db.in_transaction:
            raise ScheduleError('TRANSACCION_REQUERIDA')

    def _audit(self, action, now):
        row = self._row()
        self.db.execute('INSERT INTO schedule_audit(action,version,config,recorded_at) VALUES(?,?,?,?)',
                        (action, row['version'], row['config'], _stamp(now).isoformat()))

    def configure(self, config, now):
        self._require_tx()
        if not isinstance(config, ScheduleConfig):
            raise ScheduleError('CONFIGURACION_INVALIDA')
        self.db.execute('UPDATE batch_settings SET config=?, version=version+1 WHERE singleton=1', (_json(config),))
        self._audit('CONFIGURACION', now)

    def control(self, action, now):
        self._require_tx()
        current = self._row()['status']
        transitions = {('RUNNING', 'PAUSE'): 'PAUSED', ('PAUSED', 'RESUME'): 'RUNNING',
                       ('RUNNING', 'STOP'): 'STOPPED', ('PAUSED', 'STOP'): 'STOPPED',
                       ('RUNNING', 'CRITICAL'): 'CRITICAL', ('PAUSED', 'CRITICAL'): 'CRITICAL'}
        status = transitions.get((current, action))
        if status is None:
            raise ScheduleError('TRANSICION_NO_PERMITIDA')
        self.db.execute('UPDATE batch_settings SET status=? WHERE singleton=1', (status,))
        self._audit(action, now)

    def eligibility(self, now, *, unresolved=False):
        row = self._row()
        # Leer todos los intentos evita perder los inciertos; indexar en P9 si la medición lo justifica.
        starts = (r[0] for r in self.db.execute('SELECT started_at FROM write_attempts'))
        return evaluate(self.config(), now, starts, status=row['status'], not_before=row['not_before'], unresolved=unresolved)

    def note_closed(self, now, choose=randint):
        """Invocar dentro de la transacción de cierre del recibo (P6), una única vez."""
        self._require_tx()
        deadline = _stamp(now) + timedelta(seconds=self.config().interval_seconds(choose))
        self.db.execute('UPDATE batch_settings SET not_before=? WHERE singleton=1', (deadline.isoformat(),))
        self._audit('CIERRE_INTERVALO', now)
