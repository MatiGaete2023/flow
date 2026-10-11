"""Utilidades de lectura RUS que usa el Asistente (antes en rus/rules.py, retirado en WP-21)."""
from __future__ import annotations

import math
import re
from datetime import date, datetime, timedelta

from .columns import normalize

_EMPTY = {"", "---", "-", "nan", "nat", "none"}
_PARENS = re.compile(r"\(.*?\)|[()]")
_ISO_DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
_ISO_DATETIME = re.compile(
    r"^\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?$"
)


def as_date(value: object, excel_epoch: str = "1900") -> date | None:
    if value is None or isinstance(value, bool):
        return None
    if isinstance(value, datetime):
        return value.date()
    if isinstance(value, date):
        return value
    if isinstance(value, (int, float)) and not isinstance(value, bool) and 0 < float(value) < 100000:
        base = date(1904, 1, 1) if excel_epoch == "1904" else date(1899, 12, 30)
        try:
            return base + timedelta(days=float(value))
        except (OverflowError, ValueError):
            return None
    text = str(value).strip()
    if text.casefold() in _EMPTY:
        return None
    if _ISO_DATE.fullmatch(text):
        try:
            return datetime.strptime(text, "%Y-%m-%d").date()
        except ValueError:
            return None
    if _ISO_DATETIME.fullmatch(text):
        candidate = text[:-1] + "+00:00" if text.endswith("Z") else text
        try:
            return datetime.fromisoformat(candidate).date()
        except ValueError:
            return None
    for fmt in ("%d/%m/%Y", "%d-%m-%Y"):
        try:
            return datetime.strptime(text, fmt).date()
        except ValueError:
            pass
    return None


def as_int(value: object) -> int | None:
    if value is None or isinstance(value, bool):
        return None
    text = str(value).strip().casefold()
    if text in _EMPTY:
        return None
    try:
        number = float(text.replace(",", "."))
    except ValueError:
        return None
    return int(number) if math.isfinite(number) and number.is_integer() else None


def tribunal(value: object) -> str | None:
    text = normalize(value).upper()
    if "MULCHEN" in text:
        return "MULCHEN"
    if "LAJA" in text:
        return "LAJA"
    if "TOME" in text:
        return "TOME"
    return None


def historical_match(value: object) -> str:
    return " ".join(_PARENS.sub("", str(value or "")).strip().casefold().split())
