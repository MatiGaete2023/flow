from __future__ import annotations

import math
import re
import unicodedata
from datetime import date, datetime, timedelta
from typing import Mapping

from .catalog import render
from .columns import normalize
from .models import Issue, SourceRecord

_EMPTY = {"", "---", "-", "nan", "nat", "none"}
_MONTHS = ("", "enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre")
_RUT = re.compile(r"\d{6,8}-[\dkK]")
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


def format_date(value: date) -> str:
    return f"{value.day} de {_MONTHS[value.month]} de {value.year}"


def clean_name(value: object) -> str:
    return " ".join(token.capitalize() for token in _PARENS.sub("", str(value or "")).split())


def program_title(value: object) -> str:
    prepositions = {"de", "del", "los", "las", "y", "en", "el", "la", "por", "con", "a"}
    tokens = re.split(r"[\s-]+", " ".join(str(value or "").split()))
    result = []
    for index, token in enumerate(tokens):
        letters = re.sub(r"[^A-Za-zÁÉÍÓÚáéíóúÜüÑñ]", "", token)
        normalized = normalize(token)
        if index and normalized in prepositions:
            result.append(token.lower())
        elif len(letters) <= 4 and letters.isalpha() and normalized not in prepositions:
            result.append(token.upper())
        else:
            result.append(token.capitalize())
    return " ".join(result)


def prefix(row: Mapping[str, object], columns: Mapping[str, str]) -> str:
    first = clean_name(row.get(columns.get("nombre", ""), "")).split()
    program = unicodedata.normalize("NFKD", str(row.get(columns.get("programa", ""), "") or "")).encode("ascii", "ignore").decode().upper()[:3]
    if first and program:
        return f"{first[0]} {program}: "
    if first:
        return f"{first[0]}: "
    return f"{program}: " if program else ""


def tribunal(value: object) -> str | None:
    text = normalize(value).upper()
    if "MULCHEN" in text:
        return "MULCHEN"
    if "LAJA" in text:
        return "LAJA"
    if "TOME" in text:
        return "TOME"
    return None


def is_dce(value: object) -> bool:
    text = normalize(value).upper()
    return "DCE" in text or "DIAGNOSTICO" in text


def _starts_term(text: str, term: str) -> bool:
    return text == term or text.startswith(term + " ")


def no_follow_up(value: object) -> bool:
    text = normalize(value)
    terms = (
        "opd",
        "dam",
        "salud privada",
        "hospital",
        "unidad de salud mental",
        "cesfam",
        "red salud",
        "consulta externa",
        "colegio",
        "chile crece contigo",
    )
    return any(_starts_term(text, term) for term in terms)


def historical_match(value: object) -> str:
    return " ".join(_PARENS.sub("", str(value or "")).strip().casefold().split())


def match_key(row: Mapping[str, object], columns: Mapping[str, str]) -> tuple[str, ...] | None:
    keys = ("rit", "rut", "nombre", "tribunal", "programa")
    if not all(columns.get(key) for key in keys):
        return None
    return tuple(historical_match(row.get(columns[key], "")) for key in keys)


def _compose(prefix_value: str, fragments: list[str]) -> str:
    normalized = []
    for fragment in fragments:
        text = " ".join(str(fragment or "").split()).rstrip(". ")
        if text:
            normalized.append(text + ".")
    return prefix_value + " ".join(normalized) if normalized else ""


def _issue(record: SourceRecord, code: str, message: str) -> Issue:
    return Issue(code, message, record.source)


def _first_name(row: Mapping[str, object], columns: Mapping[str, str]) -> str:
    parts = clean_name(row.get(columns.get("nombre", ""), "")).split()
    return parts[0] if parts else ""


def _adult_date(birth: date) -> date:
    try:
        return birth.replace(year=birth.year + 18)
    except ValueError:
        return birth.replace(year=birth.year + 18, day=28)


def _age(birth: date, today: date) -> int:
    return today.year - birth.year - ((today.month, today.day) < (birth.month, birth.day))


def _curator_and_heard(
    record: SourceRecord,
    columns: Mapping[str, str],
    catalog: dict,
    today: date,
    excel_epoch: str,
) -> tuple[list[str], list[str]]:
    row, fragments, ids = record.values, [], []
    curator_column = columns.get("curador")
    if curator_column and not (
        _RUT.search(str(row.get(curator_column, "") or ""))
        or "Institución:" in str(row.get(curator_column, "") or "")
    ):
        fragments.append(render(catalog, "COMUN", "CURADOR"))
        ids.append("T-01")
    heard_column = columns.get("oido")
    heard = as_date(row.get(heard_column), excel_epoch) if heard_column else None
    if heard and 0 <= (today - heard).days <= 45:
        fragments.append(render(catalog, "COMUN", "OIDO", FECHA_OIDO=format_date(heard)))
        ids.append("T-02")
    return fragments, ids


def _audience(
    record: SourceRecord,
    columns: Mapping[str, str],
    catalog: dict,
    today: date,
    excel_epoch: str,
) -> tuple[list[str], list[str]]:
    column = columns.get("prox_aud")
    hearing = as_date(record.values.get(column), excel_epoch) if column else None
    if hearing and hearing >= today:
        return [render(catalog, "COMUN", "PROX_AUDIENCIA", FECHA_AUDIENCIA=format_date(hearing))], ["T-03"]
    return [], []


def evaluate_waiting(
    record: SourceRecord,
    columns: Mapping[str, str],
    catalog: dict,
    today: date,
    excel_epoch: str = "1900",
) -> tuple[str, tuple[str, ...], tuple[Issue, ...]]:
    row, fragments, ids, issues = record.values, [], [], []
    program = row.get(columns.get("programa", ""), "")
    if no_follow_up(program):
        return _compose(prefix(row, columns), [render(catalog, "COMUN", "NO_SEGUIMIENTO", PROGRAMA=program_title(program))]), ("E-01",), ()
    birth = as_date(row.get(columns.get("nacimiento", "")), excel_epoch) if columns.get("nacimiento") else None
    first = _first_name(row, columns)
    if birth and _age(birth, today) >= 18:
        fragments.append(render(catalog, "COMUN", "MAYORIA_EDAD", PNOMBRE=first, FECHA_MAYORIA=format_date(_adult_date(birth))))
        ids.append("E-02")
        extra, extra_ids = _curator_and_heard(record, columns, catalog, today, excel_epoch)
        fragments += extra
        ids += extra_ids
        audience, audience_ids = _audience(record, columns, catalog, today, excel_epoch)
        fragments += audience
        ids += audience_ids
        return _compose(prefix(row, columns), fragments), tuple(ids), tuple(issues)
    if birth and 1 <= (_adult_date(birth) - today).days <= 60:
        fragments.append(render(catalog, "COMUN", "PROXIMA_MAYORIA", PNOMBRE=first, FECHA_MAYORIA=format_date(_adult_date(birth))))
        ids.append("E-03")
    principal = False
    resolution = as_date(row.get(columns.get("resolucion", "")), excel_epoch) if columns.get("resolucion") else None
    if resolution and 0 <= (today - resolution).days <= 29:
        fragments.append(render(catalog, "ESPERA", "E04_RESOLUCION_RECIENTE", PROGRAMA=program_title(program), FECHA_RESOLUCION=format_date(resolution)))
        ids.append("E-04")
        principal = True
    wait_column = columns.get("espera")
    wait = as_int(row.get(wait_column)) if wait_column else None
    if wait_column and (wait is None or wait < 0):
        issues.append(_issue(record, "E-05/E-06", "Tiempo de espera ausente o inválido (G-05)."))
    court = tribunal(row.get(columns.get("tribunal", ""))) if columns.get("tribunal") else None
    if wait is not None and wait >= 30:
        if is_dce(program):
            fragments.append(render(catalog, "ESPERA", "E05_SOLO_CORREO"))
            ids.append("E-05")
            principal = True
        elif court in {"LAJA", "MULCHEN"}:
            fragments.append(render(catalog, "ESPERA", "E05_PROYECTO_Y_CORREO"))
            ids.append("E-05")
            principal = True
        elif court == "TOME":
            fragments.append(render(catalog, "ESPERA", "E05_PROYECTO_Y_CORREO" if wait >= 60 else "E05_SOLO_CORREO"))
            ids.append("E-05")
            principal = True
        else:
            issues.append(_issue(record, "G-06", "Tribunal no reconocido; regla E-05 omitida."))
    if not principal and not issues:
        fragments.append(render(catalog, "ESPERA", "E06_SIN_RESOLUCION", PROGRAMA=program_title(program)))
        ids.append("E-06")
    extra, extra_ids = _curator_and_heard(record, columns, catalog, today, excel_epoch)
    fragments += extra
    ids += extra_ids
    audience, audience_ids = _audience(record, columns, catalog, today, excel_epoch)
    fragments += audience
    ids += audience_ids
    return _compose(prefix(row, columns), fragments), tuple(ids), tuple(issues)


def evaluate_reports(
    record: SourceRecord,
    columns: Mapping[str, str],
    catalog: dict,
    today: date,
    excel_epoch: str = "1900",
) -> tuple[str, tuple[str, ...], tuple[Issue, ...]]:
    row = record.values
    program = row.get(columns.get("programa", ""), "")
    if no_follow_up(program):
        return _compose(prefix(row, columns), [render(catalog, "COMUN", "NO_SEGUIMIENTO", PROGRAMA=program_title(program))]), ("E-01",), ()
    due_column = columns.get("vencimiento")
    due = as_date(row.get(due_column), excel_epoch) if due_column else None
    if not due:
        return "", (), (_issue(record, "I-01/I-02", "Sin fecha de vencimiento o formato no interpretable (G-05)."),)
    days = (due - today).days
    if days < 0:
        key, rule_id = ("I01_VENCIDO_DCE", "I-01") if is_dce(program) else ("I01_VENCIDO_GENERAL", "I-01")
    elif days <= 30:
        key, rule_id = ("I02_POR_VENCER_DCE", "I-02") if is_dce(program) else ("I02_POR_VENCER_GENERAL", "I-02")
    else:
        return "", (), ()
    fragments = [render(catalog, "INFORMES", key, PROGRAMA=program_title(program), FECHA_VENCIMIENTO=format_date(due))]
    audience, audience_ids = _audience(record, columns, catalog, today, excel_epoch)
    return _compose(prefix(row, columns), fragments + audience), tuple([rule_id] + audience_ids), ()


def evaluate_compliance(
    record: SourceRecord,
    columns: Mapping[str, str],
    catalog: dict,
    today: date,
    cross_due: date | None = None,
    excel_epoch: str = "1900",
) -> tuple[str, tuple[str, ...], tuple[Issue, ...]]:
    row, fragments, ids, issues = record.values, [], [], []
    program = row.get(columns.get("programa", ""), "")
    if no_follow_up(program):
        return _compose(prefix(row, columns), [render(catalog, "COMUN", "NO_SEGUIMIENTO", PROGRAMA=program_title(program))]), ("E-01",), ()
    court = tribunal(row.get(columns.get("tribunal", ""))) if columns.get("tribunal") else None
    if court is None:
        issues.append(_issue(record, "G-06", "Tribunal no reconocido; la revisión no puede considerarse completa."))

    birth = as_date(row.get(columns.get("nacimiento", "")), excel_epoch) if columns.get("nacimiento") else None
    first = _first_name(row, columns)
    if birth and _age(birth, today) >= 18:
        fragments.append(render(catalog, "COMUN", "MAYORIA_EDAD", PNOMBRE=first, FECHA_MAYORIA=format_date(_adult_date(birth))))
        ids.append("C-01")
        extra, extra_ids = _curator_and_heard(record, columns, catalog, today, excel_epoch)
        fragments += extra
        ids += extra_ids
        audience, audience_ids = _audience(record, columns, catalog, today, excel_epoch)
        fragments += audience
        ids += audience_ids
        return _compose(prefix(row, columns), fragments), tuple(ids), tuple(issues)

    if birth and 1 <= (_adult_date(birth) - today).days <= 60:
        fragments.append(render(catalog, "COMUN", "PROXIMA_MAYORIA", PNOMBRE=first, FECHA_MAYORIA=format_date(_adult_date(birth))))
        ids.append("C-02")

    days_compliance_column = columns.get("dias_cumpl")
    days_compliance = as_int(row.get(days_compliance_column)) if days_compliance_column else None
    if days_compliance_column and days_compliance is None:
        issues.append(_issue(record, "C-03", "Días de cumplimiento ausentes o inválidos (G-05)."))
    entry = as_date(row.get(columns.get("ingreso", "")), excel_epoch) if columns.get("ingreso") else None
    principal = False
    if days_compliance is not None and 0 <= days_compliance <= 30:
        if entry:
            fragments.append(render(catalog, "CUMPLIMIENTO", "C03_INGRESO_RECIENTE", PROGRAMA=program_title(program), FECHA_INGRESO=format_date(entry)))
            ids.append("C-03")
            principal = True
        else:
            issues.append(_issue(record, "C-03", "Sin fecha de ingreso; regla omitida (G-05)."))

    days_exit_column = columns.get("dias_egresar")
    days_exit = as_int(row.get(days_exit_column)) if days_exit_column else None
    if days_exit_column and days_exit is None:
        issues.append(_issue(record, "C-04/C-05", "Días para egresar ausentes o inválidos (G-05)."))
    projected_exit = as_date(row.get(columns.get("egreso_proy", "")), excel_epoch) if columns.get("egreso_proy") else None
    negative_days = (days_exit is not None and days_exit < 0) or (
        days_compliance is not None and days_compliance < 0
    )
    inconsistent_future_exit = bool(projected_exit and projected_exit > today and negative_days)
    if inconsistent_future_exit:
        issues.append(
            _issue(
                record,
                "C-04",
                "Datos contradictorios: los días indican vencimiento, pero el egreso proyectado es futuro.",
            )
        )
    expired = bool(projected_exit and negative_days and not inconsistent_future_exit)
    if expired:
        fragments.append(render(catalog, "CUMPLIMIENTO", "C04_VENCIDA", FECHA_EGRESO_PROYECTADO=format_date(projected_exit)))
        ids.append("C-04")
        principal = True
    elif negative_days and not projected_exit:
        issues.append(_issue(record, "C-04", "Sin egreso proyectado (G-05)."))

    ending = bool(projected_exit and days_exit is not None and 0 <= days_exit <= 45 and not expired)
    if ending:
        key = "C05_VENCE_HOY" if days_exit == 0 else "C05_POR_VENCER"
        fragments.append(render(catalog, "CUMPLIMIENTO", key, FECHA_EGRESO_PROYECTADO=format_date(projected_exit)))
        ids.append("C-05")
        principal = True
    elif days_exit is not None and 0 <= days_exit <= 45 and not projected_exit:
        issues.append(_issue(record, "C-05", "Sin egreso proyectado; regla no evaluable (G-05)."))

    if cross_due and not expired and not ending:
        fragments.append(render(catalog, "CUMPLIMIENTO", "C10_HOJA2", PROGRAMA=program_title(program), FECHA_VENCIMIENTO=format_date(cross_due)))
        ids.append("C-10")
        principal = True

    extra, extra_ids = _curator_and_heard(record, columns, catalog, today, excel_epoch)
    fragments += extra
    ids += extra_ids

    program_normalized = normalize(program)
    individual_column = columns.get("ficha_ind")
    individual = as_date(row.get(individual_column), excel_epoch) if individual_column else None
    if program_normalized.startswith(("rta", "rtt", "res", "rfa", "rva")) and individual_column:
        if not individual:
            fragments.append(render(catalog, "CUMPLIMIENTO", "C07_SIN_FICHA"))
            ids.append("C-07")
        else:
            age = (today - individual).days
            if age > 180:
                fragments.append(render(catalog, "CUMPLIMIENTO", "C07_FICHA_ANTIGUA", FECHA_FICHA_INDIVIDUAL=format_date(individual)))
                ids.append("C-07")
            elif 0 <= age <= 30:
                fragments.append(render(catalog, "CUMPLIMIENTO", "C07_FICHA_RECIENTE", FECHA_FICHA_INDIVIDUAL=format_date(individual)))
                ids.append("C-07")

    fae_column = columns.get("ficha_fae")
    fae = as_date(row.get(fae_column), excel_epoch) if fae_column else None
    tokens = set(re.findall(r"[a-z0-9]+", program_normalized))
    if {"fae", "fas"} & tokens and entry and (today - entry).days > 120:
        if not fae_column:
            issues.append(_issue(record, "C-08", "No existe columna de ficha FAE; regla no evaluable (G-05)."))
        elif not fae:
            fragments.append(render(catalog, "CUMPLIMIENTO", "C08_FICHA_FAE", PNOMBRE=first))
            ids.append("C-08")

    audience, audience_ids = _audience(record, columns, catalog, today, excel_epoch)
    fragments += audience
    ids += audience_ids

    if not principal and not issues:
        if fragments:
            fragments.insert(0, render(catalog, "CUMPLIMIENTO", "C09_BASE_BREVE"))
            ids.insert(0, "C-09")
        else:
            fragments.append(render(catalog, "CUMPLIMIENTO", "C09_SIN_OBSERVACIONES"))
            ids.append("C-09")
    return _compose(prefix(row, columns), fragments), tuple(ids), tuple(issues)
