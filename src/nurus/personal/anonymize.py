"""Copia anónima de un Excel RUS/SITFA para compartir un caso de prueba.

Uso:
    python -m nurus.personal.anonymize original.xls copia_anonima.xlsx [--desplazar-fechas DIAS]

Reemplaza nombres, RUT, RIT, curadores, domicilios, teléfonos y correos por valores
ficticios estables: el mismo dato original recibe siempre el mismo reemplazo, así se
conservan agrupaciones por causa o hermanos. Los RUT generados tienen dígito verificador
inválido a propósito. Títulos sobre el encabezado, textos libres, nombres de hoja,
encabezados y pies de página reciben los mismos reemplazos. Se retiran comentarios,
hipervínculos, validaciones, imágenes y propiedades del documento. Tribunal, programa,
fechas y números se conservan para que las reglas den el mismo resultado.

Antes de entregar la copia se relee el archivo guardado: si queda un dato original
(nombre completo, RUT, RIT o correo) la copia se borra y se informa el error. Nunca
sobrescribe archivos ni modifica el original.
"""
from __future__ import annotations

import argparse
import html
import re
import sys
import unicodedata
import zipfile
from dataclasses import dataclass, field
from datetime import date, datetime, timedelta
from pathlib import Path

from openpyxl import Workbook, load_workbook
from openpyxl.packaging.core import DocumentProperties
from openpyxl.worksheet.header_footer import HeaderFooter

SENSITIVE = ("nombre", "rut", "rit", "ruc", "direccion", "domicilio", "telefono",
             "fono", "celular", "correo", "email", "mail", "madre", "padre", "litigante",
             "abogado", "apoderado", "tutor", "hermano", "familiar", "responsable")
KEEP = ("nombre centro", "nombre programa", "nombre tribunal", "nombre proyecto", "nombre institucion")
FREE_TEXT = ("observacion", "obs", "glosa", "detalle", "texto", "comentario", "nota", "motivo")
STOPWORDS = {"de", "del", "la", "las", "los", "san", "santa", "y", "e", "van", "von", "da", "dos"}
RIT_RE = re.compile(r"\b([A-Z])-(\d{1,6})-((?:19|20)\d{2})\b", re.IGNORECASE)
RUT_RE = re.compile(r"(?<![\d.])\d{1,2}\.?\d{3}\.?\d{3}-[\dkK](?!\w)")
MAIL_RE = re.compile(r"\b[\w.+-]+@[\w-]+(?:\.[\w-]+)+\b")
IDENTITY_KINDS = ("persona", "rut", "rit", "mail", "telefono", "direccion")
SUFFIXES = {".xls", ".xlsx", ".xlsm"}


@dataclass
class Report:
    target: Path
    changed: int = 0
    sheets: int = 0
    removed: dict[str, int] = field(default_factory=dict)
    warnings: list[str] = field(default_factory=list)


def normalize(value):
    text = unicodedata.normalize("NFKD", str(value or "")).encode("ascii", "ignore").decode()
    return re.sub(r"\s+", " ", re.sub(r"[-().:_/]+", " ", text.lower())).strip()


def check_digit(number):
    total, factor = 0, 2
    for char in reversed(str(number)):
        total += int(char) * factor
        factor = 2 if factor == 7 else factor + 1
    rest = 11 - total % 11
    return {11: "0", 10: "K"}.get(rest, str(rest))


def _rut_key(value):
    return re.sub(r"[.\s]", "", str(value)).upper()


class Pseudonyms:
    def __init__(self):
        self.values = {}
        self.counters = {}

    def get(self, kind, original):
        original = _rut_key(original) if kind == "rut" else str(original).strip()
        key = (kind, original.upper())
        if key not in self.values:
            self.counters[kind] = number = self.counters.get(kind, 0) + 1
            if kind == "rut":
                base = 90_000_000 + number
                self.values[key] = f"{base}-{'1' if check_digit(base) != '1' else '2'}"
            elif kind == "rit":
                match = RIT_RE.search(original)
                letter, year = (match.group(1).upper(), match.group(3)) if match else ("X", "2000")
                self.values[key] = f"{letter}-{number}-{year}"
            elif kind == "mail":
                self.values[key] = f"contacto{number}@example.invalid"
            elif kind == "telefono":
                self.values[key] = f"900000{number:03d}"
            elif kind == "direccion":
                self.values[key] = f"Calle Ficticia {number}"
            else:
                self.values[key] = f"Persona Ficticia {number}"
        return self.values[key]

    def originals(self):
        return [(kind, original) for kind, original in self.values]


def column_kind(header):
    name = normalize(header)
    if not name or any(name.startswith(keep) for keep in KEEP):
        return None
    if name.startswith("rit"):
        return "rit"
    if "curador" in name:
        return "curador"
    if name.startswith("rut") or " rut" in name:
        return "rut"
    if any(word in name for word in ("correo", "email", "mail")):
        return "mail"
    if any(word in name for word in ("telefono", "fono", "celular")):
        return "telefono"
    if any(word in name for word in ("direccion", "domicilio")):
        return "direccion"
    if any(re.search(r"\b" + word, name) for word in SENSITIVE):
        return "persona"
    if any(name.startswith(word) for word in FREE_TEXT):
        return "texto"
    return None


def header_row(sheet, limit=15):
    for row in sheet.iter_rows(min_row=1, max_row=min(limit, sheet.max_row)):
        names = [normalize(cell.value) for cell in row if isinstance(cell.value, str) and cell.value.strip()]
        if len(names) >= 2 and ("rit" in names or "rut" in names or any(n.startswith("nombre") for n in names)):
            return row[0].row
    return None


def _xls_book(source):
    import xlrd
    legacy = xlrd.open_workbook(str(source))
    book = Workbook()
    book.remove(book.active)
    for index in range(legacy.nsheets):
        old = legacy.sheet_by_index(index)
        sheet = book.create_sheet(old.name[:31])
        for r in range(old.nrows):
            for c in range(old.ncols):
                cell = old.cell(r, c)
                value = cell.value
                if cell.ctype == xlrd.XL_CELL_EMPTY or value == "":
                    continue
                if cell.ctype == xlrd.XL_CELL_DATE:
                    value = xlrd.xldate_as_datetime(value, legacy.datemode)
                elif cell.ctype == xlrd.XL_CELL_BOOLEAN:
                    value = bool(value)
                elif cell.ctype == xlrd.XL_CELL_ERROR:
                    continue
                elif isinstance(value, float) and value.is_integer():
                    value = int(value)
                sheet.cell(r + 1, c + 1, value)
    return book


def _load(source):
    if source.suffix.lower() == ".xls":
        return _xls_book(source)
    return load_workbook(source)


def _tokens(full_name, fake):
    parts = fake.split()
    for index, token in enumerate(re.split(r"\s+", full_name.strip())):
        clean = token.strip(",.;:")
        if len(clean) >= 3 and normalize(clean) not in STOPWORDS:
            yield clean, ("Apellido" if index else "Nombre") + parts[-1]


def _same_format(original, fake):
    if "." not in original:
        return fake
    digits, _, dv = fake.partition("-")
    return f"{int(digits):,}".replace(",", ".") + "-" + dv


def fake_rut(pseudonyms, original):
    return _same_format(str(original), pseudonyms.get("rut", original))


def curator_name(text):
    """Nombre de un curador individual: el texto de la celda sin su RUT; vacío si no hay RUT."""
    if not RUT_RE.search(text) or "Institución:" in text:
        return ""
    rest = RUT_RE.sub(" ", text)
    rest = re.sub(r"(?i)\brut\b|[()\[\]:;,/|]", " ", rest)
    return re.sub(r"\s+", " ", rest).strip(" -")


def replace_in_text(text, replacements, pseudonyms, *, tokens=True):
    result = RIT_RE.sub(lambda m: pseudonyms.get("rit", m.group(0)), text)
    result = RUT_RE.sub(lambda m: fake_rut(pseudonyms, m.group(0)), result)
    result = MAIL_RE.sub(lambda m: pseudonyms.get("mail", m.group(0)), result)
    for original, fake in sorted(replacements.items(), key=lambda item: -len(item[0])):
        if not tokens and " " not in original:
            continue
        result = re.sub(r"(?<!\w)" + re.escape(original) + r"(?!\w)", fake, result, flags=re.IGNORECASE)
    return result


def _strip_extras(book, report):
    def count(key, amount=1):
        if amount:
            report.removed[key] = report.removed.get(key, 0) + amount
    for sheet in book.worksheets:
        for row in sheet.iter_rows():
            for cell in row:
                if cell.comment is not None:
                    cell.comment = None
                    count("comentarios")
                if cell.hyperlink is not None:
                    cell.hyperlink = None
                    count("hipervinculos")
        count("validaciones", len(sheet.data_validations.dataValidation))
        sheet.data_validations.dataValidation = []
        count("imagenes", len(getattr(sheet, "_images", [])))
        sheet._images = []
        sheet.HeaderFooter = HeaderFooter()
    count("nombres_definidos", len(book.defined_names))
    for name in list(book.defined_names):
        del book.defined_names[name]
    book.properties = DocumentProperties(creator="anonimizado", lastModifiedBy="anonimizado")
    if hasattr(book, "custom_doc_props"):
        count("propiedades", len(book.custom_doc_props))
        for prop in list(book.custom_doc_props):
            book.custom_doc_props.remove(prop)


def _xml_text(target):
    with zipfile.ZipFile(target) as archive:
        return html.unescape("\n".join(archive.read(name).decode("utf-8", "ignore")
                                       for name in archive.namelist() if name.endswith((".xml", ".rels"))))


def leaks(target, pseudonyms, full_names):
    text = _xml_text(target)
    found = []
    for kind, original in pseudonyms.originals():
        if kind == "persona" and " " not in original:
            continue
        variants = {original}
        if kind == "rut":
            digits, _, dv = original.partition("-")
            if len(digits) >= 7:
                variants.add(f"{int(digits):,}".replace(",", ".") + "-" + dv)
        for variant in variants:
            if len(variant) >= 4 and re.search(r"(?<!\w)" + re.escape(variant) + r"(?!\w)", text, re.IGNORECASE):
                found.append(f"{kind}: {variant[:2]}…")
    for name in full_names:
        if re.search(r"(?<!\w)" + re.escape(name) + r"(?!\w)", text, re.IGNORECASE):
            found.append(f"persona: {name[:2]}…")
    return sorted(set(found))


def anonymize(source, target, shift_days=0):
    source, target = Path(source), Path(target)
    if source.suffix.lower() not in SUFFIXES:
        raise ValueError("El origen debe ser un Excel (.xls, .xlsx o .xlsm).")
    if target.suffix.lower() != ".xlsx":
        raise ValueError("La copia anónima se guarda como .xlsx.")
    if target.exists():
        raise FileExistsError(f"El destino ya existe; elige otro nombre: {target.name}")
    if source.resolve() == target.resolve():
        raise ValueError("El destino debe ser distinto del original.")
    book = _load(source)
    report = Report(target)
    pseudonyms = Pseudonyms()
    replacements, full_names, plans = {}, set(), []
    for sheet in book.worksheets:
        row_number = header_row(sheet)
        kinds = {cell.column: column_kind(cell.value) for cell in sheet[row_number]} if row_number else {}
        plans.append((sheet, row_number or 0, kinds))
        if not row_number:
            continue
        for row in sheet.iter_rows(min_row=row_number + 1):
            for cell in row:
                kind = kinds.get(cell.column)
                if not isinstance(cell.value, str) or kind not in ("persona", "curador"):
                    continue
                full = cell.value.strip() if kind == "persona" else curator_name(cell.value)
                if len(full) >= 3:
                    fake = pseudonyms.get("persona", full)
                    replacements[full] = fake
                    full_names.add(full)
                    for token, short in _tokens(full, fake):
                        replacements.setdefault(token, short)
    for sheet, row_number, kinds in plans:
        for row in sheet.iter_rows():
            for cell in row:
                value = cell.value
                if value is None or value == "":
                    continue
                body = cell.row > row_number
                kind = kinds.get(cell.column) if body else None
                if shift_days and isinstance(value, (datetime, date)):
                    cell.value = value + timedelta(days=shift_days)
                    continue
                if kind == "rut":
                    cell.value = fake_rut(pseudonyms, value)
                    report.changed += 1
                elif kind in IDENTITY_KINDS:
                    cell.value = pseudonyms.get(kind, value)
                    report.changed += 1
                elif isinstance(value, str) and cell.row != row_number:
                    free = kind in ("texto", "curador") or not body or not row_number
                    new_value = replace_in_text(value, replacements, pseudonyms, tokens=free)
                    if new_value != value:
                        cell.value = new_value
                        report.changed += 1
        title = replace_in_text(sheet.title, replacements, pseudonyms)[:31]
        if title != sheet.title and title not in book.sheetnames:
            sheet.title = title
        report.sheets += bool(row_number)
    _strip_extras(book, report)
    book.save(target)
    try:
        found = leaks(target, pseudonyms, full_names)
    except Exception:
        target.unlink(missing_ok=True)
        raise
    if found:
        target.unlink(missing_ok=True)
        raise ValueError("La copia conservaba datos originales y se eliminó: " + ", ".join(found))
    residual = [token for token in replacements if " " not in token
                and re.search(r"(?<!\w)" + re.escape(token) + r"(?!\w)", _xml_text(target), re.IGNORECASE)]
    if residual:
        report.warnings.append(f"{len(residual)} palabra(s) de nombres siguen en columnas no libres "
                               "(p. ej. programa o tribunal). Revisa la copia antes de compartirla.")
    return report


def main(argv=None):
    parser = argparse.ArgumentParser(description="Copia anónima de un Excel RUS/SITFA.")
    parser.add_argument("origen", type=Path)
    parser.add_argument("destino", type=Path)
    parser.add_argument("--desplazar-fechas", type=int, default=0, metavar="DIAS")
    args = parser.parse_args(argv)
    try:
        report = anonymize(args.origen, args.destino, args.desplazar_fechas)
    except (OSError, ValueError) as exc:
        print(exc)
        return 1
    print(f"Copia anónima creada: {report.target} ({report.changed} celdas reemplazadas en {report.sheets} hojas). "
          "Revísala antes de compartirla.")
    for warning in report.warnings:
        print("Advertencia:", warning)
    return 0


if __name__ == "__main__":
    sys.exit(main())
