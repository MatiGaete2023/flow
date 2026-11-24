"""P6: compara instantáneas locales del lector existente; no consulta RUS."""
from dataclasses import dataclass
from datetime import date, datetime, timezone

from .contracts import RegistroFlowV1, RemoteIdentity

METHOD = 'SITFA_TABLA_INFORME_INGRESO_V1'
TYPES = {'ADMINISTRATIVA': 'Administrativa', 'AL_TRIBUNAL': 'Al Tribunal'}


class ReadbackError(ValueError):
    pass


def _require(condition, code):
    if not condition:
        raise ReadbackError(code)


def _text(value):
    _require(isinstance(value, str) and bool(value.strip()), 'CAMPO_INCOMPLETO')
    return value


def _utc(value):
    try:
        dt = datetime.fromisoformat(value) if isinstance(value, str) else value
        _require(isinstance(dt, datetime) and dt.tzinfo is not None and dt.utcoffset() is not None, 'FECHA_CAPTURA_INVALIDA')
        return dt.astimezone(timezone.utc)
    except (ValueError, TypeError):
        raise ReadbackError('FECHA_CAPTURA_INVALIDA') from None


def _server_date(value):
    try:
        return datetime.fromisoformat(_text(value)).date()
    except (ValueError, TypeError):
        raise ReadbackError('FECHA_SERVIDOR_INVALIDA') from None


@dataclass(frozen=True)
class Entry:
    entry_id: str
    text: str
    author: str
    kind: str
    stage: str
    state: str
    server_timestamp: str


@dataclass(frozen=True)
class Snapshot:
    identity: RemoteIdentity
    captured_at: datetime
    stage_code: str
    entries: tuple[Entry, ...]
    complete: bool

    @classmethod
    def from_reader(cls, data):
        """Adapta el resultado de nurus.bitacora_html.read_snapshot, no el HTML.

        El consumidor debe acreditar captura viva/identidad, no usar archivos reutilizados.
        Alcance refiere a la ventana actual; coverage tradicional PARCIAL no lo sustituye.
        """
        try:
            ident = RemoteIdentity('SITFA', data['tribunal_codigo'], data['causa_id'], data['ingreso_id'])
            captured = _utc(data['captured_at'])
            stage = _text(data['stage_code'])
            rows, metadata = data['entries'], data['row_metadata']
            _require(type(rows) is list and type(metadata) is list, 'FILAS_INVALIDAS')
            states = {}
            for item in metadata:
                key = _text(item['entry_id'])
                _require(key not in states, 'METADATO_DUPLICADO')
                states[key] = _text(item['state'])
            entries = []
            seen = set()
            for row in rows:
                key = _text(row['entry_id'])
                _require(key not in seen and key in states, 'ID_ENTRADA_INVALIDO')
                _require(row['origen'] == 'centro' and (row['tribunal_codigo'], row['causa_id'], row['ingreso_id']) ==
                         (ident.tribunal_id, ident.causa_id, ident.ingreso_id), 'IDENTIDAD_FILA_INCORRECTA')
                _server_date(row['fecha'])
                entries.append(Entry(key, _text(row['texto']), _text(row['autor']), _text(row['tipo']),
                                     _text(row['etapa']), states[key], row['fecha']))
                seen.add(key)
            _require(seen == set(states), 'METADATO_SIN_FILA')
            complete = data.get('alcance_metodo') == METHOD and data.get('alcance') == ('COMPLETA' if entries else 'VACIA_COMPROBADA')
            return cls(ident, captured, stage, tuple(entries), complete)
        except (KeyError, TypeError, AttributeError, ValueError) as exc:
            if isinstance(exc, ReadbackError):
                raise
            raise ReadbackError('INSTANTANEA_INVALIDA') from None


@dataclass(frozen=True)
class ReadbackResult:
    state: str
    reason: str
    entry: Entry | None = None


def _matches(op, row, *, author, stage, server_day):
    return (row.text == op.approved_text and row.author == author and row.kind == TYPES[op.destination] and
            row.stage == stage and row.state == op.observation_state and _server_date(row.server_timestamp) == server_day)


def _context(op, snap, stage_code):
    return (isinstance(op, RegistroFlowV1) and isinstance(snap, Snapshot) and snap.complete is True and
            snap.identity == op.remote_identity and snap.stage_code == stage_code)


def review_previous(op, before, *, author, stage, stage_code, server_day):
    """REUTILIZABLE exige tratamiento separado; no equivale a nueva escritura."""
    if not (_context(op, before, stage_code) and type(server_day) is date and author and stage and stage_code):
        return ReadbackResult('INCIERTA', 'CONTEXTO_O_COBERTURA_INSUFICIENTE')
    matches = [r for r in before.entries if _matches(op, r, author=author, stage=stage, server_day=server_day)]
    if len(matches) > 1:
        return ReadbackResult('REVISAR_PREVIAS', 'VARIAS_PREVIAS')
    if matches:
        return ReadbackResult('REUTILIZABLE', 'PREVIA_DEL_DIA', matches[0])
    return ReadbackResult('SIN_PREVIA', 'LECTURA_COMPLETA_SIN_COINCIDENCIA')


def compare(op, before, after, *, attempt_at, author, stage, stage_code, server_day):
    """Solo resultado de comparación, no recibo persistido ni permiso de escritura."""
    previous = review_previous(op, before, author=author, stage=stage, stage_code=stage_code, server_day=server_day)
    if previous.state != 'SIN_PREVIA':
        return ReadbackResult('INCIERTA', 'PREVIA_O_BASE_NO_ELEGIBLE')
    if not _context(op, after, stage_code):
        return ReadbackResult('INCIERTA', 'CONTEXTO_O_COBERTURA_INSUFICIENTE')
    attempt = _utc(attempt_at)
    if not before.captured_at <= attempt < after.captured_at:
        return ReadbackResult('INCIERTA', 'SECUENCIA_TEMPORAL_INVALIDA')
    old, new = {r.entry_id: r for r in before.entries}, {r.entry_id: r for r in after.entries}
    if len(old) != len(before.entries) or len(new) != len(after.entries) or any(new.get(k) != v for k, v in old.items()):
        return ReadbackResult('INCIERTA', 'BASE_MODIFICADA_O_INCOMPLETA')
    matches = [r for k, r in new.items() if k not in old and _matches(op, r, author=author, stage=stage, server_day=server_day)]
    if not matches:
        return ReadbackResult('NO_HALLADA', 'SIN_NUEVA_COINCIDENCIA')
    if len(matches) > 1:
        return ReadbackResult('AMBIGUA', 'VARIAS_NUEVAS_COINCIDENCIAS')
    return ReadbackResult('COINCIDENCIA_UNICA', 'NUEVO_ID_Y_CAMPOS_EXACTOS', matches[0])
