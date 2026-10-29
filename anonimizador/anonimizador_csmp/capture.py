"""Paquetes CSMP_CAPTURE_V1 (ZIP del Descargador): tablas CSV relacionadas, manifiesto con huellas y consolidado.

La copia se reconstruye con vínculos coherentes (las mismas claves ficticias en todas las tablas y en el
consolidado) y un manifiesto nuevo con tamaños y huellas recalculados. Los documentos y originales que no se pueden
anonimizar se excluyen y las filas que los referían pasan a NO_DISPONIBLE; nunca se copian sin tratar.
"""

from __future__ import annotations

import csv
import json
import secrets
from copy import deepcopy
from hashlib import sha256
from io import StringIO

from . import jsonfmt, safezip
from .textengine import shift_dates

MANIFEST = "manifest.json"
CONSOLIDATED_PATH = "consolidado/consolidado.xlsx"
COMMON = (
    "capture_id",
    "query_id",
    "source_system",
    "court_code",
    "court_name",
    "rit_original",
    "cause_id",
    "ingreso_id",
    "person_id",
    "program_id",
    "modality_raw",
    "modality_code",
    "tab",
    "captured_at",
    "original_file",
    "original_sheet",
    "original_row",
    "source_sha256",
    "coverage",
)
TABLES = {
    "consultas": (
        "query_id",
        "source_system",
        "screen",
        "tab",
        "court_code",
        "court_name",
        "modality_code",
        "coverage",
        "captured_at",
        "filters_json",
        "expected_pages",
        "saved_pages",
        "period_from",
        "period_to",
        "time_zone",
        "history_complete",
        "institutional_verified",
        "error",
    ),
    "registros": COMMON + ("record_id", "record_type", "identity_status", "raw_json", "normalized_json"),
    "bitacoras": COMMON
    + (
        "event_id",
        "event_at",
        "time_precision",
        "time_zone",
        "origin",
        "origin_verified",
        "author",
        "event_type",
        "text",
        "response",
        "response_at",
        "identity_status",
    ),
    "calendarios": COMMON
    + ("calendar_id", "event_at", "event_type", "status", "association", "raw_json", "normalized_json"),
    "historia_tramites": COMMON
    + (
        "event_id",
        "event_at",
        "time_precision",
        "time_zone",
        "scope",
        "event_type",
        "description",
        "reference",
        "status",
        "document_id",
        "identity_status",
    ),
    "documentos": COMMON
    + (
        "document_id",
        "parent_event_id",
        "scope",
        "reference",
        "local_file",
        "sha256",
        "download_state",
        "content_read",
    ),
    "incidencias": (
        "capture_id",
        "query_id",
        "record_id",
        "incident_id",
        "phase",
        "code",
        "severity",
        "message",
        "recoverable",
    ),
}
JSON_FIELDS = ("raw_json", "normalized_json", "filters_json")
ID_FIELDS = {"query_id", "record_id", "event_id", "incident_id", "calendar_id", "document_id", "parent_event_id"}
REMOTE_ID = {"cause_id", "ingreso_id", "person_id", "program_id"}
FREE_FIELDS = {"text", "response", "description", "reference", "message", "error"}
TIME_FIELDS = {"captured_at", "event_at", "response_at", "period_from", "period_to"}
FILTER_KEEP = frozenset(
    {"tribunal", "tribunal_nombre", "modality", "screen", "tab", "source_system", "court_code", "capture_tables"}
)
CONTROL_FIELDS = {
    "source_system",
    "screen",
    "tab",
    "court_code",
    "court_name",
    "modality_code",
    "modality_raw",
    "record_type",
    "identity_status",
    "coverage",
    "event_type",
    "time_precision",
    "time_zone",
    "scope",
    "origin",
    "origin_verified",
    "status",
    "association",
    "phase",
    "code",
    "severity",
    "recoverable",
    "history_complete",
    "institutional_verified",
    "expected_pages",
    "saved_pages",
    "original_row",
    "download_state",
    "content_read",
}


class BadCapture(ValueError):
    pass


def is_capture(entries):
    for name, raw in entries:
        if name == MANIFEST:
            try:
                return json.loads(raw.decode("utf-8-sig")).get("schema_id") == "CSMP_CAPTURE"
            except (ValueError, AttributeError):
                return False
    return False


def parse(entries):
    """Manifiesto, tablas, consolidado y originales; comprueba tamaños y huellas declarados."""
    files = dict(entries)
    try:
        manifest = json.loads(files[MANIFEST].decode("utf-8-sig"))
        declared = manifest["files"]
    except (KeyError, ValueError, UnicodeError, TypeError) as exc:
        raise BadCapture("Manifiesto de captura ilegible.") from exc
    if not isinstance(declared, list) or not isinstance(manifest.get("table_states"), dict):
        raise BadCapture("Manifiesto de captura incompleto.")
    if manifest.get("required_features"):
        raise BadCapture("El paquete requiere funcionalidades desconocidas.")
    if {f.get("path") for f in declared if isinstance(f, dict)} | {MANIFEST} != set(files):
        raise BadCapture("Los archivos del ZIP no coinciden con el manifiesto.")
    tables, consolidated, originals = {}, None, {}
    csv.field_size_limit(16 * 1024 * 1024)
    for item in declared:
        raw = files[item["path"]]
        if len(raw) != item.get("bytes") or sha256(raw).hexdigest() != item.get("sha256"):
            raise BadCapture("Huella o tamaño distinto del declarado.")
        kind = item.get("type")
        if kind == "normalized_table":
            table = item.get("table")
            if table not in TABLES:
                raise BadCapture("Tabla desconocida; actualiza la herramienta antes de anonimizar.")
            reader = csv.DictReader(StringIO(raw.decode("utf-8-sig"), newline=""), strict=True)
            fields = reader.fieldnames or []
            if tuple(fields[: len(TABLES[table])]) != TABLES[table] or len(set(fields)) != len(fields):
                raise BadCapture("Encabezado de tabla incompatible.")
            rows = list(reader)
            if any(None in row or any(v is None for v in row.values()) for row in rows):
                raise BadCapture("Fila CSV irregular.")
            tables[table] = (list(fields), rows)
        elif kind == "consolidated_workbook":
            consolidated = raw
        elif kind == "original":
            originals[item["path"]] = raw
        else:
            raise BadCapture("Tipo de archivo desconocido en el manifiesto.")
    if "consultas" not in tables:
        raise BadCapture("Falta la tabla de consultas.")
    return manifest, tables, consolidated, originals


def collect(entries, session, collect_bytes):
    manifest, tables, consolidated, originals = parse(entries)
    for fields, rows in tables.values():
        for row in rows:
            for key, value in row.items():
                if not value:
                    continue
                session.reserve_rits_in(value)
                if key == "author" and value.casefold() not in ("desconocido", "sin identificar"):
                    session.replacer.add_person(value)
                elif key in JSON_FIELDS or key not in COMMON and value[:1] in "{[":
                    jsonfmt.seed_text(value, session)
    for path, raw in ([(CONSOLIDATED_PATH, consolidated)] if consolidated else []) + list(originals.items()):
        collect_bytes(raw, path)


def _field(key, value, session, capture_id, texts):
    options = session.options
    if value == "":
        return value
    if key in JSON_FIELDS:
        return jsonfmt.text(value, session, texts, keep=FILTER_KEEP if key == "filters_json" else frozenset())
    if key in ID_FIELDS or key in REMOTE_ID or key in ("source_sha256", "sha256", "original_file", "original_sheet"):
        return session.token(key, value)
    if key == "capture_id":
        return capture_id
    if key == "rit_original":
        return session.pseudonyms.get("rit", value)
    if key == "author":
        if value.casefold() in ("desconocido", "sin identificar"):
            return value
        session.replacer.add_person(value)
        return session.replacer.text(value)
    if key in FREE_FIELDS:
        return session.free(value)
    if key in TIME_FIELDS:
        return shift_dates(value, options.shift_days)
    if key in CONTROL_FIELDS or key in ("court_name",):
        return session.replacer.text(value, free=False, full_only=True)
    new = jsonfmt.value(key, value, session)
    return new if isinstance(new, str) else str(new)


def transform(entries, session, process_bytes):
    """Devuelve (ZIP nuevo, textos para la revisión). `process_bytes(raw, nombre)` anonimiza un archivo interno."""
    manifest, tables, consolidated, originals = parse(entries)
    capture_id = "anon-" + secrets.token_hex(12)
    texts, dropped = [], 0
    new_originals = {}
    for index, (path, raw) in enumerate(sorted(originals.items()), 1):
        try:
            data, ext, _ = process_bytes(raw, path)
        except Exception:
            dropped += 1
            continue
        folder = path.rsplit("/", 1)[0] if "/" in path else "originales"
        if not folder.startswith(("originales", "evidencias")):
            folder = "originales"
        new_originals[path] = (f"{folder}/fuente_{index}{ext}", data)
    out_tables = {}
    for name, (fields, rows) in tables.items():
        new_rows = []
        for row in rows:
            new = {key: _field(key, value, session, capture_id, texts) for key, value in row.items()}
            texts.extend((v, key in CONTROL_FIELDS) for key, v in new.items() if key not in JSON_FIELDS)
            if name == "documentos":
                local = row.get("local_file", "")
                if local in new_originals:
                    new["local_file"] = new_originals[local][0]
                    new["sha256"] = sha256(new_originals[local][1]).hexdigest()
                else:
                    new["local_file"] = ""
                    if row.get("download_state") == "DESCARGADO":
                        new["download_state"], new["sha256"] = "NO_DISPONIBLE", ""
            new_rows.append(new)
        out_tables[name] = (fields, new_rows)
    payload, files = {}, []
    for name, (fields, rows) in out_tables.items():
        stream = StringIO(newline="")
        writer = csv.DictWriter(stream, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
        raw = stream.getvalue().encode("utf-8")
        path = f"data/{name}.csv"
        payload[path] = raw
        files.append(
            {
                "path": path,
                "table": name,
                "type": "normalized_table",
                "rows": len(rows),
                "bytes": len(raw),
                "sha256": sha256(raw).hexdigest(),
            }
        )
    for path, (new_path, data) in new_originals.items():
        payload[new_path] = data
        files.append({"path": new_path, "type": "original", "bytes": len(data), "sha256": sha256(data).hexdigest()})
    if consolidated is not None:
        data, _, _ = process_bytes(consolidated, CONSOLIDATED_PATH)
        payload[CONSOLIDATED_PATH] = data
        files.append(
            {
                "path": CONSOLIDATED_PATH,
                "type": "consolidated_workbook",
                "bytes": len(data),
                "sha256": sha256(data).hexdigest(),
            }
        )
    version = "1.1.0" if consolidated is not None else "1.0.0"
    new_manifest = {
        "schema_id": "CSMP_CAPTURE",
        "schema_version": version,
        "capture_id": capture_id,
        "captured_at": shift_dates(str(manifest.get("captured_at", "")), session.options.shift_days),
        "downloader": {"version": "anonimizador-local", "commit": "COPIA_LOCAL"},
        "contains_sensitive_data": True,
        "originals_authorized": bool(new_originals),
        "queries": deepcopy([row for row in out_tables["consultas"][1]]),
        "table_states": deepcopy(manifest["table_states"]),
        "mapping_version": str(manifest.get("mapping_version", "")),
        "required_features": [],
        "files": files,
    }
    payload[MANIFEST] = json.dumps(new_manifest, ensure_ascii=False, sort_keys=True, indent=2).encode("utf-8")
    if dropped and not session.options.skip_unsupported:
        raise BadCapture("El paquete contiene originales no admitidos y se pidió rechazarlo completo.")
    if dropped:
        session.report.warn(
            f"{dropped} original(es)/documento(s) del paquete no se pudieron anonimizar y se excluyeron; "
            "sus filas quedaron como NO_DISPONIBLE."
        )
    data = safezip.build(list(payload.items()))
    parse(safezip.members(data))  # autocomprobación: huellas, tablas y manifiesto coherentes
    return data, texts
