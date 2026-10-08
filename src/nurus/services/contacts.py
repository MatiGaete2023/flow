from __future__ import annotations

import hashlib
import os
import re
import tempfile
from dataclasses import dataclass, replace
from collections import Counter
from pathlib import Path

from nurus.rus.columns import normalize
from nurus.storage.database import Database


class ContactImportError(ValueError):
    pass


@dataclass(frozen=True)
class ContactChange:
    entity_key: str
    display_name: str
    email: str
    cc: str
    aliases: tuple[str, ...]
    action: str
    existing_email: str = ""
    existing_cc: str = ""
    issue: str = ""


@dataclass(frozen=True)
class ContactImportPreview:
    source_name: str
    source_sha256: str
    sheet_name: str
    changes: tuple[ContactChange, ...]

    @property
    def conflicts(self) -> tuple[ContactChange, ...]:
        return tuple(item for item in self.changes if item.action == "conflict")


_ENTITY_HEADERS = ("PROGRAMA", "ENTIDAD", "NOMBRE PROGRAMA", "NOMBRE CENTRO", "CENTRO")
_EMAIL_HEADERS = ("CORREO", "CORREO ELECTRONICO", "CORREO ELECTRÓNICO", "EMAIL", "E-MAIL")
_CC_HEADERS = ("CC", "COPIA", "CORREO CC")
_ALIAS_HEADERS = ("ALIAS", "ALIAS PROGRAMA", "ALIASES")
_EMAIL_RE = re.compile(r"^[^\s@;,@]+@[^\s@;,@]+\.[^\s@;,@]+$")


def _column(headers: list[str], aliases: tuple[str, ...], *, required: bool) -> str:
    candidates = [
        header for header in headers
        if normalize(header) in {normalize(alias) for alias in aliases}
    ]
    candidates = list(dict.fromkeys(candidates))
    if len(candidates) > 1:
        raise ContactImportError(f"Columnas ambiguas: {', '.join(candidates)}.")
    if required and not candidates:
        raise ContactImportError(f"Falta una columna requerida: {' / '.join(aliases)}.")
    return candidates[0] if candidates else ""


def _addresses(value: object) -> tuple[str, ...]:
    raw = str(value or "").strip()
    if not raw:
        return ()
    result = [item.strip() for item in re.split(r"[;,]", raw) if item.strip()]
    return tuple(dict.fromkeys(result))


def _valid_addresses(value: str) -> bool:
    addresses = _addresses(value)
    return bool(addresses) and all(_EMAIL_RE.fullmatch(item) for item in addresses)


def _snapshot(path: Path) -> tuple[Path, str]:
    digest = hashlib.sha256()
    temp: Path | None = None
    try:
        with path.open("rb") as source, tempfile.NamedTemporaryFile(suffix=path.suffix, delete=False) as target:
            temp = Path(target.name)
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                digest.update(chunk)
                target.write(chunk)
        return temp, digest.hexdigest()
    except OSError as exc:
        if temp:
            temp.unlink(missing_ok=True)
        raise ContactImportError(f"No se pudo copiar el catastro: {exc}") from exc


def preview_contact_import(
    db: Database,
    path: str | Path,
    *,
    sheet_name: str | None = None,
    header_row: int = 1,
) -> ContactImportPreview:
    """Lee un catastro y devuelve cambios; no modifica SQLite ni Outlook."""
    if header_row < 1:
        raise ContactImportError("header_row debe ser mayor o igual a 1.")
    try:
        import pandas as pd
    except ImportError as exc:  # pragma: no cover
        raise ContactImportError("Falta pandas.") from exc

    source = Path(path).expanduser().resolve()
    if not source.is_file():
        raise ContactImportError(f"Archivo no encontrado: {source}")
    if source.suffix.lower() not in {".xls", ".xlsx", ".xlsm"}:
        raise ContactImportError("El catastro debe ser .xls, .xlsx o .xlsm.")

    temp, digest = _snapshot(source)
    try:
        engine = "xlrd" if source.suffix.lower() == ".xls" else "openpyxl"
        try:
            book = pd.ExcelFile(temp, engine=engine)
        except Exception as exc:
            raise ContactImportError(f"No se pudo abrir el catastro: {exc}") from exc
        with book:
            names = list(book.sheet_names)
            if not names:
                raise ContactImportError("El catastro no contiene hojas.")
            selected = sheet_name or names[0]
            if selected not in names:
                raise ContactImportError(f"No existe la hoja {selected!r}.")
            try:
                frame = pd.read_excel(
                    book,
                    sheet_name=selected,
                    header=header_row - 1,
                    dtype=object,
                    keep_default_na=False,
                )
            except Exception as exc:
                raise ContactImportError(f"No se pudo leer la hoja: {exc}") from exc
    finally:
        try:
            os.unlink(temp)
        except OSError:
            pass

    headers = [str(item).strip() for item in frame.columns]
    entity_col = _column(headers, _ENTITY_HEADERS, required=True)
    email_col = _column(headers, _EMAIL_HEADERS, required=True)
    cc_col = _column(headers, _CC_HEADERS, required=False)
    alias_col = _column(headers, _ALIAS_HEADERS, required=False)

    existing = {row["entity_key"]: row for row in db.list_contacts()}
    seen: set[str] = set()
    changes: list[ContactChange] = []
    for _, row in frame.iterrows():
        display_name = str(row.get(entity_col, "") or "").strip()
        email = "; ".join(_addresses(row.get(email_col, "")))
        cc = "; ".join(_addresses(row.get(cc_col, ""))) if cc_col else ""
        aliases = tuple(
            item.strip() for item in re.split(r"[;,]", str(row.get(alias_col, "") or "")) if item.strip()
        )
        aliases = tuple(dict.fromkeys(aliases))
        if not display_name and not email:
            continue
        entity_key = normalize(display_name)
        issue = ""
        action = "add"
        current = existing.get(entity_key)
        if not display_name:
            issue = "Falta nombre de entidad."
        elif not _valid_addresses(email):
            issue = "Correo ausente o con sintaxis inválida."
        elif cc and not all(_EMAIL_RE.fullmatch(item) for item in _addresses(cc)):
            issue = "CC con sintaxis inválida."
        elif entity_key in seen:
            issue = "Entidad duplicada dentro del catastro."
        if issue:
            action = "conflict"
        elif current:
            if current["display_name"] == display_name and current["email"] == email and current["cc"] == cc:
                action = "unchanged"
            else:
                action = "update"
        seen.add(entity_key)
        changes.append(
            ContactChange(
                entity_key=entity_key,
                display_name=display_name,
                email=email,
                cc=cc,
                aliases=aliases,
                action=action,
                existing_email=current["email"] if current else "",
                existing_cc=current["cc"] if current else "",
                issue=issue,
            )
        )
    counts = Counter(item.entity_key for item in changes)
    changes = [
        replace(item, action="conflict", issue="Entidad duplicada dentro del catastro.")
        if counts[item.entity_key] > 1 else item
        for item in changes
    ]
    return ContactImportPreview(source.name, digest, selected, tuple(changes))


def apply_contact_preview(
    db: Database,
    preview: ContactImportPreview,
    accepted_entity_keys: set[str] | tuple[str, ...] | list[str],
) -> int:
    """Aplica en una sola transacción solo altas/cambios aceptados.

    No modifica contactos de Outlook. Los alias son explícitos y exactos.
    """
    accepted = set(accepted_entity_keys)
    eligible = {
        item.entity_key: item
        for item in preview.changes
        if item.action in {"add", "update"}
    }
    for conflict in preview.conflicts:
        eligible.pop(conflict.entity_key, None)
    unknown = accepted - set(eligible)
    if unknown:
        raise ContactImportError("La selección contiene contactos no aplicables o en conflicto.")
    if not accepted:
        return 0

    with db.connect() as conn:
        conn.execute("BEGIN IMMEDIATE")
        conn.execute(
            """CREATE TABLE IF NOT EXISTS contact_aliases(
                   alias_key TEXT PRIMARY KEY,
                   contact_id TEXT NOT NULL,
                   display_alias TEXT NOT NULL,
                   FOREIGN KEY(contact_id) REFERENCES contacts(id) ON DELETE CASCADE
               )"""
        )
        conn.execute(
            """CREATE TABLE IF NOT EXISTS contact_imports(
                   id INTEGER PRIMARY KEY AUTOINCREMENT,
                   source_name TEXT NOT NULL,
                   source_hash TEXT NOT NULL,
                   sheet_name TEXT NOT NULL,
                   applied_count INTEGER NOT NULL,
                   applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
               )"""
        )
        count = 0
        for key in sorted(accepted):
            item = eligible[key]
            current = conn.execute("SELECT * FROM contacts WHERE entity_key=?", (key,)).fetchone()
            if ((item.action == "add" and current is not None)
                or (item.action == "update" and (current is None
                    or current["email"] != item.existing_email or current["cc"] != item.existing_cc))):
                raise ContactImportError("El contacto cambió desde la vista previa; importa nuevamente.")
            # Una identidad nueva tampoco puede ocultar el alias de otra entidad.
            owner = conn.execute("SELECT contact_id FROM contact_aliases WHERE alias_key=?", (key,)).fetchone()
            if owner and (current is None or owner[0] != current["id"]):
                raise ContactImportError("La identidad coincide con un alias de otro contacto.")
            conn.execute(
                """INSERT INTO contacts(id,entity_key,display_name,email,cc,active,updated_at)
                   VALUES(?,?,?,?,?,?,CURRENT_TIMESTAMP)
                   ON CONFLICT(entity_key) DO UPDATE SET
                   display_name=excluded.display_name,email=excluded.email,cc=excluded.cc,
                   active=1,updated_at=CURRENT_TIMESTAMP""",
                (item.entity_key, item.entity_key, item.display_name, item.email, item.cc, 1),
            )
            contact_id = conn.execute("SELECT id FROM contacts WHERE entity_key=?", (key,)).fetchone()[0]
            conn.execute("DELETE FROM contact_aliases WHERE contact_id=?", (contact_id,))
            for alias in item.aliases:
                alias_key = normalize(alias)
                if not alias_key or alias_key == item.entity_key:
                    continue
                if conn.execute("SELECT 1 FROM contacts WHERE entity_key=? AND id<>?", (alias_key, contact_id)).fetchone():
                    raise ContactImportError("El alias coincide con la identidad de otro contacto.")
                try:
                    conn.execute(
                        "INSERT INTO contact_aliases(alias_key,contact_id,display_alias) VALUES(?,?,?)",
                        (alias_key, contact_id, alias),
                    )
                except Exception as exc:
                    raise ContactImportError(
                        f"El alias {alias!r} ya pertenece a otro contacto; no se aplicó el lote."
                    ) from exc
            count += 1
        conn.execute(
            "INSERT INTO contact_imports(source_name,source_hash,sheet_name,applied_count) VALUES(?,?,?,?)",
            (preview.source_name, preview.source_sha256, preview.sheet_name, count),
        )
    return count


def resolve_contact_exact(db: Database, entity_or_alias: str):
    """Resuelve solo identidad o alias explícito normalizado; nunca por similitud."""
    key = normalize(entity_or_alias)
    with db.connect() as conn:
        direct = conn.execute(
            "SELECT * FROM contacts WHERE entity_key=? AND active=1", (key,)
        ).fetchone()
        if direct is not None:
            return direct
        try:
            return conn.execute(
                """SELECT c.* FROM contact_aliases a
                   JOIN contacts c ON c.id=a.contact_id
                   WHERE a.alias_key=? AND c.active=1""",
                (key,),
            ).fetchone()
        except Exception:
            return None

