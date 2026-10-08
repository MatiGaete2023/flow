"""Registro recuperable de observaciones. El adaptador RUS debe aportar evidencia real."""
from contextlib import contextmanager
from dataclasses import asdict, dataclass
from datetime import date, datetime
from hashlib import sha256
import json
from pathlib import Path
import re
import sqlite3
from uuid import uuid4

from .bitacoras import cc_for_type, validate_period


def _json(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def _digest(value):
    return sha256(_json(value).encode('utf-8')).hexdigest()


def _moment(value):
    result = datetime.fromisoformat(value)
    if result.tzinfo is None:
        raise ValueError('La evidencia de consulta debe indicar hora y zona.')
    return result


def _source_id(value):
    return '' if value is None else str(value)


def _matches(entry, intent):
    return (entry.author_id == intent.author_id
            and entry.text.replace('\r\n', '\n') == intent.text.replace('\r\n', '\n')
            and entry.type == intent.type and entry.state == intent.state
            and entry.send_to_tribunal is intent.send_to_tribunal)


def _receipt(data, intent, entry, *, new):
    data.update(state='PENDIENTE_EXCEL', verified=True, new_registration=new,
                remote_entry_id=entry.entry_id, registered_at=entry.registered_at,
                author=entry.author_id, type=intent.type, cc=cc_for_type(intent.type), text=intent.text,
                remote_text=entry.text, tribunal=intent.tribunal, rit=intent.identity.rit,
                ingreso_id=intent.identity.ingreso_id)


@dataclass(frozen=True)
class RemoteIdentity:
    tribunal_codigo: str
    causa_id: str
    ingreso_id: str
    persona_id: str
    centro_id: str
    rit: str

    def validate(self):
        if not all(re.fullmatch(r'\d{1,20}', str(value)) and int(value) > 0 for value in (
            self.tribunal_codigo, self.causa_id, self.ingreso_id, self.persona_id, self.centro_id
        )) or not re.fullmatch(r'[A-Z]+-\d+-\d{4}', self.rit):
            raise ValueError('El registro requiere identidad real completa y RIT comprobado.')

    @classmethod
    def from_row(cls, work, row):
        if row.values.get('SITFA_VINCULO_ESTADO') != 'Vinculado desde respuesta actual':
            raise ValueError('Falta el vínculo comprobado del ingreso; no basta el RIT o el ID local.')
        keys = ('SITFA_TRIBUNAL_CODIGO', 'SITFA_CAUSA_RUS', 'SITFA_INGRESO_RUS',
                'SITFA_PERSONA_RUS', 'SITFA_CENTRO_RUS')
        parts = [_source_id(row.values.get(key, '')) for key in keys]
        result = cls(*parts, str(row.values.get(work.mapping.get('rit', ''), '')).strip().upper())
        result.validate()
        return result


@dataclass(frozen=True)
class Intent:
    identity: RemoteIdentity
    text: str
    type: str
    state: str
    send_to_tribunal: bool
    author_id: str
    source_sha256: str
    source_sheet: str
    source_row: int
    tribunal: str = ''
    stage: str = ''
    antiguo: str = ''
    modality: str = ''

    def validate(self):
        self.identity.validate()
        if not isinstance(self.text, str) or not self.text.strip():
            raise ValueError('La observación del Excel está vacía.')
        if cc_for_type(self.type) is None or not self.state.strip():
            raise ValueError('Selecciona tipo y estado explícitos para esta operación.')
        if type(self.send_to_tribunal) is not bool or not self.author_id.strip():
            raise ValueError('Falta destino explícito o identidad del usuario de la sesión.')
        if not re.fullmatch('[0-9a-f]{64}', self.source_sha256) or not self.source_sheet or self.source_row < 1:
            raise ValueError('Falta la procedencia de la observación.')
        if not all(re.fullmatch(r'\d{1,20}', value) for value in (self.stage, self.antiguo, self.modality)):
            raise ValueError('Falta comprobar etapa, modalidad o indicador del registro antiguo.')


def intent_from_excel(work, row, *, type, state, send_to_tribunal, author_id):
    """No usa la propuesta del motor como si fuera texto aprobado del Excel."""
    if not getattr(work, 'external_input', False) or 'OBSERVACION' not in row.review:
        raise ValueError('Carga el Excel de registro: se necesita su columna OBSERVACION efectiva.')
    text = row.review['OBSERVACION']
    if not isinstance(text, str):
        raise ValueError('La observación debe ser texto, sin conversión automática.')
    from nurus.rus.columns import normalize
    original = [value for key, value in row.values.items()
                if normalize(key) in ('observacion', 'observaciones')]
    if len(original) != 1 or original[0] != text:
        raise ValueError('El texto cambió en CSMP: guarda y vuelve a cargar el Excel de registro.')
    if sha256(Path(work.path).read_bytes()).hexdigest() != work.source_hash:
        raise ValueError('El Excel cambió desde la lectura: vuelve a cargarlo antes de preparar el registro.')
    from .outputs import value
    result = Intent(RemoteIdentity.from_row(work, row), text, type, state,
                    send_to_tribunal, author_id, work.source_hash, work.sheet, row.source_row,
                    str(value(work, row, 'tribunal')),
                    _source_id(row.values.get('SITFA_ETAPA_RUS', '')),
                    _source_id(row.values.get('SITFA_ANTIGUO_RUS', '')),
                    _source_id(row.values.get('SITFA_MODALIDAD', '')))
    result.validate()
    return result


@dataclass(frozen=True)
class RemoteEntry:
    entry_id: str
    registered_at: str
    author_id: str
    text: str
    type: str
    state: str
    send_to_tribunal: bool


@dataclass(frozen=True)
class FormEvidence:
    """Campos releídos y límite observado; None significa ausencia comprobada de maxlength."""
    intent: Intent
    max_text_units: int | None
    observed_at: str
    source_sha256: str

    def validate(self, intent, now):
        if self.intent != intent:
            raise ValueError('Los campos efectivos del formulario no coinciden con la intención preparada.')
        self.intent.validate()
        observed = _moment(self.observed_at)
        if observed > now or (now - observed).total_seconds() > 120:
            raise ValueError('El formulario requiere una lectura reciente antes del guardado.')
        if not re.fullmatch('[0-9a-f]{64}', self.source_sha256):
            raise ValueError('Falta evidencia del formulario y su límite efectivo.')
        limit = self.max_text_units
        if limit is not None and (type(limit) is not int or limit < 0):
            raise ValueError('El límite observado del formulario es inválido.')
        if limit is not None and len(intent.text.encode('utf-16-le')) // 2 > limit:
            raise ValueError(f'La observación excede el límite observado de {limit} unidades UTF-16.')


@dataclass(frozen=True)
class Snapshot:
    identity: RemoteIdentity
    author_id: str
    observed_at: str
    source_sha256: str
    start: date
    end: date
    complete: bool
    entries: tuple[RemoteEntry, ...]
    stage: str = ''
    antiguo: str = ''
    modality: str = ''

    def validate(self, intent, now):
        if self.identity != intent.identity or self.author_id != intent.author_id:
            raise ValueError('La ventana o el usuario no corresponden al registro preparado.')
        if (self.stage, self.antiguo, self.modality) != (intent.stage, intent.antiguo, intent.modality):
            raise ValueError('El contexto de etapa o modalidad cambió; vuelve a comprobar el ingreso.')
        observed = _moment(self.observed_at)
        if observed > now or (now - observed).total_seconds() > 120:
            raise ValueError('La lectura es futura o antigua; vuelve a consultar la bitácora.')
        validate_period(self.start, self.end, today=now.date())
        if not self.start <= now.date() <= self.end or self.complete is not True:
            raise ValueError('La lectura completa debe incluir el día de la gestión.')
        if not re.fullmatch('[0-9a-f]{64}', self.source_sha256):
            raise ValueError('Falta la fuente comprobable de la bitácora.')
        ids = [entry.entry_id for entry in self.entries]
        if any(not value for value in ids) or len(set(ids)) != len(ids):
            raise ValueError('La lectura contiene IDs ausentes o repetidos.')


class Journal:
    """SQLite guarda la intención antes del POST y conserva cada transición."""
    def __init__(self, path, *, clock=None):
        self.path = Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.clock = clock or (lambda: datetime.now().astimezone())
        with self._transaction() as db:
            version = db.execute('PRAGMA user_version').fetchone()[0]
            if version == 0:
                if db.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchone():
                    raise ValueError('El archivo no es un diario de registro CSMP vacío.')
                db.execute('CREATE TABLE operations (id TEXT PRIMARY KEY, operation_key TEXT UNIQUE NOT NULL, '
                           'identity_key TEXT NOT NULL, state TEXT NOT NULL, data TEXT NOT NULL)')
                db.execute('CREATE TABLE events (sequence INTEGER PRIMARY KEY, operation_id TEXT NOT NULL, '
                           'occurred_at TEXT NOT NULL, state TEXT NOT NULL, detail TEXT NOT NULL)')
                db.execute('PRAGMA user_version=1')
            elif version != 1:
                raise ValueError('Versión de diario de registro incompatible.')

    @contextmanager
    def _transaction(self):
        db = sqlite3.connect(self.path, timeout=5)
        try:
            db.execute('PRAGMA synchronous=FULL')
            db.execute('BEGIN IMMEDIATE')
            yield db
            db.commit()
        except BaseException:
            db.rollback()
            raise
        finally:
            db.close()

    def _save(self, db, data, detail):
        db.execute('UPDATE operations SET state=?, data=? WHERE id=?',
                   (data['state'], _json(data), data['operation_id']))
        db.execute('INSERT INTO events(operation_id,occurred_at,state,detail) VALUES(?,?,?,?)',
                   (data['operation_id'], self.clock().isoformat(), data['state'], detail))

    @staticmethod
    def _get(db, operation_id):
        row = db.execute('SELECT data FROM operations WHERE id=?', (operation_id,)).fetchone()
        if row is None:
            raise ValueError('La operación no existe en este diario.')
        return json.loads(row[0])

    def get(self, operation_id):
        with self._transaction() as db:
            return self._get(db, operation_id)

    def list(self):
        with self._transaction() as db:
            return [json.loads(row[0]) for row in db.execute('SELECT data FROM operations ORDER BY rowid')]

    def prepare(self, intent):
        intent.validate()
        now = self.clock()
        payload = asdict(intent)
        facts = {key: payload[key] for key in ('identity', 'text', 'type', 'state', 'send_to_tribunal', 'author_id')}
        operation_key = _digest([now.date().isoformat(), facts])
        identity_key = _digest({key: value for key, value in payload['identity'].items() if key != 'rit'})
        with self._transaction() as db:
            existing = db.execute('SELECT id FROM operations WHERE operation_key=?', (operation_key,)).fetchone()
            if existing:
                return self._get(db, existing[0])
            pending = db.execute("SELECT id FROM operations WHERE identity_key=? AND state NOT IN "
                                 "('COMPROBADA','PENDIENTE_EXCEL')", (identity_key,)).fetchone()
            if pending:
                raise ValueError('Este ingreso tiene una operación pendiente. Consúltala antes de preparar otra.')
            operation_id = uuid4().hex
            data = {'operation_id': operation_id, 'kind': 'rus_observation', 'state': 'PREPARADA',
                    'verified': False, 'intent': payload, 'created_at': now.isoformat()}
            db.execute('INSERT INTO operations VALUES(?,?,?,?,?)',
                       (operation_id, operation_key, identity_key, data['state'], _json(data)))
            self._save(db, data, 'Intención conservada; todavía no enviada a RUS.')
            return data

    def begin(self, operation_id, baseline, form_evidence=None):
        now = self.clock()
        with self._transaction() as db:
            data = self._get(db, operation_id)
            if data['state'] != 'PREPARADA':
                raise ValueError('La operación ya intentó enviarse; solo puede consultarse para recuperación.')
            intent = restore_intent(data)
            baseline.validate(intent, now)
            if form_evidence is not None:
                form_evidence.validate(intent, now)
                data['form_sha256'] = form_evidence.source_sha256
                data['form_observed_at'] = form_evidence.observed_at
                data['form_max_text_units'] = form_evidence.max_text_units
            data.update(state='ENVIANDO', started_at=now.isoformat(),
                        baseline_ids=[entry.entry_id for entry in baseline.entries],
                        baseline_sha256=baseline.source_sha256)
            self._save(db, data, 'Estado persistido antes del único intento de guardar.')
            return data

    def uncertain(self, operation_id, reason='Envío pendiente de comprobación mediante lectura.'):
        with self._transaction() as db:
            data = self._get(db, operation_id)
            if data['state'] == 'ENVIANDO':
                data['state'] = 'INCIERTA'
                self._save(db, data, reason)
            return data

    def check_existing(self, operation_id, snapshot):
        """Una entrada idéntica del día se reutiliza; no constituye una gestión nueva."""
        now = self.clock()
        with self._transaction() as db:
            data = self._get(db, operation_id)
            if data['state'] not in ('PREPARADA', 'REVISAR_PREVIAS'):
                raise ValueError('Esta operación ya salió de la preparación; no se vuelve a enviar.')
            intent = restore_intent(data)
            snapshot.validate(intent, now)
            found = []
            for entry in snapshot.entries:
                if not _matches(entry, intent):
                    continue
                try:
                    day = datetime.fromisoformat(entry.registered_at).date()
                except (TypeError, ValueError) as exc:
                    raise ValueError('Una observación coincidente tiene fecha ilegible; compruébala antes de guardar.') from exc
                if day == now.date():
                    found.append(entry)
            if len(found) == 1:
                _receipt(data, intent, found[0], new=False)
                data['checked_at'] = snapshot.observed_at
                data['check_sha256'] = snapshot.source_sha256
                self._save(db, data, 'Entrada idéntica del día ya existente; no se envió una observación nueva.')
            elif len(found) > 1:
                data['state'] = 'REVISAR_PREVIAS'
                self._save(db, data, 'Varias entradas previas coinciden; no se envió una observación nueva.')
            elif data['state'] == 'REVISAR_PREVIAS':
                data['state'] = 'PREPARADA'
                self._save(db, data, 'La nueva lectura no contiene coincidencias previas del día; falta preparar el envío.')
            return data

    def reconcile(self, operation_id, snapshot):
        now = self.clock()
        with self._transaction() as db:
            data = self._get(db, operation_id)
            if data.get('verified') is True:
                return data
            if data['state'] not in ('ENVIANDO', 'INCIERTA', 'NO_HALLADA', 'AMBIGUA'):
                raise ValueError('No hay un intento de envío que comprobar.')
            intent = restore_intent(data)
            snapshot.validate(intent, now)
            if _moment(snapshot.observed_at) < _moment(data['started_at']):
                raise ValueError('La lectura precede al intento de guardado.')
            found = [entry for entry in snapshot.entries
                     if entry.entry_id not in data['baseline_ids'] and _matches(entry, intent)]
            data['checked_at'] = snapshot.observed_at
            data['check_sha256'] = snapshot.source_sha256
            if len(found) == 1:
                entry = found[0]
                try:
                    day = datetime.fromisoformat(entry.registered_at).date()
                except (TypeError, ValueError) as exc:
                    raise ValueError('El guardado leído no tiene una fecha interpretable.') from exc
                if not _moment(data['started_at']).date() <= day <= now.date():
                    raise ValueError('La fecha leída no pertenece al período del intento de guardado.')
                _receipt(data, intent, entry, new=True)
                detail = 'Entrada nueva releída; falta devolver la fecha a Excel.'
            else:
                data['state'] = 'AMBIGUA' if found else 'NO_HALLADA'
                detail = ('Varias entradas coinciden; requiere revisión.' if found else
                          'La lectura no identifica una entrada nueva; no se repetirá el envío automáticamente.')
            self._save(db, data, detail)
            return data

    def complete_excel(self, operation_id, destination, digest):
        return self.complete_excel_many([operation_id], destination, digest)[0]

    def complete_excel_many(self, operation_ids, destination, digest):
        """Uso interno tras releer la copia: todas las devoluciones se confirman juntas."""
        path = Path(destination).resolve()
        if not path.is_file() or sha256(path.read_bytes()).hexdigest() != digest:
            raise ValueError('El Excel de devolución no coincide con el archivo comprobado.')
        with self._transaction() as db:
            records = [self._get(db, key) for key in operation_ids]
            if any(data.get('verified') is not True for data in records):
                raise ValueError('No se devuelve una fecha sin registro comprobado en RUS.')
            for data in records:
                data.update(state='COMPROBADA', excel_path=str(path), excel_sha256=digest)
                self._save(db, data, 'Copia de Excel generada y releída por identidad.')
            return records


def restore_intent(data):
    values = dict(data['intent'])
    values['identity'] = RemoteIdentity(**values['identity'])
    result = Intent(**values)
    result.validate()
    return result


def submit(journal, operation_id, adapter):
    """prepare_form fija y relee identidad, texto, tipo, estado y destino explícitos."""
    data = journal.get(operation_id)
    if data['state'] != 'PREPARADA':
        return recover(journal, operation_id, adapter)
    intent = restore_intent(data)
    baseline = adapter.read(intent.identity)
    baseline.validate(intent, journal.clock())
    data = journal.check_existing(operation_id, baseline)
    if data['state'] != 'PREPARADA':
        return data
    form = adapter.prepare_form(intent)
    if not isinstance(form, FormEvidence):
        raise ValueError('El adaptador debe aportar campos releídos y el límite observado del formulario.')
    form.validate(intent, journal.clock())
    journal.begin(operation_id, baseline, form)
    try:
        adapter.save(form.intent)
    except Exception:
        journal.uncertain(operation_id, 'No se confirmó el envío; consultar antes de continuar.')
    else:
        journal.uncertain(operation_id)
    return recover(journal, operation_id, adapter)


def recover(journal, operation_id, adapter):
    """Nunca ejecuta save, incluso cuando la consulta resulta vacía o falla."""
    data = journal.get(operation_id)
    if data.get('verified') is True:
        return data
    if data['state'] == 'REVISAR_PREVIAS':
        return journal.check_existing(operation_id, adapter.read(restore_intent(data).identity))
    return journal.reconcile(operation_id, adapter.read(restore_intent(data).identity))


def attach_receipts(work, journal):
    identities = set()
    for row in work.rows:
        try:
            identities.add(RemoteIdentity.from_row(work, row))
        except ValueError:
            continue
    for data in journal.list():
        if restore_intent(data).identity in identities:
            work.receipts['rus:' + data['operation_id']] = data


def export_journal(journal, destination):
    from .reports import _book
    records = journal.list()
    rows = []
    for data in records:
        intent = restore_intent(data)
        rows.append([data['operation_id'], intent.tribunal, intent.identity.rit, intent.identity.ingreso_id,
                     data['state'], data.get('verified') is True, intent.type, intent.send_to_tribunal,
                     intent.text, data.get('remote_text', ''), data.get('registered_at', ''),
                     data.get('remote_entry_id', ''), data.get('excel_path', ''),
                     intent.source_sheet, intent.source_row, intent.source_sha256, data.get('new_registration')])
    return _book([
        ('Resumen', ('Categoría', 'Cantidad'), [
            ['Operaciones preparadas', len(records)],
            ['Registros RUS comprobados', sum(data.get('verified') is True for data in records)],
            ['Gestiones nuevas comprobadas', sum(data.get('verified') is True and data.get('new_registration') is True for data in records)],
            ['Entradas ya existentes comprobadas', sum(data.get('verified') is True and data.get('new_registration') is False for data in records)],
            ['Retornos a Excel pendientes', sum(data['state'] == 'PENDIENTE_EXCEL' for data in records)],
            ['Operaciones sin comprobación', sum(data.get('verified') is not True for data in records)]]),
        ('Operaciones', ('Operación', 'Tribunal', 'RIT', 'Ingreso RUS', 'Estado', 'Guardado RUS comprobado',
                        'Tipo', 'Enviar al tribunal', 'Texto del Excel', 'Texto releído', 'Fecha efectiva RUS',
                        'Entrada RUS', 'Copia Excel', 'Hoja de origen', 'Fila de origen', 'Fuente SHA256', 'Gestión nueva'), rows)
    ], destination)
