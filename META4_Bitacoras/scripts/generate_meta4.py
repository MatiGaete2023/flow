"""Clasificación de reportes y escritura de Registros.xlsx y META 4 (base: generador v2.4).

Biblioteca de scripts/meta4_bitacoras.py; no tiene línea de comandos propia."""

from __future__ import annotations

import datetime as dt
import os
import re
import tempfile
import unicodedata
from collections import Counter, defaultdict
from copy import copy
from dataclasses import dataclass, field, replace
from functools import lru_cache
from pathlib import Path
from typing import Any, Callable, Iterable

import pandas as pd
from openpyxl import Workbook, load_workbook
from openpyxl.cell.cell import ILLEGAL_CHARACTERS_RE
from openpyxl.formatting.rule import FormulaRule
from openpyxl.styles import Alignment, Border, Font, PatternFill, Side
from openpyxl.utils import get_column_letter


TRIBUNALS = [
    "ARAUCO",
    "CABRERO",
    "CAÑETE",
    "CONCEPCIÓN",
    "CORONEL",
    "CURANILAHUE",
    "FLORIDA",
    "LOS ANGELES",
    "LAJA",
    "LEBU",
    "MULCHÉN",
    "NACIMIENTO",
    "SANTA BÁRBARA",
    "SANTA JUANA",
    "TALCAHUANO",
    "TOMÉ",
    "YUMBEL",
]

TRIBUNAL_PATTERNS = [
    ("SANTA BARBARA", "SANTA BÁRBARA"),
    ("SANTA JUANA", "SANTA JUANA"),
    ("LOS ANGELES", "LOS ANGELES"),
    ("TALCAHUANO", "TALCAHUANO"),
    ("CONCEPCION", "CONCEPCIÓN"),
    ("CONCEPCIN", "CONCEPCIÓN"),
    ("CURANILAHUE", "CURANILAHUE"),
    ("NACIMIENTO", "NACIMIENTO"),
    ("CABRERO", "CABRERO"),
    ("CANETE", "CAÑETE"),
    ("CAETE", "CAÑETE"),
    ("CORONEL", "CORONEL"),
    ("FLORIDA", "FLORIDA"),
    ("ARAUCO", "ARAUCO"),
    ("MULCHEN", "MULCHÉN"),
    ("LAJA", "LAJA"),
    ("LEBU", "LEBU"),
    ("TOME", "TOMÉ"),
    ("YUMBEL", "YUMBEL"),
]

# Alias aceptados. AMBIGUOUS_ALIASES exige validación de contexto porque la
# abreviatura coincide con una palabra común del español.
TRIBUNAL_ALIASES = {
    "THNO": "TALCAHUANO",
    "CCP": "CONCEPCIÓN",
    "LA": "LOS ANGELES",
    "LOSA": "LOS ANGELES",
    "LGS": "LOS ANGELES",
}

AMBIGUOUS_ALIASES = {"LA"}

TRIBUNAL_REGEXES = [
    (re.compile(rf"(?<![A-Z]){re.escape(pattern)}(?![A-Z])"), label)
    for pattern, label in TRIBUNAL_PATTERNS
]

# Si el alias ambiguo viene precedido por uno de estos términos, es un artículo.
ALIAS_CONNECTORS = {
    "DE", "DEL", "EN", "A", "AL", "CON", "POR", "PARA", "DESDE", "SOBRE",
    "HASTA", "ENTRE", "SEGUN", "Y", "O", "U", "ANTE", "BAJO", "TRAS",
    "TODA", "TODAS", "ESTA", "ESA", "AQUELLA", "SOLO", "SOLAMENTE",
}

# Si el alias ambiguo va seguido por uno de estos términos (o por nada, o por un
# número), es una abreviatura de tribunal.
ALIAS_NEIGHBOURS = {
    "FAE", "FAES", "FAS", "RES", "RESIDENCIA", "RESIDENCIAS", "CREAD",
    "ESPERA", "ESP", "CUMPLIMIENTO", "CUMPLIMEINTO", "CUMPL", "CUMP", "CUM",
    "INFORME", "INFORMES", "JF", "JUZGADO", "FAMILIA",
    "ENERO", "FEBRERO", "MARZO", "ABRIL", "MAYO", "JUNIO", "JULIO",
    "AGOSTO", "SEPTIEMBRE", "SETIEMBRE", "OCTUBRE", "NOVIEMBRE", "DICIEMBRE",
    "XLS", "XLSX", "XLSM", "PDF", "PNG", "JPG", "JPEG",
}

ESPERA_HEADERS = [
    "RIT",
    "TRIBUNAL",
    "RUT",
    "NOMBRE",
    "NACIONALIDAD / PAIS",
    "TIPO",
    "SEXO",
    "DERIVACIÓN",
    "CURADOR",
    "T ESPERA",
    "FEC. RESOLUCIÓN",
    "REVISIÓN",
    "DURACIÓN",
    "80 BIS",
    "FEC. NACIMIENTO",
    "EDAD",
    "FEC. ACT. F. INDIVIDUAL",
    "FEC. ACT. F. AMBULATORIA",
    "FEC. ACT. F. FAE/FAS",
    "FEC. OÍDO",
    "PROXS. AUDS.",
    "FECHA OBS.",
    "ARCHIVO ORIGEN",
    "RUTA ORIGEN",
]

CUMPLIMIENTO_HEADERS = [
    "RIT",
    "TRIBUNAL",
    "RUT",
    "NOMBRE",
    "NACIONALIDAD / PAÍS",
    "TIPO",
    "SEXO",
    "DERIVACIÓN",
    "CURADOR",
    "FEC. RESOLUCIÓN",
    "FEC. INGRESO EFECTIVO",
    "FEC. EGRESO PROYECTADO",
    "DÍAS DE CUMPLIMIENTO",
    "DÍAS PARA EGRESAR",
    "80 BIS",
    "FEC. NACIMIENTO",
    "EDAD",
    "FEC. ACT. F. INDIVIDUAL",
    "FEC. ACT. F. AMBULATORIA",
    "FEC. ACT. F. FAE/FAS",
    "FEC. OÍDO",
    "FECHA OBS.",
    "ARCHIVO ORIGEN",
    "RUTA ORIGEN",
]

SOURCE_HEADERS = {
    "Espera": ESPERA_HEADERS,
    "Cumplimiento": CUMPLIMIENTO_HEADERS,
}

SHEET_DEFS = {
    "Residencia Espera": {
        "source": "ESPERA",
        "category": "Residencia",
        "report": "Informe de Ingresos en Lista de Espera",
        "headers": [
            "RIT",
            "TRIBUNAL",
            "TIPO",
            "SEXO",
            "DERIVACIÓN",
            "T ESPERA",
            "FEC. RESOLUCIÓN",
            "FECHA OBS.",
        ],
    },
    "Residencia Cumplimiento": {
        "source": "CUMPLIMIENTO",
        "category": "Residencia",
        "report": "Informe de Ingresos en Lista de Cumplimiento",
        "headers": [
            "RIT",
            "TRIBUNAL",
            "TIPO",
            "SEXO",
            "DERIVACIÓN",
            "FEC. RESOLUCIÓN",
            "FEC. INGRESO EFECTIVO",
            "FECHA OBS.",
        ],
    },
    "FAE Espera": {
        "source": "ESPERA",
        "category": "FAE",
        "report": "Informe de Ingresos en Lista de Espera",
        "headers": [
            "RIT",
            "TRIBUNAL",
            "TIPO",
            "SEXO",
            "DERIVACIÓN",
            "T ESPERA",
            "FEC. RESOLUCIÓN",
            "FECHA OBS.",
        ],
    },
    "FAE Cumplimiento": {
        "source": "CUMPLIMIENTO",
        "category": "FAE",
        "report": "Informe de Ingresos en Lista de Cumplimiento",
        "headers": [
            "RIT",
            "TRIBUNAL",
            "TIPO",
            "SEXO",
            "DERIVACIÓN",
            "FEC. RESOLUCIÓN",
            "FEC. INGRESO EFECTIVO",
            "FECHA OBS.",
        ],
    },
}

EXCEL_SUFFIXES = {".xls", ".xlsx", ".xlsm"}
PDF_SUFFIXES = {".pdf"}

DATE_HEADERS = {
    "FEC RESOLUCION",
    "FEC INGRESO EFECTIVO",
    "FEC EGRESO PROYECTADO",
    "FEC NACIMIENTO",
    "FEC ACT F INDIVIDUAL",
    "FEC ACT F AMBULATORIA",
    "FEC ACT F FAE FAS",
    "FEC OIDO",
    "FECHA OBS",
}


@dataclass
class Issue:
    level: str
    file: str
    combination: str
    message: str


@dataclass
class SourceRow:
    status: str
    category: str | None
    tribunal: str | None
    values: dict[str, Any]
    source_path: str
    excel_row: int


@dataclass
class Candidate:
    file_result: "FileResult"
    tribunal: str
    category: str
    status: str
    kind: str
    rows: list[SourceRow] = field(default_factory=list)
    emission_date: dt.date | None = None
    modified_at: float = 0.0
    selected: bool = False
    selection_note: str = ""

    @property
    def key(self) -> tuple[str, str, str]:
        return self.tribunal, self.category, self.status


@dataclass
class FileResult:
    path: Path
    relative_path: str
    kind: str
    result: str
    tribunal: str | None = None
    status: str | None = None
    categories: list[str] = field(default_factory=list)
    record_count: int = 0
    emission_date: dt.date | None = None
    sheet_name: str = ""
    header_row: int | None = None
    observations: list[str] = field(default_factory=list)
    candidates: list[Candidate] = field(default_factory=list)
    orphan_rows: list[SourceRow] = field(default_factory=list)

    def selected_label(self) -> str:
        if not self.candidates:
            return "N/A"
        selected = sum(1 for candidate in self.candidates if candidate.selected)
        if selected == len(self.candidates):
            return "SÍ"
        if selected:
            return "PARCIAL"
        return "NO"


@lru_cache(maxsize=1 << 16)
def _norm_text(text: str) -> str:
    text = text.replace("\ufffd", "")
    text = unicodedata.normalize("NFKD", text).encode("ascii", "ignore").decode("ascii")
    return re.sub(r"[^A-Z0-9]+", " ", text.upper()).strip()


def norm(value: Any) -> str:
    if isinstance(value, str):
        return _norm_text(value)
    if value is None or pd.isna(value):
        return ""
    return _norm_text(str(value))


def clean_text(value: Any) -> Any:
    if value is None or pd.isna(value):
        return None
    if isinstance(value, str):
        # Los caracteres de control no caben en un XLSX: openpyxl abortaría toda la escritura.
        return re.sub(r"\s+", " ", ILLEGAL_CHARACTERS_RE.sub(" ", value)).strip()
    return value


def identifier_text(value: Any) -> str | None:
    if value is None or pd.isna(value):
        return None
    if isinstance(value, bool):
        return str(value)
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float) and value.is_integer():
        return str(int(value))
    return str(clean_text(value))


def excel_value(value: Any) -> Any:
    if value is None or pd.isna(value):
        return None
    if isinstance(value, pd.Timestamp):
        return value.to_pydatetime()
    if hasattr(value, "item") and not isinstance(value, (str, bytes, dt.date, dt.datetime)):
        try:
            value = value.item()
        except (TypeError, ValueError):
            pass
    return clean_text(value)


def classify_derivation(value: Any) -> str | None:
    text = norm(value)
    if text.startswith("FAE"):
        return "FAE"
    if text.startswith("R") or text.startswith("CREAD"):
        return "Residencia"
    return None


def segment_tribunals(value: Any) -> tuple[set[str], list[str]]:
    """Devuelve (tribunales hallados, notas) para UN segmento de texto.

    Prioridad: nombre completo > alias. Un alias solo se acepta si el contexto
    descarta que sea una palabra común (ver ALIAS_CONNECTORS/ALIAS_NEIGHBOURS).
    """
    found, notes = _segment_tribunals_text(str(value or ""))
    return set(found), list(notes)


@lru_cache(maxsize=1 << 14)
def _segment_tribunals_text(value: str) -> tuple[frozenset[str], tuple[str, ...]]:
    # Función pura del texto: se memoiza porque la misma celda TRIBUNAL se
    # clasifica una vez por fila y por bitácora.
    raw = re.sub(r"\bL\.\s?A\.?", "LA", value, flags=re.IGNORECASE)
    text = norm(raw)
    if not text:
        return frozenset(), ()
    found: set[str] = set()
    for regex, label in TRIBUNAL_REGEXES:
        if regex.search(text):
            found.add(label)
    # L-01: antes se retornaba aquí apenas un nombre completo coincidía, sin
    # revisar alias. Un segmento con un nombre completo Y un alias de OTRO
    # tribunal (p. ej. "Tome CCP") quedaba resuelto solo al nombre completo,
    # ignorando en silencio la mención del alias. Los alias se acumulan en el
    # mismo conjunto `found`; la ambigüedad (len(found) > 1) la resuelve el
    # llamador exactamente igual que con dos nombres completos.

    notes: list[str] = []
    tokens = text.split()
    for index, token in enumerate(tokens):
        label = TRIBUNAL_ALIASES.get(token)
        if not label:
            continue
        if token not in AMBIGUOUS_ALIASES:
            found.add(label)
            continue
        previous = tokens[index - 1] if index else None
        following = tokens[index + 1] if index + 1 < len(tokens) else None
        if previous in ALIAS_CONNECTORS:
            continue
        if following is None or following in ALIAS_NEIGHBOURS or following.isdigit():
            found.add(label)
        else:
            notes.append(
                f"Se ignoró el alias '{token}' en '{value}' porque el contexto no permite "
                f"distinguirlo de una palabra común; renombra el archivo para usarlo como {label}"
            )
    return frozenset(found), tuple(notes)


def tribunal_name(value: Any) -> str | None:
    """Clasificación para el contenido de una celda (un solo valor)."""
    found, _ = _segment_tribunals_text(str(value or ""))
    if len(found) == 1:
        return next(iter(found))
    return None


def path_segments(value: Any) -> list[str]:
    text = str(value or "").replace("\\", "/")
    return [segment for segment in text.split("/") if segment.strip()]


def tribunal_from_path(value: Any) -> tuple[str | None, list[str]]:
    """Clasificación por ruta: segmento más profundo primero, ambigüedad = None."""
    notes: list[str] = []
    for segment in reversed(path_segments(value)):
        found, segment_notes = segment_tribunals(segment)
        notes.extend(segment_notes)
        if len(found) == 1:
            return next(iter(found)), notes
        if len(found) > 1:
            notes.append(
                f"El segmento '{segment}' menciona más de un tribunal ({', '.join(sorted(found))}); "
                "no se clasificó por ruta"
            )
            return None, notes
    return None, notes


def status_from_title(value: Any) -> str | None:
    text = norm(value)
    if "LISTA DE CUMPLIMIENTO" in text:
        return "Cumplimiento"
    if "LISTA DE ESPERA" in text:
        return "Espera"
    return None


def status_from_headers(headers: Iterable[Any]) -> str | None:
    values = {norm(value) for value in headers if norm(value)}
    if "FEC INGRESO EFECTIVO" in values or "DIAS DE CUMPLIMIENTO" in values:
        return "Cumplimiento"
    if "T ESPERA" in values:
        return "Espera"
    return None


def _status_candidates(segment: str) -> set[str]:
    text = norm(segment)
    tokens = set(text.split())
    candidates: set[str] = set()
    if (
        "CUMPLIMIENTO" in text
        or "CUMPLIMEINTO" in text
        or any(token.startswith("CUMPL") for token in tokens)
        or bool(tokens.intersection({"CUMP", "CUM"}))
    ):
        candidates.add("Cumplimiento")
    if "ESPERA" in text or "ESP" in tokens:
        candidates.add("Espera")
    return candidates


def status_from_path(value: Any) -> str | None:
    """Evalúa primero el nombre del archivo y luego las carpetas hacia arriba,
    igual que tribunal_from_path (M-01). Antes se evaluaba toda la ruta como un
    solo texto: una carpeta padre con la señal contraria (ESPERA/cumpl.pdf)
    anulaba un nombre de archivo inequívoco.

    Un segmento con ambas señales a la vez es ambiguo y detiene la búsqueda en
    ese punto (no se sigue subiendo a una carpeta más lejana a adivinar)."""
    for segment in reversed(path_segments(value)):
        candidates = _status_candidates(segment)
        if len(candidates) == 1:
            return next(iter(candidates))
        if len(candidates) > 1:
            return None
    return None


def _category_candidates(segment: str) -> set[str]:
    tokens = set(norm(segment).split())
    candidates: set[str] = set()
    if tokens.intersection({"FAE", "FAES"}):
        candidates.add("FAE")
    if any(token.startswith("RESIDENCIA") for token in tokens) or tokens.intersection({"RES", "CREAD"}):
        candidates.add("Residencia")
    return candidates


def category_from_path(value: Any) -> str | None:
    """Misma prioridad por segmento que status_from_path (M-01)."""
    for segment in reversed(path_segments(value)):
        candidates = _category_candidates(segment)
        if len(candidates) == 1:
            return next(iter(candidates))
        if len(candidates) > 1:
            return None
    return None


def as_date(value: Any) -> dt.date | Any:
    if value is None or pd.isna(value):
        return None
    if isinstance(value, pd.Timestamp):
        return value.to_pydatetime().date()
    if isinstance(value, dt.datetime):
        return value.date()
    if isinstance(value, dt.date):
        return value
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if value > 20000:
            return dt.date(1899, 12, 30) + dt.timedelta(days=int(value))
        return value
    text = str(value).strip()
    if not text or text == "---":
        return None
    for fmt in ("%d/%m/%Y", "%d-%m-%Y", "%Y-%m-%d %H:%M:%S", "%Y-%m-%d"):
        try:
            return dt.datetime.strptime(text, fmt).date()
        except ValueError:
            pass
    parsed = pd.to_datetime(text, dayfirst=True, errors="coerce")
    if pd.notna(parsed):
        return parsed.to_pydatetime().date()
    return clean_text(value)


def parse_emission_value(value: Any) -> dt.date | None:
    if value is None or pd.isna(value):
        return None
    if isinstance(value, (dt.date, dt.datetime, pd.Timestamp)):
        parsed = as_date(value)
        return parsed if isinstance(parsed, dt.date) else None
    text = str(value)
    match = re.search(r"(\d{1,2}[/-]\d{1,2}[/-]\d{4}|\d{4}-\d{1,2}-\d{1,2})", text)
    if not match:
        return None
    parsed = as_date(match.group(1))
    return parsed if isinstance(parsed, dt.date) else None


def as_wait_days(value: Any) -> Any:
    if value is None or pd.isna(value):
        return None
    text = str(value).strip().replace(",", ".")
    try:
        number = float(text)
        return int(number) if number.is_integer() else number
    except ValueError:
        return clean_text(value)


def expected_keys() -> list[tuple[str, str, str]]:
    return [
        (tribunal, category, status)
        for tribunal in TRIBUNALS
        for category in ("FAE", "Residencia")
        for status in ("Espera", "Cumplimiento")
    ]


def combination_text(key: tuple[str, str, str] | None) -> str:
    return " | ".join(key) if key else ""


def find_header_row(df: pd.DataFrame) -> int | None:
    for row_idx in range(min(len(df.index), 30)):
        values = {norm(value) for value in df.iloc[row_idx] if norm(value)}
        if "RIT" in values and "TRIBUNAL" in values and any(value.startswith("DERIV") for value in values):
            return row_idx
    return None


def header_index(headers: list[Any], wanted: str) -> int | None:
    normalized = [norm(value) for value in headers]
    target = norm(wanted)
    if target in normalized:
        return normalized.index(target)
    if target == "DERIVACION":
        return next((idx for idx, value in enumerate(normalized) if value.startswith("DERIV")), None)
    if target == "FEC RESOLUCION":
        return next((idx for idx, value in enumerate(normalized) if value.startswith("FEC RESOLUC")), None)
    if target == "FECHA OBS":
        return next((idx for idx, value in enumerate(normalized) if value.startswith("FECHA OBS")), None)
    return None


def output_file_reason(path: Path) -> str | None:
    name = norm(path.name)
    if path.name.startswith("~$"):
        return "Archivo temporal de Excel"
    if name == "REGISTROS XLSX":
        return "Salida Registros.xlsx de una ejecución anterior"
    if name.startswith("META 4 ACTUALIZADO"):
        return "Salida META 4 de una ejecución anterior"
    if name.startswith("REPORTE DE PROCESAMIENTO META 4"):
        return "Reporte de una ejecución anterior"
    if name == "META4 TEMPLATE XLSX":
        return "Plantilla interna"
    return None


def relative_string(path: Path, root: Path) -> str:
    try:
        return str(path.relative_to(root))
    except ValueError:
        return str(path)


def read_sheet_frames(readable_path: Path) -> list[tuple[str, pd.DataFrame]]:
    with pd.ExcelFile(readable_path) as xls:
        return [
            (sheet_name, pd.read_excel(xls, sheet_name=sheet_name, header=None, dtype=object))
            for sheet_name in xls.sheet_names
        ]


def inspect_excel(
    path: Path,
    root: Path,
    readable_path: Path | None,
    issues: list[Issue],
    frames: list[tuple[str, pd.DataFrame]] | None = None,
) -> FileResult:
    """Clasifica un Excel. `frames` permite entregar hojas ya leídas en memoria."""
    relative = relative_string(path, root)
    result = FileResult(path=path, relative_path=relative, kind="EXCEL", result="PROCESADO")
    try:
        if frames is None:
            frames = read_sheet_frames(readable_path)
        sheets: list[tuple[tuple[int, int], str, pd.DataFrame, int]] = []
        for sheet_name, df in frames:
            header_row = find_header_row(df)
            if header_row is None:
                continue
            headers = list(df.iloc[header_row])
            rit_idx = header_index(headers, "RIT")
            record_count = 0
            if rit_idx is not None:
                data = df.iloc[header_row + 1 :]
                mask = data.iloc[:, rit_idx].notna() & data.iloc[:, rit_idx].astype(str).str.strip().ne("")
                record_count = int(mask.sum())
            header_score = sum(1 for value in headers if norm(value))
            sheets.append(((record_count, header_score), sheet_name, df, header_row))
    except Exception as exc:
        result.result = "ERROR"
        message = f"No se pudo abrir como Excel: {type(exc).__name__}: {exc}"
        result.observations.append(message)
        issues.append(Issue("ERROR", relative, "", message))
        return result

    if not sheets:
        result.result = "ERROR"
        message = "No se encontró una hoja con encabezados RIT, TRIBUNAL y DERIVACIÓN"
        result.observations.append(message)
        issues.append(Issue("ERROR", relative, "", message))
        return result

    sheets.sort(key=lambda item: item[0], reverse=True)
    _, sheet_name, df, header_row = sheets[0]
    result.sheet_name = sheet_name
    result.header_row = header_row + 1
    if len(sheets) > 1:
        message = f"Se encontraron {len(sheets)} hojas candidatas; se utilizó {sheet_name}"
        result.observations.append(message)
        issues.append(Issue("ADVERTENCIA", relative, "", message))

    headers = list(df.iloc[header_row])
    rit_idx = header_index(headers, "RIT")
    tri_idx = header_index(headers, "TRIBUNAL")
    der_idx = header_index(headers, "DERIVACIÓN")
    if rit_idx is None or tri_idx is None or der_idx is None:
        result.result = "ERROR"
        message = "La hoja seleccionada no contiene las columnas mínimas"
        result.observations.append(message)
        issues.append(Issue("ERROR", relative, "", message))
        return result

    title = df.iat[1, 0] if len(df.index) > 1 else None
    title_status = status_from_title(title)
    column_status = status_from_headers(headers)
    path_status = status_from_path(relative)
    status = title_status or column_status or path_status
    if status is None:
        result.result = "ERROR"
        message = "No se pudo determinar Espera o Cumplimiento"
        result.observations.append(message)
        issues.append(Issue("ERROR", relative, "", message))
        return result
    result.status = status
    if title_status and column_status and title_status != column_status:
        message = f"El título indica {title_status}, pero las columnas indican {column_status}; prevaleció el título"
        result.observations.append(message)
        issues.append(Issue("ADVERTENCIA", relative, "", message))
    elif not title_status:
        message = f"El estado se determinó mediante las columnas: {status}"
        result.observations.append(message)
        issues.append(Issue("ADVERTENCIA", relative, "", message))

    a1 = df.iat[0, 0] if len(df.index) else None
    a1_tribunal = tribunal_name(a1)
    path_tribunal, path_notes = tribunal_from_path(relative)
    for note in path_notes:
        result.observations.append(note)
        issues.append(Issue("ADVERTENCIA", relative, "", note))
    result.emission_date = parse_emission_value(df.iat[2, 0] if len(df.index) > 2 else None)

    data = df.iloc[header_row + 1 :].copy()
    mask = data.iloc[:, rit_idx].notna() & data.iloc[:, rit_idx].astype(str).str.strip().ne("")
    data = data[mask]
    result.record_count = len(data)

    row_tribunals: Counter[str] = Counter()
    category_counts: Counter[str] = Counter()
    rows: list[SourceRow] = []
    canonical_headers = SOURCE_HEADERS[status]
    column_positions = {header: header_index(headers, header) for header in canonical_headers}

    required = ["RIT", "TRIBUNAL", "TIPO", "SEXO", "DERIVACIÓN", "FEC. RESOLUCIÓN"]
    if status == "Espera":
        required.append("T ESPERA")
    else:
        required.append("FEC. INGRESO EFECTIVO")
    missing_columns = [header for header in required if column_positions.get(header) is None]
    if missing_columns:
        message = "Columnas requeridas ausentes: " + ", ".join(missing_columns)
        result.observations.append(message)
        issues.append(Issue("ADVERTENCIA", relative, "", message))

    for df_idx, row in data.iterrows():
        original_tribunal = row.iloc[tri_idx] if tri_idx < len(row) else None
        row_tribunal = tribunal_name(original_tribunal)
        if row_tribunal:
            row_tribunals[row_tribunal] += 1
        original_derivation = row.iloc[der_idx] if der_idx < len(row) else None
        category = classify_derivation(original_derivation)
        if category:
            category_counts[category] += 1

        values: dict[str, Any] = {}
        for header in canonical_headers:
            if header == "ARCHIVO ORIGEN":
                values[header] = path.name
                continue
            if header == "RUTA ORIGEN":
                values[header] = relative
                continue
            col_idx = column_positions.get(header)
            value = row.iloc[col_idx] if col_idx is not None and col_idx < len(row) else None
            if header in {"RIT", "RUT"}:
                value = identifier_text(value)
            elif norm(header) == "T ESPERA":
                value = as_wait_days(value)
            elif norm(header) in DATE_HEADERS:
                value = as_date(value)
            else:
                value = excel_value(value)
            values[header] = value

        rows.append(
            SourceRow(
                status=status,
                category=category,
                tribunal=row_tribunal,
                values=values,
                source_path=relative,
                excel_row=int(df_idx) + 1,
            )
        )

    if row_tribunals:
        most_common, _ = row_tribunals.most_common(1)[0]
        file_tribunal = most_common
        if len(row_tribunals) > 1:
            message = "La columna TRIBUNAL contiene más de un tribunal; se utilizó el más frecuente"
            result.observations.append(message)
            issues.append(Issue("ADVERTENCIA", relative, "", message))
        if a1_tribunal and a1_tribunal != file_tribunal:
            message = f"A1 indica {a1_tribunal}, pero los registros indican {file_tribunal}; prevalecieron los registros"
            result.observations.append(message)
            issues.append(Issue("ADVERTENCIA", relative, "", message))
    else:
        file_tribunal = a1_tribunal or path_tribunal
    result.tribunal = file_tribunal

    if not file_tribunal:
        result.result = "ERROR"
        message = "No se pudo determinar el tribunal desde A1, la columna TRIBUNAL o la ruta"
        result.observations.append(message)
        issues.append(Issue("ERROR", relative, "", message))
        return result

    categories = list(category_counts)
    if not categories:
        fallback_category = category_from_path(relative)
        if fallback_category:
            categories = [fallback_category]
            message = f"La modalidad se determinó por la ruta: {fallback_category}"
            result.observations.append(message)
            issues.append(Issue("ADVERTENCIA", relative, "", message))
        else:
            result.result = "ERROR"
            message = "No se pudo determinar FAE o Residencia"
            result.observations.append(message)
            issues.append(Issue("ERROR", relative, "", message))
            result.orphan_rows.extend(rows)
            return result
    result.categories = sorted(categories, key=lambda value: (value != "FAE", value))
    if len(result.categories) > 1:
        message = "El Excel mezcla FAE y Residencia; se separaron los registros por derivación"
        result.observations.append(message)
        issues.append(Issue("ADVERTENCIA", relative, "", message))

    modified_at = path.stat().st_mtime
    # Filas cuyo TRIBUNAL propio no coincide con el tribunal del archivo generan
    # un candidato aparte, con SU tribunal. La matriz de cobertura se arma desde
    # candidate.tribunal y el META 4 se reconstruye leyendo la columna TRIBUNAL
    # de cada fila (read_registros); si ambos no usan el mismo criterio, pueden
    # contradecirse: la matriz marca un tribunal FALTANTE mientras el META 4 ya
    # tiene filas de ese tribunal (defecto H-03).
    other_tribunals = {row.tribunal for row in rows if row.tribunal and row.tribunal != file_tribunal}
    if other_tribunals:
        message = (
            f"El archivo está asociado a {file_tribunal}, pero contiene filas de "
            f"{', '.join(sorted(other_tribunals))}; se generó un candidato separado por cada "
            "tribunal detectado en la columna TRIBUNAL"
        )
        result.observations.append(message)
        issues.append(Issue("ADVERTENCIA", relative, "", message))

    for category in result.categories:
        rows_by_tribunal: dict[str, list[SourceRow]] = defaultdict(list)
        for row in rows:
            if row.category != category:
                continue
            rows_by_tribunal[row.tribunal or file_tribunal].append(row)
        if not rows_by_tribunal:
            # El Excel no tiene filas de datos para esta categoría (por ejemplo,
            # cero registros y la modalidad se determinó por la ruta). Debe
            # seguir cubriendo la combinación con 0 registros, igual que antes
            # de dividir candidatos por tribunal.
            rows_by_tribunal[file_tribunal] = []
        for candidate_tribunal, candidate_rows in rows_by_tribunal.items():
            candidate = Candidate(
                file_result=result,
                tribunal=candidate_tribunal,
                category=category,
                status=status,
                kind="EXCEL",
                rows=candidate_rows,
                emission_date=result.emission_date,
                modified_at=modified_at,
            )
            result.candidates.append(candidate)

    unknown_rows = [row for row in rows if row.category is None]
    if unknown_rows:
        result.orphan_rows.extend(unknown_rows)
        message = (
            f"{len(unknown_rows)} registro(s) quedan fuera del alcance FAE/Residencia "
            "y no se incorporan a Registros ni a META 4"
        )
        result.observations.append(message)
        issues.append(Issue("ADVERTENCIA", relative, "", message))
    return result


def inspect_zero_record_file(path: Path, root: Path, issues: list[Issue], kind: str = "PDF") -> FileResult:
    """Evidencia sin registros (PDF de una consulta vacía): se clasifica solo por su ruta."""
    relative = relative_string(path, root)
    tribunal, path_notes = tribunal_from_path(relative)
    status = status_from_path(relative)
    category = category_from_path(relative)
    result = FileResult(
        path=path,
        relative_path=relative,
        kind=kind,
        result="SIN REGISTROS",
        tribunal=tribunal,
        status=status,
        categories=[category] if category else [],
        record_count=0,
    )
    for note in path_notes:
        result.observations.append(note)
        issues.append(Issue("ADVERTENCIA", relative, "", note))
    if not tribunal or not status or not category:
        result.result = "NO CLASIFICADO"
        missing = []
        if not tribunal:
            missing.append("tribunal")
        if not category:
            missing.append("modalidad")
        if not status:
            missing.append("estado")
        message = "No se pudo identificar: " + ", ".join(missing)
        result.observations.append(message)
        issues.append(Issue("ADVERTENCIA", relative, "", message))
        return result
    candidate = Candidate(
        file_result=result,
        tribunal=tribunal,
        category=category,
        status=status,
        kind=kind,
        rows=[],
        emission_date=None,
        modified_at=path.stat().st_mtime,
    )
    result.candidates.append(candidate)
    return result


def candidate_sort_key(candidate: Candidate) -> tuple[int, int, float, str]:
    """Un Excel siempre vence a un PDF: su contenido fue leído.

    Entre archivos del mismo rango decide la fecha de emisión y luego la de
    modificación. Sin este primer criterio, un PDF más reciente eliminaba los
    registros de un Excel (defecto F-02).
    """
    effective_date = candidate.emission_date
    if effective_date:
        date_score = effective_date.toordinal()
    else:
        date_score = dt.datetime.fromtimestamp(candidate.modified_at).date().toordinal()
    evidence_score = 1 if candidate.kind == "EXCEL" else 0
    return (
        evidence_score,
        date_score,
        candidate.modified_at,
        norm(candidate.file_result.relative_path),
    )


def _row_identity(row: SourceRow) -> tuple[str, ...]:
    return tuple(norm(row.values.get(header)) for header in ("RIT", "RUT", "NOMBRE", "DERIVACIÓN"))


def _merge_strays(chosen: Candidate, strays: list[Candidate], key: tuple[str, str, str], issues: list[Issue]) -> Candidate:
    seen = {_row_identity(row) for row in chosen.rows}
    extra: list[SourceRow] = []
    for stray in strays:
        added = [row for row in stray.rows if _row_identity(row) not in seen]
        seen.update(_row_identity(row) for row in added)
        extra.extend(added)
        stray.selected = True
        stray.selection_note = f"Filas incorporadas a {combination_text(key)}"
        issues.append(
            Issue(
                "ADVERTENCIA",
                stray.file_result.relative_path,
                combination_text(key),
                f"El archivo de {stray.file_result.tribunal} contiene {len(stray.rows)} fila(s) de {key[0]}: "
                f"se incorporaron {len(added)}; {len(stray.rows) - len(added)} ya estaban en el reporte propio",
            )
        )
    return replace(chosen, rows=chosen.rows + extra)


def select_candidates(
    file_results: list[FileResult],
    issues: list[Issue],
) -> tuple[dict[tuple[str, str, str], Candidate], dict[tuple[str, str, str], list[Candidate]]]:
    groups: dict[tuple[str, str, str], list[Candidate]] = defaultdict(list)
    for result in file_results:
        for candidate in result.candidates:
            groups[candidate.key].append(candidate)

    selected: dict[tuple[str, str, str], Candidate] = {}
    for key in expected_keys():
        candidates = groups.get(key, [])
        if not candidates:
            issues.append(Issue("FALTANTE", "", combination_text(key), "No se encontró un archivo para esta combinación"))
            continue
        # Las filas de este tribunal halladas en el archivo de OTRO tribunal no son una versión
        # del reporte: no compiten con el archivo propio, se suman a él sin repetir filas.
        primaries = [c for c in candidates if c.tribunal == c.file_result.tribunal]
        chosen = max(primaries or candidates, key=candidate_sort_key)
        strays = [c for c in candidates if c is not chosen and c.tribunal != c.file_result.tribunal]
        candidates = primaries or [chosen]
        groups[key] = list(candidates)  # la matriz cuenta como duplicado solo otra versión del reporte
        chosen.selected = True
        chosen.selection_note = "Seleccionado"
        selected[key] = _merge_strays(chosen, strays, key, issues) if strays else chosen
        if len(candidates) > 1:
            omitted = [candidate for candidate in candidates if candidate is not chosen]
            for candidate in omitted:
                candidate.selection_note = f"Duplicado omitido; se utilizó {chosen.file_result.relative_path}"
                candidate.file_result.observations.append(candidate.selection_note)
            chosen.file_result.observations.append(f"Seleccionado entre {len(candidates)} archivos duplicados")
            omitted_names = "; ".join(candidate.file_result.relative_path for candidate in omitted)
            message = (
                f"Se utilizó {chosen.file_result.relative_path}. Omitidos: {omitted_names}. "
                "Criterio: Excel sobre PDF/imagen, luego fecha de emisión y fecha de modificación"
            )
            issues.append(Issue("DUPLICADO", chosen.file_result.relative_path, combination_text(key), message))

            lost_rows = sum(
                len(candidate.rows) for candidate in omitted if candidate.kind == "EXCEL"
            )
            if not chosen.rows and lost_rows:
                issues.append(
                    Issue(
                        "ERROR",
                        chosen.file_result.relative_path,
                        combination_text(key),
                        f"El archivo seleccionado no aporta registros, pero se omitieron "
                        f"{lost_rows} registro(s) de un Excel. Revisa fechas y nombres antes de usar el META 4",
                    )
                )
            newer_omitted = [
                candidate
                for candidate in omitted
                if candidate.kind != "EXCEL" and candidate.modified_at > chosen.modified_at
            ]
            if chosen.kind == "EXCEL" and newer_omitted:
                newer_names = "; ".join(candidate.file_result.relative_path for candidate in newer_omitted)
                issues.append(
                    Issue(
                        "REQUIERE_REVISION",
                        chosen.file_result.relative_path,
                        combination_text(key),
                        f"Se usó el Excel {chosen.file_result.relative_path} para no perder sus "
                        f"{len(chosen.rows)} registro(s), pero existe un archivo más reciente sin "
                        f"registros para la misma combinación: {newer_names}. Confirma que el Excel "
                        "corresponde al período informado antes de dar el META 4 por definitivo",
                    )
                )
    return selected, groups


def consolidated_rows(
    selected: dict[tuple[str, str, str], Candidate],
) -> dict[str, list[SourceRow]]:
    rows: dict[str, list[SourceRow]] = {"Espera": [], "Cumplimiento": []}
    tribunal_order = {tribunal: index for index, tribunal in enumerate(TRIBUNALS)}
    ordered_candidates = sorted(
        selected.values(),
        key=lambda candidate: (
            tribunal_order.get(candidate.tribunal, 999),
            candidate.category != "FAE",
            candidate.status != "Espera",
            norm(candidate.file_result.relative_path),
        ),
    )
    for candidate in ordered_candidates:
        if candidate.kind != "EXCEL":
            continue
        # Alcance de la herramienta: solo FAE y Residencia. Las filas con otra
        # derivación (OPD, DAM, ambulatorios, etc.) quedan fuera de Registros y
        # solo se informan como advertencia con archivo y conteo.
        rows[candidate.status].extend(candidate.rows)
    return rows


NAVY = "17365D"
LIGHT_BLUE = "D9EAF7"
PALE_BLUE = "EAF3F8"
GREEN = "E2F0D9"
ORANGE = "FCE4D6"
RED = "F4CCCC"
GRAY = "E7E6E6"
WHITE = "FFFFFF"
DARK_TEXT = "1F2937"
LIGHT_BORDER = "CBD5E1"

NUMBER_HEADERS = {"T ESPERA", "DURACION", "EDAD", "DIAS DE CUMPLIMIENTO", "DIAS PARA EGRESAR"}


def guard_text(cell) -> None:
    """Un texto de origen que empieza con '=' se conserva como texto, no como fórmula.

    Sin esto openpyxl lo guarda como fórmula sin valor calculado: pandas y la
    verificación lo releen vacío y el dato se pierde."""
    if cell.data_type == "f":
        cell.data_type = "s"


class StyleMemo:
    """Reutiliza el estilo ya registrado en un libro.

    Asignar fuente, relleno y borde a cada celda obliga a openpyxl a copiar y
    volver a indexar esos objetos; la combinación ya registrada se copia como
    índice. Válido solo dentro del mismo libro."""

    def __init__(self) -> None:
        self._arrays: dict[Any, Any] = {}

    def apply(self, cell, key: Any, setter: Callable[[Any], None]) -> None:
        array = self._arrays.get(key)
        if array is None:
            setter(cell)
            self._arrays[key] = copy(cell._style)
        else:
            cell._style = copy(array)


def save_workbook_atomic(workbook: Workbook, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(
            prefix=f"{path.stem}_",
            suffix=path.suffix,
            dir=path.parent,
            delete=False,
        ) as handle:
            temporary = Path(handle.name)
        workbook.save(temporary)
        os.replace(temporary, path)
        temporary = None
    finally:
        if temporary and temporary.exists():
            temporary.unlink()


def apply_header_style(ws, row: int, start_col: int, end_col: int) -> None:
    thin = Side(style="thin", color=LIGHT_BORDER)
    for col in range(start_col, end_col + 1):
        cell = ws.cell(row, col)
        cell.font = Font(name="Aptos", size=10, bold=True, color=WHITE)
        cell.fill = PatternFill("solid", fgColor=NAVY)
        cell.alignment = Alignment(horizontal="center", vertical="center", wrap_text=True)
        cell.border = Border(bottom=thin)
    ws.row_dimensions[row].height = 30


def style_data_sheet(
    ws,
    headers: list[str],
    rows: list[SourceRow],
    row_fill: Callable[[SourceRow], str | None] | None = None,
    extra_wide: int = 0,
) -> None:
    """`row_fill` puede devolver un color que reemplaza el cebreado de la fila.
    Las últimas `extra_wide` columnas (controles agregados) se muestran anchas."""
    ws.sheet_view.showGridLines = False
    ws.freeze_panes = "A2"
    ws.auto_filter.ref = f"A1:{get_column_letter(len(headers))}{max(1, len(rows) + 1)}"
    for col_idx, header in enumerate(headers, 1):
        ws.cell(1, col_idx, header)
    apply_header_style(ws, 1, 1, len(headers))

    formats: list[tuple[str | None, bool]] = []
    for header in headers:
        normalized = norm(header)
        if normalized in DATE_HEADERS:
            formats.append(("DD-MM-YYYY", True))
        elif normalized in NUMBER_HEADERS:
            formats.append(("0", False))
        elif normalized in {"RIT", "RUT"}:
            formats.append(("@", False))
        else:
            formats.append((None, False))
    font = Font(name="Aptos", size=9, color=DARK_TEXT)
    alignment = Alignment(vertical="top", wrap_text=False)
    memo = StyleMemo()

    def styler(fill: str | None, number_format: str | None) -> Callable[[Any], None]:
        def setter(cell) -> None:
            cell.font = font
            cell.alignment = alignment
            if fill:
                cell.fill = PatternFill("solid", fgColor=fill)
            if number_format:
                cell.number_format = number_format
        return setter

    for row_idx, source_row in enumerate(rows, 2):
        fill = row_fill(source_row) if row_fill else None
        if fill is None and row_idx % 2 == 0:
            fill = "F7FAFC"
        for col_idx, header in enumerate(headers, 1):
            cell = ws.cell(row_idx, col_idx, source_row.values.get(header))
            guard_text(cell)
            number_format, only_with_value = formats[col_idx - 1]
            if only_with_value and cell.value is None:
                number_format = None
            memo.apply(cell, (fill, number_format), styler(fill, number_format))

    width_by_header = {
        "RIT": 15,
        "TRIBUNAL": 34,
        "RUT": 15,
        "NOMBRE": 30,
        "NACIONALIDAD PAIS": 20,
        "TIPO": 14,
        "SEXO": 9,
        "DERIVACION": 42,
        "CURADOR": 30,
        "T ESPERA": 12,
        "REVISION": 14,
        "DURACION": 12,
        "80 BIS": 10,
        "EDAD": 9,
        "PROXS AUDS": 22,
        "ARCHIVO ORIGEN": 38,
        "RUTA ORIGEN": 60,
    }
    wide_columns = set(range(len(headers) - extra_wide + 1, len(headers) + 1)) if extra_wide else set()
    for col_idx, header in enumerate(headers, 1):
        normalized = norm(header)
        if col_idx in wide_columns:
            width = 42
        elif normalized in width_by_header:
            width = width_by_header[normalized]
        elif normalized in DATE_HEADERS or normalized.startswith("FEC ") or normalized.startswith("FECHA "):
            width = 18
        elif normalized.startswith("DIAS "):
            width = 16
        else:
            width = 16
        ws.column_dimensions[get_column_letter(col_idx)].width = width
    ws.sheet_properties.pageSetUpPr.fitToPage = True
    ws.page_setup.orientation = "landscape"
    ws.page_setup.fitToWidth = 1
    ws.page_setup.fitToHeight = 0


def style_simple_table(
    ws,
    header_row: int,
    first_col: int,
    last_col: int,
    last_row: int,
    freeze: str,
) -> None:
    ws.sheet_view.showGridLines = False
    ws.freeze_panes = freeze
    apply_header_style(ws, header_row, first_col, last_col)
    if last_row >= header_row:
        ws.auto_filter.ref = (
            f"{get_column_letter(first_col)}{header_row}:"
            f"{get_column_letter(last_col)}{max(header_row, last_row)}"
        )
    for row in range(header_row + 1, last_row + 1):
        for col in range(first_col, last_col + 1):
            cell = ws.cell(row, col)
            cell.font = Font(name="Aptos", size=9, color=DARK_TEXT)
            cell.alignment = Alignment(vertical="top", wrap_text=True)
            if row % 2 == 0:
                cell.fill = PatternFill("solid", fgColor="F7FAFC")


def write_inventory_sheet(ws, file_results: list[FileResult]) -> None:
    headers = [
        "N°",
        "ARCHIVO",
        "RUTA RELATIVA",
        "TIPO",
        "TRIBUNAL",
        "MODALIDAD",
        "ESTADO",
        "REGISTROS",
        "FECHA EMISIÓN",
        "RESULTADO",
        "SELECCIONADO",
        "OBSERVACIÓN",
    ]
    for col_idx, header in enumerate(headers, 1):
        ws.cell(1, col_idx, header)
    for row_idx, result in enumerate(file_results, 2):
        values = [
            row_idx - 1,
            result.path.name,
            result.relative_path,
            result.kind,
            result.tribunal,
            ", ".join(result.categories),
            result.status,
            result.record_count,
            result.emission_date,
            result.result,
            result.selected_label(),
            " | ".join(dict.fromkeys(result.observations)),
        ]
        for col_idx, value in enumerate(values, 1):
            guard_text(ws.cell(row_idx, col_idx, value))
        if result.emission_date:
            ws.cell(row_idx, 9).number_format = "DD-MM-YYYY"
        result_cell = ws.cell(row_idx, 10)
        if result.result in {"ERROR", "NO CLASIFICADO"}:
            result_cell.fill = PatternFill("solid", fgColor=RED)
        elif result.result == "SIN REGISTROS":
            result_cell.fill = PatternFill("solid", fgColor=PALE_BLUE)
        elif result.result == "IGNORADO":
            result_cell.fill = PatternFill("solid", fgColor=GRAY)
        else:
            result_cell.fill = PatternFill("solid", fgColor=GREEN)
    style_simple_table(ws, 1, 1, len(headers), len(file_results) + 1, "A2")
    widths = [7, 42, 65, 12, 23, 16, 16, 12, 16, 18, 14, 70]
    for index, width in enumerate(widths, 1):
        ws.column_dimensions[get_column_letter(index)].width = width


def matrix_cell_text(
    key: tuple[str, str, str],
    selected: dict[tuple[str, str, str], Candidate],
    groups: dict[tuple[str, str, str], list[Candidate]],
) -> str:
    candidate = selected.get(key)
    if not candidate:
        return "FALTANTE"
    if candidate.kind == "EXCEL":
        text = f"EXCEL · {len(candidate.rows)} registro(s)"
        out_of_scope = len(candidate.file_result.orphan_rows)
        if out_of_scope:
            text += f" (+{out_of_scope} fuera de alcance)"
    else:
        text = "PDF · 0 registros"
    siblings = groups.get(key, [])
    if len(siblings) > 1:
        text += " · DUPLICADO"
    if candidate.kind == "EXCEL" and any(
        sibling.kind != "EXCEL" and sibling.modified_at > candidate.modified_at for sibling in siblings
    ):
        text += " · REQUIERE REVISIÓN"
    return text


def write_matrix_sheet(
    ws,
    selected: dict[tuple[str, str, str], Candidate],
    groups: dict[tuple[str, str, str], list[Candidate]],
    scope: Iterable[str] | None = None,
) -> None:
    """`scope` limita la cobertura esperada a esos tribunales; el resto se marca NO INCLUIDO."""
    included = set(TRIBUNALS if scope is None else scope)
    ws.sheet_view.showGridLines = False
    ws.merge_cells("A1:E1")
    ws["A1"] = "MATRIZ DE COBERTURA DE ARCHIVOS"
    ws["A1"].font = Font(name="Aptos Display", size=16, bold=True, color=WHITE)
    ws["A1"].fill = PatternFill("solid", fgColor=NAVY)
    ws["A1"].alignment = Alignment(horizontal="left", vertical="center")
    ws.row_dimensions[1].height = 30

    expected = [key for key in expected_keys() if key[0] in included]
    found = sum(1 for key in expected if key in selected)
    duplicates = sum(1 for key in expected if len(groups.get(key, [])) > 1)
    ws.merge_cells("A2:E2")
    ws["A2"] = (
        f"Cobertura: {found}/{len(expected)} · Faltantes: {len(expected) - found} · "
        f"Combinaciones duplicadas: {duplicates}"
    )
    if scope is not None:
        ws["A2"] = ws["A2"].value + f" · Tribunales incluidos: {len(included)}/{len(TRIBUNALS)}"
    ws["A2"].font = Font(name="Aptos", size=10, bold=True, color=DARK_TEXT)
    ws["A2"].fill = PatternFill("solid", fgColor=LIGHT_BLUE)
    ws["A2"].alignment = Alignment(vertical="center")

    headers = ["TRIBUNAL", "FAE ESPERA", "FAE CUMPLIMIENTO", "RESIDENCIA ESPERA", "RESIDENCIA CUMPLIMIENTO"]
    for col_idx, header in enumerate(headers, 1):
        ws.cell(4, col_idx, header)
    apply_header_style(ws, 4, 1, 5)

    for row_idx, tribunal in enumerate(TRIBUNALS, 5):
        ws.cell(row_idx, 1, tribunal)
        keys = [
            (tribunal, "FAE", "Espera"),
            (tribunal, "FAE", "Cumplimiento"),
            (tribunal, "Residencia", "Espera"),
            (tribunal, "Residencia", "Cumplimiento"),
        ]
        for col_idx, key in enumerate(keys, 2):
            text = matrix_cell_text(key, selected, groups) if tribunal in included else "NO INCLUIDO"
            ws.cell(row_idx, col_idx, text)
        for col_idx in range(1, 6):
            cell = ws.cell(row_idx, col_idx)
            cell.font = Font(name="Aptos", size=9, bold=(col_idx == 1), color=DARK_TEXT)
            cell.alignment = Alignment(vertical="center", wrap_text=True)
            if row_idx % 2 == 0:
                cell.fill = PatternFill("solid", fgColor="F7FAFC")
        ws.row_dimensions[row_idx].height = 30

    data_range = f"B5:E{4 + len(TRIBUNALS)}"
    ws.conditional_formatting.add(
        data_range,
        FormulaRule(formula=["ISNUMBER(SEARCH(\"FALTANTE\",B5))"], fill=PatternFill("solid", fgColor=RED)),
    )
    ws.conditional_formatting.add(
        data_range,
        FormulaRule(formula=["ISNUMBER(SEARCH(\"DUPLICADO\",B5))"], fill=PatternFill("solid", fgColor=ORANGE)),
    )
    ws.conditional_formatting.add(
        data_range,
        FormulaRule(formula=["ISNUMBER(SEARCH(\"EXCEL\",B5))"], fill=PatternFill("solid", fgColor=GREEN)),
    )
    ws.conditional_formatting.add(
        data_range,
        FormulaRule(formula=["ISNUMBER(SEARCH(\"PDF\",B5))"], fill=PatternFill("solid", fgColor=PALE_BLUE)),
    )
    ws.freeze_panes = "B5"
    ws.column_dimensions["A"].width = 24
    for column in "BCDE":
        ws.column_dimensions[column].width = 29


def write_issues_sheet(ws, issues: list[Issue]) -> None:
    ws.sheet_view.showGridLines = False
    ws.merge_cells("A1:D1")
    ws["A1"] = "ERRORES Y ADVERTENCIAS"
    ws["A1"].font = Font(name="Aptos Display", size=16, bold=True, color=WHITE)
    ws["A1"].fill = PatternFill("solid", fgColor=NAVY)
    ws["A1"].alignment = Alignment(horizontal="left", vertical="center")
    ws.row_dimensions[1].height = 30
    headers = ["NIVEL", "ARCHIVO", "COMBINACIÓN", "MENSAJE"]
    for col_idx, header in enumerate(headers, 1):
        ws.cell(3, col_idx, header)

    display_issues = issues or [Issue("INFO", "", "", "No se detectaron errores ni advertencias")]
    for row_idx, issue in enumerate(display_issues, 4):
        values = [issue.level, issue.file, issue.combination, issue.message]
        for col_idx, value in enumerate(values, 1):
            guard_text(ws.cell(row_idx, col_idx, value))
        level_cell = ws.cell(row_idx, 1)
        if issue.level in {"ERROR", "FALTANTE"}:
            level_cell.fill = PatternFill("solid", fgColor=RED)
        elif issue.level in {"ADVERTENCIA", "DUPLICADO", "REQUIERE_REVISION"}:
            level_cell.fill = PatternFill("solid", fgColor=ORANGE)
        else:
            level_cell.fill = PatternFill("solid", fgColor=GREEN)
    style_simple_table(ws, 3, 1, 4, len(display_issues) + 3, "A4")
    widths = [16, 55, 38, 90]
    for index, width in enumerate(widths, 1):
        ws.column_dimensions[get_column_letter(index)].width = width


def write_registros_workbook(
    path: Path,
    rows: dict[str, list[SourceRow]],
    file_results: list[FileResult],
    selected: dict[tuple[str, str, str], Candidate],
    groups: dict[tuple[str, str, str], list[Candidate]],
    issues: list[Issue],
    extra_headers: Iterable[str] = (),
    row_fill: Callable[[SourceRow], str | None] | None = None,
    scope: Iterable[str] | None = None,
) -> None:
    """`extra_headers` agrega columnas de control al final de ESPERA y CUMPLIMIENTO.
    `scope` limita la matriz de cobertura a esos tribunales."""
    extra = list(extra_headers)
    wb = Workbook()
    wb.remove(wb.active)
    espera = wb.create_sheet("ESPERA")
    style_data_sheet(espera, ESPERA_HEADERS + extra, rows["Espera"], row_fill, len(extra))
    cumplimiento = wb.create_sheet("CUMPLIMIENTO")
    style_data_sheet(cumplimiento, CUMPLIMIENTO_HEADERS + extra, rows["Cumplimiento"], row_fill, len(extra))
    inventory = wb.create_sheet("INVENTARIO_ARCHIVOS")
    write_inventory_sheet(inventory, file_results)
    matrix = wb.create_sheet("MATRIZ_COBERTURA")
    write_matrix_sheet(matrix, selected, groups, scope)
    issue_sheet = wb.create_sheet("ERRORES_Y_ADVERTENCIAS")
    write_issues_sheet(issue_sheet, issues)
    wb.active = 0
    save_workbook_atomic(wb, path)


def find_sheet(workbook_sheets: Iterable[str], target: str) -> str:
    wanted = norm(target)
    for sheet in workbook_sheets:
        if norm(sheet) == wanted:
            return sheet
    raise KeyError(f"Falta la hoja: {target}")


def find_col(columns: Iterable[Any], kind: str, required: bool = True) -> Any:
    normalized = [(col, norm(col)) for col in columns]
    if kind == "rit":
        matches = [col for col, value in normalized if value == "RIT"]
    elif kind == "tribunal":
        matches = [col for col, value in normalized if value == "TRIBUNAL"]
    elif kind == "tipo":
        matches = [col for col, value in normalized if value == "TIPO"]
    elif kind == "sexo":
        matches = [col for col, value in normalized if value == "SEXO"]
    elif kind == "derivacion":
        matches = [col for col, value in normalized if value.startswith("DERIV")]
    elif kind == "t_espera":
        matches = [col for col, value in normalized if value == "T ESPERA"]
    elif kind == "fec_resolucion":
        matches = [col for col, value in normalized if value.startswith("FEC RESOLUC")]
    elif kind == "fec_ingreso":
        matches = [col for col, value in normalized if value.startswith("FEC INGRESO EFECTIVO")]
    elif kind == "fecha_obs":
        matches = [col for col, value in normalized if value.startswith("FECHA OBS")]
    else:
        matches = []
    if not matches:
        if required:
            raise KeyError(f"Falta una columna para {kind}")
        return None
    return matches[0]


def read_registros(
    path: Path,
) -> tuple[
    dict[str, dict[str, list[list[Any]]]],
    dict[str, int],
    list[dict[str, Any]],
]:
    records = {name: {tribunal: [] for tribunal in TRIBUNALS} for name in SHEET_DEFS}
    source_counts: dict[str, int] = {}
    unmatched: list[dict[str, Any]] = []

    with pd.ExcelFile(path) as xls:
        source_sheet_names = {
            "ESPERA": find_sheet(xls.sheet_names, "ESPERA"),
            "CUMPLIMIENTO": find_sheet(xls.sheet_names, "CUMPLIMIENTO"),
        }
        for source, actual_sheet in source_sheet_names.items():
            df = pd.read_excel(xls, sheet_name=actual_sheet, dtype=object)
            cols = df.columns
            c_rit = find_col(cols, "rit")
            c_tri = find_col(cols, "tribunal")
            c_tipo = find_col(cols, "tipo")
            c_sexo = find_col(cols, "sexo")
            c_der = find_col(cols, "derivacion")
            c_res = find_col(cols, "fec_resolucion")
            c_obs = find_col(cols, "fecha_obs", required=False)
            c_wait = find_col(cols, "t_espera") if source == "ESPERA" else None
            c_ing = find_col(cols, "fec_ingreso") if source == "CUMPLIMIENTO" else None

            real = df[df[c_rit].notna() & df[c_rit].astype(str).str.strip().ne("")].copy()
            source_counts[source] = len(real)
            for idx, row in real.iterrows():
                category = classify_derivation(row[c_der])
                tribunal = tribunal_name(row[c_tri])
                if category is None or tribunal is None:
                    unmatched.append(
                        {
                            "sheet": source,
                            "row": int(idx) + 2,
                            "tribunal": clean_text(row[c_tri]),
                            "derivacion": clean_text(row[c_der]),
                        }
                    )
                    continue

                sheet_name = f"{category} Espera" if source == "ESPERA" else f"{category} Cumplimiento"
                observation = row[c_obs] if c_obs is not None else None
                if source == "ESPERA":
                    rec = [
                        clean_text(row[c_rit]),
                        clean_text(row[c_tri]),
                        clean_text(row[c_tipo]),
                        clean_text(row[c_sexo]),
                        clean_text(row[c_der]),
                        as_wait_days(row[c_wait]),
                        as_date(row[c_res]),
                        as_date(observation),
                    ]
                else:
                    rec = [
                        clean_text(row[c_rit]),
                        clean_text(row[c_tri]),
                        clean_text(row[c_tipo]),
                        clean_text(row[c_sexo]),
                        clean_text(row[c_der]),
                        as_date(row[c_res]),
                        as_date(row[c_ing]),
                        as_date(observation),
                    ]
                records[sheet_name][tribunal].append(rec)
    return records, source_counts, unmatched


def build_default_template() -> Workbook:
    wb = Workbook()
    wb.remove(wb.active)
    widths = [13, 34, 15, 8, 44, 18, 22, 16]
    thin = Side(style="thin", color="000000")
    border = Border(top=thin, left=thin, right=thin, bottom=thin)
    header_fill = PatternFill("solid", fgColor=LIGHT_BLUE)

    for sheet_name, spec in SHEET_DEFS.items():
        ws = wb.create_sheet(sheet_name)
        ws.sheet_view.showGridLines = False
        for idx, width in enumerate(widths, 1):
            ws.column_dimensions[get_column_letter(idx)].width = width
        row = 1
        ws.cell(row, 1, "ARAUCO").font = Font(bold=True)
        row += 1
        ws.cell(row, 1, spec["report"])
        row += 1
        ws.cell(row, 1, "Fecha Emisión   : 01-01-2000")
        row += 2
        for col, value in enumerate(spec["headers"], 1):
            cell = ws.cell(row, col, value)
            cell.font = Font(bold=True)
            cell.fill = header_fill
            cell.border = border
            cell.alignment = Alignment(horizontal="center", wrap_text=True)
        row += 1
        if "Cumplimiento" in sheet_name:
            dummy = [
                "X-0-0000",
                "Tribunal",
                "Tipo",
                "S",
                "DERIVACIÓN",
                dt.date(2000, 1, 1),
                dt.date(2000, 1, 1),
                dt.date(2000, 1, 1),
            ]
        else:
            dummy = [
                "X-0-0000",
                "Tribunal",
                "Tipo",
                "S",
                "DERIVACIÓN",
                0,
                dt.date(2000, 1, 1),
                dt.date(2000, 1, 1),
            ]
        for col, value in enumerate(dummy, 1):
            cell = ws.cell(row, col, value)
            cell.border = border
            if "Cumplimiento" in sheet_name and col in (6, 7, 8):
                cell.number_format = "DD-MM-YYYY"
            elif "Espera" in sheet_name and col in (7, 8):
                cell.number_format = "DD-MM-YYYY"
            elif "Espera" in sheet_name and col == 6:
                cell.number_format = "0"
        row += 2
        ws.cell(row, 1, "SIN REGISTROS").font = Font(bold=True)
        row += 2
        ws.cell(row, 1, "TOTAL:").font = Font(bold=True)
        ws.cell(row, 2, 0).font = Font(bold=True)
    return wb


def load_template(path: Path | None) -> Workbook:
    if path and path.exists():
        wb = load_workbook(path)
    else:
        wb = build_default_template()
    for ws in list(wb.worksheets):
        clean = ws.title.strip()
        if clean != ws.title:
            ws.title = clean
    return wb


def row_styles(ws, row_num: int | None, width: int = 8) -> list[dict[str, Any]]:
    if row_num is None:
        return [{} for _ in range(width)]
    styles = []
    for col in range(1, width + 1):
        cell = ws.cell(row_num, col)
        styles.append(
            {
                "font": copy(cell.font),
                "fill": copy(cell.fill),
                "border": copy(cell.border),
                "alignment": copy(cell.alignment),
                "number_format": cell.number_format,
                "protection": copy(cell.protection),
            }
        )
    return styles


def find_row(ws, predicate) -> int | None:
    for row in range(1, ws.max_row + 1):
        if predicate(row):
            return row
    return None


def capture_templates(ws) -> dict[str, Any]:
    title = find_row(
        ws,
        lambda row: isinstance(ws.cell(row, 1).value, str)
        and isinstance(ws.cell(row + 1, 1).value, str)
        and ws.cell(row + 1, 1).value.startswith("Informe de Ingresos"),
    )
    header = find_row(ws, lambda row: ws.cell(row, 1).value == "RIT")
    data = header + 1 if header else None
    sin = find_row(ws, lambda row: ws.cell(row, 1).value == "SIN REGISTROS")
    total = find_row(
        ws,
        lambda row: isinstance(ws.cell(row, 1).value, str)
        and ws.cell(row, 1).value.upper().startswith("TOTAL"),
    )
    if not title or not header or not data or not sin or not total:
        raise RuntimeError(f"La hoja de plantilla {ws.title} no contiene las filas de estilo requeridas")
    return {
        "title": row_styles(ws, title),
        "report": row_styles(ws, title + 1),
        "date": row_styles(ws, title + 2),
        "header": row_styles(ws, header),
        "data": row_styles(ws, data),
        "sin": row_styles(ws, sin),
        "total": row_styles(ws, total),
        "heights": {
            "title": ws.row_dimensions[title].height,
            "report": ws.row_dimensions[title + 1].height,
            "date": ws.row_dimensions[title + 2].height,
            "header": ws.row_dimensions[header].height,
            "data": ws.row_dimensions[data].height,
            "sin": ws.row_dimensions[sin].height,
            "total": ws.row_dimensions[total].height,
        },
    }


def apply_style(cell, style: dict[str, Any]) -> None:
    if not style:
        return
    cell.font = copy(style["font"])
    cell.fill = copy(style["fill"])
    cell.border = copy(style["border"])
    cell.alignment = copy(style["alignment"])
    cell.number_format = style["number_format"]
    cell.protection = copy(style["protection"])


def write_row(
    ws,
    row_num: int,
    values: list[Any],
    styles: list[dict[str, Any]],
    date_cols: tuple[int, ...] = (),
    memo: StyleMemo | None = None,
) -> None:
    for col in range(1, 9):
        cell = ws.cell(row_num, col)
        cell.value = values[col - 1] if col <= len(values) else None
        guard_text(cell)
        style = styles[col - 1]
        as_date = col in date_cols and cell.value is not None

        def setter(target, style=style, as_date=as_date) -> None:
            apply_style(target, style)
            if as_date:
                target.number_format = "DD-MM-YYYY"

        if memo is None:
            setter(cell)
        else:
            memo.apply(cell, (id(style), as_date), setter)


def set_height(ws, row_num: int, templates: dict[str, Any], key: str) -> None:
    height = templates["heights"].get(key)
    if height is not None:
        ws.row_dimensions[row_num].height = height


def rebuild(
    wb: Workbook,
    records: dict[str, dict[str, list[list[Any]]]],
    emission_date: dt.date,
    tribunals: Iterable[str] | None = None,
) -> dict[str, int]:
    """`tribunals` limita los bloques escritos (por defecto, todos en el orden de TRIBUNALS)."""
    blocks = TRIBUNALS if tribunals is None else [t for t in TRIBUNALS if t in set(tribunals)]
    emission_text = emission_date.strftime("%d-%m-%Y")
    summary: dict[str, int] = {}
    # Los estilos capturados viven hasta el final; sus id() son claves estables.
    memo = StyleMemo()
    captured: list[dict[str, Any]] = []
    for sheet_name, spec in SHEET_DEFS.items():
        if sheet_name not in wb.sheetnames:
            ws = wb.create_sheet(sheet_name)
            temp = build_default_template()[sheet_name]
            for col in range(1, 9):
                letter = get_column_letter(col)
                ws.column_dimensions[letter].width = temp.column_dimensions[letter].width
            templates = capture_templates(temp)
        else:
            ws = wb[sheet_name]
            templates = capture_templates(ws)
        captured.append(templates)
        if ws.max_row:
            ws.delete_rows(1, ws.max_row)

        ws.sheet_view.showGridLines = False
        ws.sheet_properties.pageSetUpPr.fitToPage = True
        ws.page_setup.orientation = "landscape"
        ws.page_setup.fitToWidth = 1
        ws.page_setup.fitToHeight = 0
        row = 1
        total = 0
        for tribunal in blocks:
            tribunal_records = records[sheet_name][tribunal]
            write_row(ws, row, [tribunal], templates["title"], memo=memo)
            set_height(ws, row, templates, "title")
            row += 1
            write_row(ws, row, [spec["report"]], templates["report"], memo=memo)
            set_height(ws, row, templates, "report")
            row += 1
            write_row(ws, row, [f"Fecha Emisión   : {emission_text}"], templates["date"], memo=memo)
            set_height(ws, row, templates, "date")
            row += 2
            if tribunal_records:
                write_row(ws, row, spec["headers"], templates["header"], memo=memo)
                set_height(ws, row, templates, "header")
                row += 1
                date_cols = (7, 8) if "Espera" in sheet_name else (6, 7, 8)
                for rec in tribunal_records:
                    write_row(ws, row, rec, templates["data"], date_cols=date_cols, memo=memo)
                    set_height(ws, row, templates, "data")
                    row += 1
                total += len(tribunal_records)
            else:
                write_row(ws, row, ["SIN REGISTROS"], templates["sin"], memo=memo)
                set_height(ws, row, templates, "sin")
                row += 1
            row += 1
        row += 1
        write_row(ws, row, ["TOTAL:", total], templates["total"], memo=memo)
        set_height(ws, row, templates, "total")
        summary[sheet_name] = total
    wb.active = 0
    return summary


def verify_meta_output(path: Path) -> tuple[dict[str, Any], dict[str, int], list[str]]:
    """Lee el TOTAL escrito y, por separado, recuenta las filas de datos.

    El recuento independiente cierra la verificación circular del programa
    original (§5.4 del informe v2.0).
    """
    wb = load_workbook(path, read_only=True, data_only=True)
    try:
        totals: dict[str, Any] = {}
        counted: dict[str, int] = {}
        problems: list[str] = []
        for sheet_name in SHEET_DEFS:
            ws = wb[sheet_name]
            total = None
            data_rows = 0
            in_block = False
            tribunal_labels = {norm(tribunal) for tribunal in TRIBUNALS}
            for row in ws.iter_rows(min_col=1, max_col=2, values_only=True):
                first = row[0]
                text = norm(first) if isinstance(first, str) else ""
                if text == "RIT":
                    in_block = True
                    continue
                if text.startswith("TOTAL"):
                    total = row[1]
                    in_block = False
                    continue
                if text in tribunal_labels or text == "SIN REGISTROS":
                    in_block = False
                    continue
                if in_block and first is not None and str(first).strip():
                    data_rows += 1
            totals[sheet_name] = total
            counted[sheet_name] = data_rows
            if total != data_rows:
                problems.append(
                    f"{sheet_name}: TOTAL declara {total} pero se contaron {data_rows} filas de datos"
                )
        return totals, counted, problems
    finally:
        wb.close()


