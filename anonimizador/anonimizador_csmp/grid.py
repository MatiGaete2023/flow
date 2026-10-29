"""Anonimización de tablas (hojas Excel, tablas HTML, CSV): una sola lógica para todos los formatos."""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from datetime import date, datetime

from .textengine import (
    DATE_DMY_RE,
    DATE_ISO_RE,
    OMITTED,
    RUT_RE,
    SAFE_STRUCT,
    curator_name,
    fake_rut,
    is_placeholder,
    normalize,
    scrub_unknown,
    shift_dates,
    shift_value,
)
from .vocab import SAFE_TITLE, STATE_WORDS, TECHNICAL, column_kind, is_birth, is_protected

HEADER_LIMIT = 60
IDENTITY_KINDS = ("rit", "rut", "curador", "persona")
MAX_CELLS = 3_000_000


@dataclass
class Sheet:
    title: str
    rows: list
    merged: list = field(default_factory=list)


def looks_like_person(text):
    """Celda que parece un nombre propio y no un título de columna conocido."""
    return column_kind(text) is None and not is_protected(text) and scrub_unknown(text) != text


def header_row(rows, limit=HEADER_LIMIT):
    """Primera fila con al menos dos títulos y una columna de identidad (RIT, RUT/RUN, nombre o curador)."""
    for index, row in enumerate(rows[:limit]):
        texts = [value for value in row if isinstance(value, str) and value.strip()]
        names = [normalize(value) for value in texts]
        if len(names) < 2 or any(len(n) > 40 for n in names):
            continue
        if any(value.strip().endswith(":") or looks_like_person(value) for value in texts):
            continue  # fila de pares «Nombre: valor» o con un nombre propio: son datos, no encabezados
        kinds = [column_kind(value) for value in texts]
        identity = sum(kind in IDENTITY_KINDS for kind in kinds)
        if identity >= 2 or any(
            n in ("rit", "rut", "run") or n.startswith(("nombre", "rut ", "rit ", "run ")) for n in names
        ):
            return index
    return None


def structural(text):
    """Valor sin datos personales posibles: fecha, número corto o estado conocido."""
    text = text.strip()
    if not text:
        return True
    if SAFE_STRUCT.fullmatch(text):
        return True
    return text.upper().replace(" ", "_") in STATE_WORDS


def label_kind(text):
    """Etiqueta de pares «Nombre: valor» o «Nombre | valor» usada en tablas sin encabezado."""
    label = text.strip().rstrip(":").strip()
    if not label or len(label) > 40 or not text.strip().endswith(":") and len(label.split()) > 3:
        return None
    kind = column_kind(label)
    if kind in IDENTITY_KINDS + ("mail", "telefono", "direccion"):
        return kind
    return "protegido" if is_protected(label) else None


def _plan(rows):
    index = header_row(rows)
    kinds = {}
    if index is not None:
        kinds = {col: column_kind(value) if isinstance(value, str) else None for col, value in enumerate(rows[index])}
    return index, kinds


def _pair_kinds(rows, index):
    """(fila, columna) -> clase para el valor que sigue a una etiqueta de identidad fuera del cuerpo."""
    pairs = {}
    last = len(rows) if index is None else index
    for r in range(last):
        row = rows[r]
        for c, value in enumerate(row):
            if isinstance(value, str) and (kind := label_kind(value)):
                for n in range(c + 1, len(row)):
                    if row[n] not in (None, ""):
                        pairs[r, n] = kind
                        break
    return pairs


def collect(sheets, session):
    """Primera pasada: nombres de columnas de identidad y causas, para toda la sesión."""
    for sheet in sheets:
        session.reserve_rits_in(sheet.title)
        for row in sheet.rows:
            for value in row:
                if isinstance(value, str):
                    session.reserve_rits_in(value)
        index, kinds = _plan(sheet.rows)
        for (r, c), kind in _pair_kinds(sheet.rows, index).items():
            if kind in ("persona", "curador"):
                _seed(session, kind, sheet.rows[r][c])
        if index is None:
            continue
        for row in sheet.rows[index + 1 :]:
            for col, value in enumerate(row):
                if isinstance(value, str) and kinds.get(col) in ("persona", "curador"):
                    _seed(session, kinds[col], value)
                elif isinstance(value, str) and kinds.get(col) == "json":
                    session.seed_json_text(value)


def _seed(session, kind, value):
    if not isinstance(value, str):
        return
    if kind == "persona":
        session.replacer.add_person(
            re.sub(r"(?i)^\s*(dupla|profesional|contacto|referente)\s*:\s*", "", clean_person(value))
        )
    elif kind == "curador":
        session.replacer.add_person(curator_name(value))


def clean_person(value):
    from .textengine import MAIL_RE, PHONE_RE

    return MAIL_RE.sub(" ", PHONE_RE.sub(" ", value))


def long_number(value):
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return False
    return float(value).is_integer() and 7 <= len(str(abs(int(value)))) <= 12


def identity_value(kind, value, session):
    """Valor de una columna de identidad."""
    pseudo, replacer = session.pseudonyms, session.replacer
    if isinstance(value, (datetime, date, bool)):
        return value
    text = str(value).strip()
    if kind == "rut":
        if isinstance(value, (int, float)) or re.fullmatch(r"\d{6,9}", text):
            fake = pseudo.get("rut", re.sub(r"\D", "", text))
            return int(fake.split("-")[0]) if isinstance(value, (int, float)) else fake.split("-")[0]
        if not re.search(r"\d", text):
            return value
        return RUT_RE.sub(lambda m: fake_rut(pseudo, m), text) if RUT_RE.search(text) else "#" * len(text)
    if is_placeholder(text):
        return value
    if kind in ("persona", "curador"):
        return replacer.text(text)
    if kind == "ident":
        return pseudo.get("numero", text)
    return pseudo.get(kind, text)


def transform(sheets, session):
    """Segunda pasada. Devuelve (hojas nuevas, textos para la revisión final, celdas cambiadas)."""
    total = sum(len(row) for sheet in sheets for row in sheet.rows)
    if total > MAX_CELLS:
        raise ValueError("El archivo supera el límite de tres millones de celdas.")
    out, texts, changed = [], [], 0
    used = set()
    for number, sheet in enumerate(sheets, 1):
        index, kinds = _plan(sheet.rows)
        pairs = _pair_kinds(sheet.rows, index)
        others = {s.title for s in sheets if s is not sheet}
        title = sheet.title if SAFE_TITLE.fullmatch(normalize(sheet.title)) else f"Hoja{number}"
        if title in used or (title != sheet.title and title in others):
            title = f"Hoja{number}"
        used.add(title)
        new_rows = []
        for r, row in enumerate(sheet.rows):
            body = index is not None and r > index
            new_row = []
            for c, value in enumerate(row):
                if value is None or value == "":
                    new_row.append(value)
                    continue
                header = sheet.rows[index][c] if index is not None and c < len(sheet.rows[index]) else None
                kind = kinds.get(c) if body else pairs.get((r, c))
                protected = body and isinstance(header, str) and is_protected(header) and kind is None
                if kind == "protegido":
                    kind, protected = None, True
                if index is not None and r == index:
                    new = _header(value, session)
                    protected = True
                else:
                    new = _cell(value, kind, protected, body, header, session)
                if new != value:
                    changed += 1
                if isinstance(new, str):
                    texts.append((new, protected))
                new_row.append(new)
            new_rows.append(new_row)
        out.append(Sheet(title, new_rows, sheet.merged))
        texts.append((title, True))
    return out, texts, changed


def _header(value, session):
    """Encabezado: se conserva, salvo identificadores o nombres completos conocidos (tablas dinámicas)."""
    return session.replacer.text(value, free=False, full_only=True) if isinstance(value, str) else value


def _cell(value, kind, protected, body, header, session):
    options, replacer, pseudo = session.options, session.replacer, session.pseudonyms
    days = options.shift_days
    if isinstance(value, bool):
        return value
    if isinstance(value, (datetime, date)):
        if options.generalize_birth and body and header and is_birth(header):
            return date(value.year, 1, 1) if not isinstance(value, datetime) else datetime(value.year, 1, 1)
        return shift_value(value, days)
    if isinstance(value, (int, float)):
        if kind == "rut":
            return identity_value("rut", value, session)
        if kind == "tech":
            return session.token(TECHNICAL[normalize(header)], str(int(value)))
        if kind in ("ident", "hash") or long_number(value):
            return pseudo.get("numero", str(int(value)) if float(value).is_integer() else str(value))
        return value
    if not isinstance(value, str):
        return value
    if kind in ("rut", "rit", "persona", "curador", "mail", "telefono", "direccion", "ident"):
        return identity_value(kind, value, session)
    if kind == "hash":
        return pseudo.get("hash", value.strip()) if value.strip() else value
    if kind == "tech":
        return session.token(TECHNICAL[normalize(header)], value)
    if kind == "json":
        return session.json_text(value)
    if kind == "texto":
        return session.free(value)
    if protected:
        text = replacer.text(value, free=False, phones=True, full_only=True)
        if options.generalize_birth and header and is_birth(header):
            text = DATE_ISO_RE.sub(lambda m: f"{m.group(1)}-01-01", text)
            text = DATE_DMY_RE.sub(lambda m: f"01{m.group(2)}01{m.group(2)}{m.group(4)}", text)
        return text
    stripped = value.strip()
    if stripped[:1] in "{[" and stripped[-1:] in "}]":
        return session.json_text(value)
    if structural(value):
        return shift_dates(value, days)
    return OMITTED
