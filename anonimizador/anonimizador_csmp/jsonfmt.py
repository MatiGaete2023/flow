"""Anonimización de JSON arbitrario (resumen, flujo, bitácoras, tablas de captura, estructuras anidadas)."""

from __future__ import annotations

import json
import re

from .textengine import OMITTED, curator_name, scrub_unknown
from .vocab import SAFE_EXTRA_KEYS, column_kind, is_protected

MAX_DEPTH = 60
MAX_ITEMS = 2_000_000
PATH_KEYS = {
    "folder",
    "carpeta",
    "ruta",
    "directorio",
    "directory",
    "target",
    "destino",
    "origen",
    "source",
    "file",
    "archivo",
    "local_file",
    "original_file",
    "manifest",
    "excel",
    "diagnostico_anonimo",
    "path",
    "filename",
}
IDENT_KEYS = {"query_id", "record_id", "event_id", "incident_id", "calendar_id", "document_id", "parent_event_id"}


class BadJson(ValueError):
    pass


def loads(text):
    try:
        return json.loads(text, parse_constant=lambda _: None)
    except (ValueError, RecursionError) as exc:
        raise BadJson("JSON no válido.") from exc


def _is_path_value(key, value):
    return (
        key.lower() in PATH_KEYS
        and key.lower() != "path"
        or bool(re.match(r"^(?:[A-Za-z]:[\\/]|\\\\|/(?:home|Users|mnt|tmp|var)/)", value))
    )


def _path(value, session):
    base = re.split(r"[\\/]+", value.rstrip("\\/"))[-1]
    return session.name_map.get(base) or session.replacer.text(base, free=True)


def seed(obj, session, key="", depth=0):
    """Reserva causas y registra personas antes de reemplazar nada (primera pasada)."""
    if depth > MAX_DEPTH:
        raise BadJson("Estructura JSON demasiado profunda.")
    if isinstance(obj, dict):
        for label, child in obj.items():
            seed(child, session, str(label), depth + 1)
    elif isinstance(obj, list):
        for child in obj:
            seed(child, session, key, depth + 1)
    elif isinstance(obj, str):
        session.reserve_rits_in(obj)
        kind = column_kind(key)
        if kind == "persona" and len(obj) < 200:
            session.replacer.add_person(obj)
        elif kind == "curador":
            session.replacer.add_person(curator_name(obj))
        elif kind == "json" or obj.lstrip()[:1] in "{[" and obj.rstrip()[-1:] in "}]":
            seed_text(obj, session)


def seed_text(text, session):
    try:
        seed(loads(text), session)
    except BadJson:
        session.reserve_rits_in(text)


def value(key, item, session, keep=frozenset()):
    from .grid import identity_value, long_number

    pseudo, replacer = session.pseudonyms, session.replacer
    if item is None or isinstance(item, bool) or key in keep:
        return item
    low = key.lower()
    if isinstance(item, (int, float)):
        if low in IDENT_KEYS or low.endswith("_id"):
            return session.token(low, str(item))
        if column_kind(key) in ("ident", "hash") or long_number(item):
            return pseudo.get("numero", str(item))
        return item
    if not isinstance(item, str):
        return OMITTED
    if item == "":
        return item
    if low in IDENT_KEYS or low.endswith("_id"):
        return session.token(low, item)
    kind = column_kind(key)
    if kind in ("rut", "rit", "persona", "curador", "mail", "telefono", "direccion", "ident"):
        return identity_value(kind, item, session)
    if kind == "hash":
        return pseudo.get("hash", item) if item.strip() else item
    if low in ("source_sha256", "sha256", "sitfa_sha256"):
        return session.token(low, item)
    if _is_path_value(key, item):
        return _path(item, session)
    stripped = item.strip()
    if stripped[:1] in "{[" and stripped[-1:] in "}]":
        return text(item, session)
    if kind == "json":
        return text(item, session)
    if kind == "texto":
        return session.free(item)
    if low in SAFE_EXTRA_KEYS or is_protected(key) or structural_value(item):
        return replacer.text(item, free=False, phones=True, full_only=True)
    return session.free(item) if not session.strict else OMITTED


def structural_value(item):
    from .grid import structural

    return structural(item)


def key_name(label, session, taken):
    clean = session.replacer.text(label, free=False)
    if " " in clean.strip() and scrub_unknown(clean) != clean:
        clean = f"clave_{len(taken) + 1}"
    while clean in taken:
        clean += "_"
    taken.add(clean)
    return clean


def walk(obj, session, key="", depth=0, texts=None, keep=frozenset()):
    if depth > MAX_DEPTH:
        raise BadJson("Estructura JSON demasiado profunda.")
    if isinstance(obj, dict):
        result, taken = {}, set()
        for label, child in obj.items():
            new_label = key_name(str(label), session, taken)
            if texts is not None:
                texts.append((new_label, False))
            result[new_label] = walk(child, session, str(label), depth + 1, texts, keep)
        return result
    if isinstance(obj, list):
        if len(obj) > MAX_ITEMS:
            raise BadJson("Lista JSON excesiva.")
        return [walk(child, session, key, depth + 1, texts, keep) for child in obj]
    new = value(key, obj, session, keep)
    if texts is not None and isinstance(new, str):
        texts.append((new, is_protected(key)))
    return new


def text(raw, session, texts=None, keep=frozenset()):
    """JSON en texto -> JSON anonimizado en texto; si no es JSON válido, se trata como texto libre."""
    try:
        parsed = loads(raw)
    except BadJson:
        new = session.free(raw)
        if texts is not None and isinstance(new, str):
            texts.append((new, False))
        return new
    return json.dumps(walk(parsed, session, texts=texts, keep=keep), ensure_ascii=False, separators=(",", ":"))


def anonymize_bytes(data, session):
    """Archivo .json o .jsonl -> (bytes, textos de revisión)."""
    raw = decode(data)
    texts = []
    stripped = raw.strip()
    try:
        parsed = loads(stripped)
    except BadJson:
        lines = [line for line in raw.splitlines() if line.strip()]
        if len(lines) < 2:
            raise
        out = [json.dumps(walk(loads(line), session, texts=texts), ensure_ascii=False) for line in lines]
        return ("\n".join(out) + "\n").encode("utf-8"), texts
    result = walk(parsed, session, texts=texts)
    return (json.dumps(result, ensure_ascii=False, indent=2) + "\n").encode("utf-8"), texts


def seed_bytes(data, session):
    raw = decode(data)
    try:
        seed(loads(raw.strip()), session)
    except BadJson:
        for line in raw.splitlines():
            if line.strip():
                seed_text(line, session)


def decode(data):
    for encoding in ("utf-8-sig", "cp1252"):
        try:
            return data.decode(encoding)
        except UnicodeDecodeError:
            continue
    raise BadJson("Codificación no admitida.")
