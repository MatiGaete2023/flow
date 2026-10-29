"""Orquestación: detecta el formato por contenido, anonimiza en dos pasadas y publica sin sobrescribir."""

from __future__ import annotations

import os
import secrets
from dataclasses import dataclass, field
from pathlib import Path

from . import archive, capture, formats, grid, jsonfmt, safezip, textfmt
from .textengine import Options, Session, Report

MAX_FILE = 400 * 1024 * 1024
LABELS = {
    "xlsx": "Excel",
    "xls": "Excel antiguo",
    "xml2003": "Excel XML",
    "csv": "CSV",
    "html": "HTML",
    "json": "JSON",
    "ics": "Calendario",
    "txt": "Texto",
    "docx": "Word",
    "zip": "ZIP",
    "captura": "Paquete de captura",
}
SUPPORTED = ", ".join(sorted({LABELS[k] for k in ("xlsx", "xls", "csv", "html", "json", "ics", "txt", "docx", "zip")}))


class Rejected(Exception):
    """Archivo que no se puede anonimizar con garantías. El mensaje es fijo: nunca incluye datos del archivo."""

    label = "no admitido"

    def __init__(self, message):
        super().__init__(message)
        self.message = message


class Leak(Rejected):
    label = "datos personales no depurables"


@dataclass
class Output:
    data: bytes
    ext: str
    kind: str
    texts: list = field(default_factory=list)
    changed: int = 0


@dataclass
class Item:
    label: str
    kind: str
    ok: bool
    output: str = ""
    detail: str = ""
    changed: int = 0


@dataclass
class Result:
    outputs: list = field(default_factory=list)
    items: list = field(default_factory=list)
    report: Report = field(default_factory=Report)

    @property
    def ok(self):
        return bool(self.items) and all(item.ok for item in self.items)


def _read_sheets(kind, data, session=None):
    if kind == "xlsx" and session is not None and formats.has_formulas(data):
        session.report.warn(
            "El libro tiene fórmulas: se conservan solo los valores que Excel dejó calculados; "
            "las fórmulas sin valor guardado quedan vacías."
        )
    return {"xlsx": formats.read_xlsx, "xls": formats.read_xls, "xml2003": formats.read_xml2003}[kind](data)


def _as_workbook(kind, data, name):
    """SITFA guarda tablas HTML con extensión .xls: se tratan como libro (y salen como .xlsx)."""
    if kind == "html" and Path(name).suffix.lower() in (".xls", ".xlsx", ".xlsm"):
        tables = [content for block, content in formats.read_html(data) if block == "table"]
        if tables:
            return [grid.Sheet(f"Hoja{number}", rows) for number, rows in enumerate(tables, 1)]
    return None


def collect_bytes(data, name, session, depth=0):
    """Primera pasada: nombres, causas e identidades de todos los archivos antes de reemplazar nada."""
    kind = formats.sniff(data, name)
    if kind in ("xlsx", "xls", "xml2003"):
        grid.collect(_read_sheets(kind, data), session)
    elif kind == "csv":
        grid.collect(formats.read_csv(data)[0], session)
    elif kind == "html":
        book = _as_workbook(kind, data, name)
        if book:
            grid.collect(book, session)
            return
        for block, content in formats.read_html(data):
            if block == "table":
                grid.collect([grid.Sheet("Tabla", content)], session)
            else:
                textfmt.seed_line(content, session)
    elif kind == "json":
        jsonfmt.seed_bytes(data, session)
    elif kind == "ics":
        textfmt.seed_ics(data, session)
    elif kind == "txt":
        textfmt.seed_text(data, session)
    elif kind == "docx":
        for text in textfmt.docx_texts(data):
            textfmt.seed_line(text, session)
    elif kind == "zip" and depth < safezip.MAX_DEPTH:
        archive.collect(data, session, collect_bytes, depth)


def _scan(texts, session):
    found, residual = session.leaks(texts)
    if found:
        raise Leak("Quedarían datos personales (" + ", ".join(sorted(found)) + ").")
    if residual:
        session.report.warn(
            f"{residual} celda(s) de programa o tribunal contienen palabras iguales a nombres; "
            "se conservaron porque las lee el motor. Revisa la copia."
        )


def process_bytes(data, name, session, depth=0):
    """Segunda pasada de un archivo (o miembro de ZIP). Devuelve Output ya revisado o lanza Rejected."""
    try:
        return _process(data, name, session, depth)
    except Rejected:
        raise
    except (formats.Unsupported, safezip.UnsafeArchive, capture.BadCapture, jsonfmt.BadJson) as exc:
        raise Rejected(str(exc)) from None
    except Exception:
        raise Rejected("No se pudo leer el archivo: formato dañado o no compatible.") from None


def _process(data, name, session, depth):
    kind = formats.sniff(data, name)
    if kind is None or kind == "pdf":
        raise Rejected(
            "Formato no admitido (no se pueden anonimizar PDF, imágenes ni binarios). Admitidos: " + SUPPORTED
        )
    suffix = Path(name).suffix.lower()
    texts, changed = [], 0
    book = _as_workbook(kind, data, name)
    if kind in ("xlsx", "xls", "xml2003") or book:
        sheets, texts, changed = grid.transform(book or _read_sheets(kind, data, session), session)
        out, ext, kind = formats.write_xlsx(sheets), ".xlsx", "xls" if book else kind
    elif kind == "csv":
        sheets, delimiter, bom = formats.read_csv(data)
        sheets, texts, changed = grid.transform(sheets, session)
        out, ext = formats.write_csv(sheets[0], delimiter, bom), (".tsv" if delimiter == "\t" else ".csv")
    elif kind == "html":
        blocks, number = [], 0
        for block, content in formats.read_html(data):
            if block == "table":
                number += 1
                sheets, table_texts, count = grid.transform([grid.Sheet(f"Tabla{number}", content)], session)
                blocks.append(("table", sheets[0].rows))
                texts += [t for t in table_texts if t[0] != sheets[0].title]
                changed += count
            else:
                new = textfmt.line(content, session)
                blocks.append(("text", new))
                texts.append((new, False))
        out, ext = formats.write_html(blocks), ".html"
    elif kind == "json":
        out, texts = jsonfmt.anonymize_bytes(data, session)
        ext = suffix if suffix in (".json", ".jsonl", ".ndjson") else ".json"
    elif kind == "ics":
        out, texts = textfmt.anonymize_ics(data, session)
        ext = ".ics"
    elif kind == "txt":
        out, texts = textfmt.anonymize_text(data, session)
        ext = suffix if suffix in formats.TEXT_SUFFIXES else ".txt"
    elif kind == "docx":
        out, texts = textfmt.anonymize_docx(data, session)
        ext = ".docx"
    else:
        out, texts, kind = archive.process(
            data, session, lambda raw, member, level: process_bytes(raw, member, session, level), depth
        )
        ext = ".zip"
    _scan(texts, session)
    return Output(out, ext, kind, [], changed)


def _expand(paths):
    """Archivos de entrada: carpetas recursivas (sin ocultos); conserva el orden dado."""
    files = []
    for path in paths:
        path = Path(path)
        if path.is_dir():
            files += sorted(
                p
                for p in path.rglob("*")
                if p.is_file() and not any(s.startswith(".") for s in p.parts[len(path.parts) :])
            )
        else:
            files.append(path)
    return files


def _publish(target, data):
    """Escribe sin sobrescribir nunca: temporal oculto + enlace físico, o creación exclusiva si no hay enlaces."""
    temp = target.with_name(f".anon-{secrets.token_hex(6)}.tmp")
    created = False
    try:
        with open(temp, "xb") as handle:
            handle.write(data)
        try:
            os.link(temp, target)
        except FileExistsError:
            raise
        except OSError:
            with open(target, "xb") as handle:
                created = True
                handle.write(data)
    except BaseException:
        if created:
            target.unlink(missing_ok=True)
        raise
    finally:
        temp.unlink(missing_ok=True)


def run(inputs, options=None, out_dir=None, progress=None):
    """Anonimiza uno o varios archivos/carpetas con una sola sesión (mismos seudónimos entre archivos)."""
    options = options or Options()
    session = Session(options)
    files = _expand(inputs)
    result = Result(report=session.report)
    if not files:
        result.items.append(Item("Entrada", "", False, detail="No hay archivos para procesar."))
        return result
    first = Path(inputs[0])
    base = Path(out_dir) if out_dir else (first if first.is_dir() else first.parent)
    folder_run = len(files) > 1 or any(Path(p).is_dir() for p in inputs)
    target_dir = base
    base.mkdir(parents=True, exist_ok=True)
    if folder_run:
        target_dir = base / f"anonimizado_{secrets.token_hex(4)}"
        target_dir.mkdir(parents=True, exist_ok=False)
    loaded = []
    for number, path in enumerate(files, 1):
        label = f"Archivo {number}"
        try:
            if path.stat().st_size > MAX_FILE:
                raise Rejected("El archivo supera el tamaño máximo admitido (400 MB).")
            loaded.append((label, path))
        except Rejected as exc:
            result.items.append(Item(label, "", False, detail=exc.message))
        except OSError:
            result.items.append(Item(label, "", False, detail="No se pudo leer el archivo."))
    total = len(loaded)
    for step, (label, path) in enumerate(loaded):
        if progress:
            progress(label, step, total * 2)
        try:
            collect_bytes(path.read_bytes(), path.name, session)
        except Exception:
            continue
    for step, (label, path) in enumerate(loaded):
        if progress:
            progress(label, total + step, total * 2)
        data = None
        try:
            data = path.read_bytes()
            output = process_bytes(data, path.name, session)
        except Rejected as exc:
            result.items.append(
                Item(label, formats.sniff(data, path.name) or "" if data else "", False, detail=exc.message)
            )
            continue
        except OSError:
            result.items.append(Item(label, "", False, detail="No se pudo leer el archivo."))
            continue
        finally:
            data = None
        name = f"{'archivo_%02d' % (step + 1) if folder_run else 'Copia_CSMP_anon_' + secrets.token_hex(6)}{output.ext}"
        target = target_dir / name
        try:
            _publish(target, output.data)
        except OSError:
            result.items.append(
                Item(label, output.kind, False, detail="No se pudo escribir la copia en la carpeta de destino.")
            )
            continue
        result.outputs.append(target)
        result.items.append(Item(label, output.kind, True, output=name, changed=output.changed))
    session.report.files = result.items
    if folder_run and result.outputs:
        _publish(target_dir / "informe_anonimizacion.txt", report_text(result, options).encode("utf-8"))
    if progress:
        progress("", total * 2, total * 2)
    return result


def report_text(result, options):
    lines = [
        "INFORME DE ANONIMIZACIÓN (no contiene datos de los archivos originales)",
        f"Modo: {options.mode}" + (f" · fechas desplazadas {options.shift_days} días" if options.shift_days else ""),
        "",
    ]
    for item in result.items:
        state = f"OK -> {item.output}" if item.ok else f"NO SE CREÓ: {item.detail}"
        lines.append(f"- {item.label} ({LABELS.get(item.kind, item.kind or 'desconocido')}): {state}")
    lines += ["", *[f"Advertencia: {w}" for w in warnings(options, result.report)]]
    return "\n".join(lines) + "\n"


def warnings(options, report):
    items = list(report.warnings)
    if options.shift_days:
        items.append("Las fechas se desplazaron: las reglas pueden dar resultados distintos al original.")
    items.append(
        "Revisa manualmente la copia antes de compartirla: los textos libres pueden contener datos no reconocidos."
    )
    items.append("No se conserva formato, fórmulas, macros, imágenes ni metadatos; solo valores.")
    return items
