"""Crea una copia anónima de un Excel RUS/SITFA para compartir un caso de prueba.

Uso: botón «Exportar caso de prueba anónimo» (Configuración) o
    python tools/anonimizar_excel.py original.xlsx copia_anonima.xlsx [--desplazar-fechas DIAS]

Reemplaza nombres, RUT, RIT, curadores, domicilios, teléfonos y correos por valores
ficticios estables: el mismo dato original recibe siempre el mismo reemplazo, así
se conservan agrupaciones por causa o hermanos. Los RUT generados tienen dígito
verificador inválido a propósito. Los textos libres (OBSERVACION y similares)
reciben los mismos reemplazos. Tribunal, programa, fechas y números se conservan
para que las reglas den el mismo resultado; --desplazar-fechas mueve todas las
fechas la misma cantidad de días (usa entonces la fecha de corte desplazada).
Nunca sobrescribe archivos. Revisa la copia antes de compartirla.
"""
import re
import unicodedata
from datetime import date, datetime, timedelta
from pathlib import Path

from openpyxl import load_workbook

SENSITIVE = ("nombre", "rut", "rit", "ruc", "curador", "direccion", "domicilio", "telefono",
             "fono", "celular", "correo", "email", "mail", "madre", "padre", "litigante",
             "abogado", "apoderado", "tutor")
KEEP = ("nombre centro", "nombre programa", "nombre tribunal")
FREE_TEXT = ("observacion", "obs", "glosa", "detalle", "texto", "comentario")
RIT_RE = re.compile(r"\b([A-Z])-(\d{1,6})-((?:19|20)\d{2})\b")
RUT_RE = re.compile(r"\b\d{1,2}\.?\d{3}\.?\d{3}-[\dkK]\b")
MAIL_RE = re.compile(r"\b[\w.+-]+@[\w-]+(?:\.[\w-]+)+\b")


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


class Pseudonyms:
    def __init__(self):
        self.values = {}
        self.counters = {}

    def _next(self, kind):
        self.counters[kind] = self.counters.get(kind, 0) + 1
        return self.counters[kind]

    def get(self, kind, original):
        key = (kind, str(original).strip().upper())
        if key not in self.values:
            number = self._next(kind)
            if kind == "rut":
                base = 90_000_000 + number
                valid = check_digit(base)
                invalid = "1" if valid != "1" else "2"
                self.values[key] = f"{base}-{invalid}"
            elif kind == "rit":
                match = RIT_RE.search(str(original).upper())
                letter, year = (match.group(1), match.group(3)) if match else ("X", "2000")
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


def column_kind(header):
    name = normalize(header)
    if not name or any(name.startswith(keep) for keep in KEEP):
        return None
    if name.startswith("rit"):
        return "rit"
    if name.startswith("rut") or " rut" in name:
        return "rut"
    if any(word in name for word in ("correo", "email", "mail")):
        return "mail"
    if any(word in name for word in ("telefono", "fono", "celular")):
        return "telefono"
    if any(word in name for word in ("direccion", "domicilio")):
        return "direccion"
    if any(word in name for word in SENSITIVE):
        return "persona"
    if any(name.startswith(word) for word in FREE_TEXT):
        return "texto"
    return None


def header_row(sheet, limit=15):
    for row in sheet.iter_rows(min_row=1, max_row=min(limit, sheet.max_row)):
        names = {normalize(cell.value) for cell in row}
        if "rit" in names or any(name.startswith("nombre") for name in names):
            return row[0].row
    return None


def replace_in_text(text, replacements, pseudonyms):
    result = RIT_RE.sub(lambda m: pseudonyms.get("rit", m.group(0)), text)
    result = RUT_RE.sub(lambda m: pseudonyms.get("rut", m.group(0).replace(".", "")), result)
    result = MAIL_RE.sub(lambda m: pseudonyms.get("mail", m.group(0)), result)
    for original, fake in sorted(replacements.items(), key=lambda item: -len(item[0])):
        result = re.sub(r"\b" + re.escape(original) + r"\b", fake, result, flags=re.IGNORECASE)
    return result


def anonymize(source, target, shift_days=0):
    book = load_workbook(source)
    pseudonyms = Pseudonyms()
    name_tokens = {}
    plans = []
    for sheet in book.worksheets:
        row_number = header_row(sheet)
        if row_number is None:
            continue
        kinds = {cell.column: column_kind(cell.value) for cell in sheet[row_number]}
        plans.append((sheet, row_number, kinds))
        for row in sheet.iter_rows(min_row=row_number + 1):
            for cell in row:
                kind = kinds.get(cell.column)
                if kind == "persona" and isinstance(cell.value, str) and cell.value.strip():
                    fake = pseudonyms.get("persona", cell.value)
                    name_tokens[cell.value.strip()] = fake
                    for index, token in enumerate(cell.value.split()):
                        if len(token) >= 3:
                            name_tokens.setdefault(token, fake.split()[-1] if index else "Nombre" + fake.split()[-1])
    changed = 0
    for sheet, row_number, kinds in plans:
        for row in sheet.iter_rows(min_row=row_number + 1):
            for cell in row:
                kind = kinds.get(cell.column)
                value = cell.value
                if value is None or value == "":
                    continue
                if shift_days and isinstance(value, (datetime, date)):
                    cell.value = value + timedelta(days=shift_days)
                    continue
                if kind in ("persona", "rut", "rit", "mail", "telefono", "direccion"):
                    original = str(value).replace(".", "") if kind == "rut" else value
                    cell.value = pseudonyms.get(kind, original)
                    changed += 1
                elif isinstance(value, str) and (kind == "texto" or RUT_RE.search(value) or RIT_RE.search(value)):
                    new_value = replace_in_text(value, name_tokens, pseudonyms)
                    if new_value != value:
                        cell.value = new_value
                        changed += 1
    for sheet in book.worksheets:
        if sheet not in [plan[0] for plan in plans]:
            for row in sheet.iter_rows():
                for cell in row:
                    if isinstance(cell.value, str):
                        cell.value = replace_in_text(cell.value, name_tokens, pseudonyms)
    book.properties.creator = "anonimizado"
    book.properties.lastModifiedBy = "anonimizado"
    book.properties.title = None
    book.save(target)
    return changed, len(plans)


def export_copy(source, target, shift_days=0):
    """Nunca sobrescribe: el destino debe ser nuevo y distinto del original."""
    source, target = Path(source), Path(target)
    if target.exists():
        raise ValueError("El destino ya existe; elige otro nombre.")
    if source.resolve() == target.resolve():
        raise ValueError("El destino debe ser distinto del original.")
    return anonymize(source, target, shift_days)
