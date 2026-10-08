from __future__ import annotations

import hashlib
import json
from pathlib import Path


class CatalogError(ValueError):
    pass


def default_catalog_path() -> Path:
    return Path(__file__).with_name("textos_observaciones.json")


def catalog_sha256(path: str | Path | None = None) -> str:
    catalog_path = Path(path) if path else default_catalog_path()
    try:
        return hashlib.sha256(catalog_path.read_bytes()).hexdigest()
    except OSError as exc:
        raise CatalogError(f"No se pudo abrir el catálogo: {catalog_path}") from exc


def load_catalog(path: str | Path | None = None) -> dict[str, dict[str, dict[str, object]]]:
    return load_catalog_snapshot(path)[0]


def load_catalog_snapshot(path: str | Path | None = None):
    """Valida textos y calcula el hash sobre una única lectura de bytes."""
    catalog_path = Path(path) if path else default_catalog_path()
    try:
        content = catalog_path.read_bytes()
        raw = json.loads(content.decode("utf-8"))
    except OSError as exc:
        raise CatalogError(f"No se pudo abrir el catálogo: {catalog_path}") from exc
    except (json.JSONDecodeError, UnicodeError) as exc:
        raise CatalogError(f"El catálogo no contiene JSON válido: {exc}") from exc
    if not isinstance(raw, dict):
        raise CatalogError("El catálogo debe contener un objeto JSON.")
    return raw, hashlib.sha256(content).hexdigest()


def render(catalog: dict, scope: str, key: str, **values: str) -> str:
    try:
        text = catalog[scope][key]["texto"]
    except (KeyError, TypeError) as exc:
        raise CatalogError(f"Texto no definido: {scope}.{key}") from exc
    try:
        return str(text).format_map(values)
    except KeyError as exc:
        raise CatalogError(f"Falta el campo {exc.args[0]} para {scope}.{key}") from exc
