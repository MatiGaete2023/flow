"""Falla si el repositorio contiene RUT chilenos con dígito verificador válido.

Uso:
    python tools/check_real_ids.py            # revisa archivos versionados
    python tools/check_real_ids.py ruta1 ruta2 # revisa rutas indicadas (hook pre-commit)
    python tools/check_real_ids.py --stage  # revisa blobs del índice, no el árbol de trabajo
    python tools/check_real_ids.py --historia  # revisa todos los blobs alcanzables (git rev-list --all)

Lista blanca: tools/real_ids_allowlist.txt (un RUT sintético por línea, formato 11111111-1).
Revisa texto plano y el XML interno de .docx/.xlsx/.zip. Código de salida 1 si hay hallazgos.
Acepta UTF-8 con o sin BOM, UTF-16 con BOM y cp1252 estricto como respaldo.
Contenido ilegible, codificación ambigua o revisión incompleta termina con código 2.
"""

import argparse
import codecs
import io
import lzma
import os
import re
import subprocess
import sys
import zipfile
import zlib
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ALLOWLIST = ROOT / "tools" / "real_ids_allowlist.txt"
TEXT_SUFFIXES = {
    ".py",
    ".json",
    ".md",
    ".txt",
    ".csv",
    ".html",
    ".js",
    ".xml",
    ".yml",
    ".yaml",
    ".b64",
    ".toml",
    ".cjs",
}
ZIP_SUFFIXES = {".docx", ".docm", ".dotx", ".dotm", ".xlsx", ".xlsm", ".xltx", ".xltm", ".zip"}
BINARY_SUFFIXES = {".png", ".jpg", ".jpeg", ".gif", ".ico", ".ttf", ".woff", ".woff2"}
MAX_ARCHIVE_DEPTH = 4
MAX_ARCHIVE_BYTES = 64 * 1024 * 1024
MAX_ARCHIVE_MEMBERS = 10_000
PATTERN = re.compile(r"(?<![\d.])(\d{1,2}[. \u00a0]?\d{3}[. \u00a0]?\d{3})\s*[-\u2010-\u2015\u2212]\s*([\dkK])(?![\w])")


def check_digit(number: str) -> str:
    total, factor = 0, 2
    for char in reversed(number):
        total += int(char) * factor
        factor = 2 if factor == 7 else factor + 1
    rest = 11 - total % 11
    return {11: "0", 10: "K"}.get(rest, str(rest))


def is_real_looking(number: str, digit: str) -> bool:
    if int(number) < 1_000_000 or len(set(number)) <= 2:
        return False
    return check_digit(number) == digit.upper()


def load_allowlist(*, stage: bool = False) -> set[str]:
    if stage:
        relative = ALLOWLIST.relative_to(ROOT).as_posix()
        result = subprocess.run(["git", "show", f":{relative}"], cwd=ROOT, capture_output=True)
        if result.returncode:
            return set()
        content = decode_text(result.stdout)
    else:
        if not ALLOWLIST.is_file():
            return set()
        content = ALLOWLIST.read_text(encoding="utf-8")
    return {line.strip().upper() for line in content.splitlines() if line.strip() and not line.startswith("#")}


def tracked_files() -> list[Path]:
    output = subprocess.run(["git", "ls-files", "-z"], cwd=ROOT, capture_output=True, check=True).stdout
    return [ROOT / name for name in output.decode("utf-8", errors="surrogateescape").split("\0") if name]


def decode_text(data: bytes) -> str:
    """Decode known text encodings strictly; ambiguity must not certify a file."""
    try:
        if data.startswith((codecs.BOM_UTF32_LE, codecs.BOM_UTF32_BE)):
            raise ValueError("Codificación no admitida")
        if data.startswith(codecs.BOM_UTF8):
            content = data.decode("utf-8-sig")
        elif data.startswith((codecs.BOM_UTF16_LE, codecs.BOM_UTF16_BE)):
            content = data.decode("utf-16")
        else:
            try:
                content = data.decode("utf-8")
            except UnicodeDecodeError:
                content = data.decode("cp1252")
    except UnicodeError:
        raise ValueError("Codificación ilegible; control incompleto") from None
    if "\x00" in content:
        raise ValueError("Texto con NUL o codificación ambigua; control incompleto")
    return content


def _utf16_without_bom(raw: bytes) -> bool:
    """Texto UTF-16 sin BOM: decodifica limpio y casi todo es imprimible (una imagen no lo es)."""
    if len(raw) < 16 or len(raw) % 2:
        return False
    for codec in ("utf-16-le", "utf-16-be"):
        try:
            text = raw.decode(codec)
        except UnicodeDecodeError:
            continue
        printable = sum(ch.isprintable() or ch in "\r\n\t" for ch in text)
        if printable / len(text) > 0.95:
            return True
    return False


def sniff(path: Path, staged: bytes | None) -> tuple[str, bytes | None]:
    """Sufijo efectivo de un archivo de sufijo desconocido según su contenido (WP-80).

    Una sesión guardada (.bin que es un XLSX) se revisa como ZIP; un script (.ps1, .iss…) como texto; un binario
    sin texto (imagen, fuente) no se revisa.
    """
    raw = staged if staged is not None else path.read_bytes()
    if zipfile.is_zipfile(io.BytesIO(raw)):
        return ".zip", raw
    if raw.startswith(b"\x1f\x8b"):
        raise ValueError("Contenido gzip no inspeccionable; control incompleto")  # E3 WP-94 C6
    if b"\0" in raw and _utf16_without_bom(raw):
        raise ValueError("Texto UTF-16 sin BOM no inspeccionable; control incompleto")
    if raw.startswith((codecs.BOM_UTF16_LE, codecs.BOM_UTF16_BE)) or b"\0" not in raw:
        try:
            decode_text(raw)
        except ValueError:
            # Sufijo desconocido que no es texto: binario, no es un control incompleto.
            return "", None
        return ".txt", raw
    return "", None


def texts(path: Path, staged: bytes | None = None, *, depth=0, budget=None):
    budget = budget if budget is not None else {"bytes": 0, "members": 0}
    if depth == 0:
        # Git paths and archive names are also metadata that can identify a person.
        name = path
        if path.is_absolute():
            name = path.relative_to(ROOT) if path.is_relative_to(ROOT) else path.name
        yield str(path), str(name)
    suffix = path.suffix.lower()
    if suffix not in TEXT_SUFFIXES and suffix not in ZIP_SUFFIXES:
        suffix, staged = sniff(path, staged)
    if suffix in TEXT_SUFFIXES:
        data = staged if staged is not None else path.read_bytes()
        content = decode_text(data)
        yield str(path), content
    elif suffix in ZIP_SUFFIXES:
        if depth >= MAX_ARCHIVE_DEPTH:
            raise ValueError("Profundidad de contenedores excedida; control incompleto")
        try:
            with zipfile.ZipFile(io.BytesIO(staged) if staged is not None else path) as archive:
                for member in archive.infolist():
                    budget["members"] += 1
                    if budget["members"] > MAX_ARCHIVE_MEMBERS:
                        raise ValueError("Demasiados miembros; control incompleto")
                    yield f"{path}!{member.filename}", member.filename
                    if member.is_dir():
                        continue
                    name = member.filename
                    member_suffix = Path(name).suffix.lower()
                    budget["bytes"] += member.file_size
                    if budget["bytes"] > MAX_ARCHIVE_BYTES:
                        raise ValueError("Expansión de contenedores excedida; control incompleto")
                    data = archive.read(member)
                    if member_suffix not in TEXT_SUFFIXES | ZIP_SUFFIXES:
                        # Revisión externa H5: un miembro renombrado (sesión .bin que es un XLSX) se revisa por
                        # su contenido, igual que un archivo suelto de sufijo desconocido.
                        member_suffix, data = sniff(Path(name), data)
                        if not member_suffix:
                            continue
                    if member_suffix in ZIP_SUFFIXES:
                        yield from texts(Path(f"{path}!{name}"), data, depth=depth + 1, budget=budget)
                    else:
                        content = decode_text(data)
                        if name.lower().endswith(".xml"):
                            try:
                                tree = ET.fromstring(data)
                            except (ET.ParseError, LookupError):
                                raise ValueError("XML interno ilegible") from None
                            # Word runs and Excel rich text split a visible identity into
                            # multiple XML tags. Join only within one paragraph/cell.
                            segments = [
                                "".join(node.itertext())
                                for node in tree.iter()
                                if node.tag.rsplit("}", 1)[-1] in {"p", "si", "is"}
                            ]
                            seen = {
                                (re.sub(r"\D", "", match.group(1)), match.group(2).upper())
                                for match in PATTERN.finditer(content)
                            }
                            extra = []
                            for segment in segments:
                                for match in PATTERN.finditer(segment):
                                    identity = (re.sub(r"\D", "", match.group(1)), match.group(2).upper())
                                    if identity not in seen:
                                        extra.append(match.group(0))
                                        seen.add(identity)
                            content += "\n" + "\n".join(extra)
                        yield f"{path}!{name}", content
        except (zipfile.BadZipFile, zlib.error, lzma.LZMAError):
            raise ValueError("Contenedor ZIP ilegible") from None


def redact_label(label):
    return PATTERN.sub(
        lambda match: re.sub(r"\D", "", match.group(1))[:2] + "******-" + match.group(2).upper(),
        label,
    )


def scan(paths: list[Path], allowlist: set[str], *, stage: bool = False) -> list[tuple[str, int, str]]:
    findings = []
    index = {}
    if stage:
        entries = subprocess.run(
            ["git", "ls-files", "--stage", "-z"], cwd=ROOT, capture_output=True, check=True
        ).stdout
        for entry in entries.decode("utf-8", errors="surrogateescape").split("\0"):
            if entry:
                metadata, name = entry.split("\t", 1)
                mode, oid, level = metadata.split()
                index[name] = (mode, oid, level)
    for path in paths:
        staged = None
        if stage:
            relative = path.relative_to(ROOT).as_posix()
            if relative not in index:
                raise ValueError("Ruta ausente del índice; revisión incompleta")
            mode, oid, level = index[relative]
            if mode == "160000":
                raise ValueError("Submódulo en el índice: requiere control separado; revisión incompleta")
            if level != "0":
                raise ValueError("Conflicto en el índice; revisión incompleta")
            staged = subprocess.run(
                ["git", "cat-file", "blob", oid], cwd=ROOT, capture_output=True, check=True
            ).stdout
        elif not path.is_file():
            continue
        for label, content in texts(path, staged):
            for match in PATTERN.finditer(content):
                number = re.sub(r"\D", "", match.group(1))
                normalized = f"{number}-{match.group(2).upper()}"
                if normalized in allowlist or not is_real_looking(number, match.group(2)):
                    continue
                line = content.count("\n", 0, match.start()) + 1
                findings.append((redact_label(label), line, normalized[:2] + "******-" + normalized[-1]))
    return findings


def history_blobs() -> list[tuple[str, str]]:
    """(blob, ruta) de cada blob distinto alcanzable desde cualquier referencia que pueda contener texto."""
    output = subprocess.run(["git", "rev-list", "--all", "--objects"], cwd=ROOT, capture_output=True, check=True).stdout
    seen, result = set(), []
    for line in output.decode("utf-8", errors="surrogateescape").splitlines():
        sha, _, name = line.partition(" ")
        # Como texts(): también sufijos desconocidos (p. ej. sesiones .bin), salvo imágenes y fuentes.
        if not name or sha in seen or Path(name).suffix.lower() in BINARY_SUFFIXES:
            continue
        seen.add(sha)
        result.append((sha, name))
    return result


def scan_history(allowlist: set[str]) -> list[tuple[str, int, str]]:
    findings = []
    for sha, name in history_blobs():
        content = subprocess.run(["git", "cat-file", "-p", sha], cwd=ROOT, capture_output=True, check=True).stdout
        for label, text in texts(Path(name), content):
            for match in PATTERN.finditer(text):
                number = re.sub(r"\D", "", match.group(1))
                normalized = f"{number}-{match.group(2).upper()}"
                if normalized in allowlist or not is_real_looking(number, match.group(2)):
                    continue
                line = text.count("\n", 0, match.start()) + 1
                findings.append(
                    (redact_label(f"{sha[:10]}:{label}"), line, normalized[:2] + "******-" + normalized[-1])
                )
    return findings


def staged_files() -> list[Path]:
    output = subprocess.run(
        ["git", "diff", "--cached", "--name-only", "--diff-filter=ACMRT", "-z"],
        cwd=ROOT,
        capture_output=True,
        check=True,
    ).stdout
    return [ROOT / name for name in output.decode("utf-8", errors="surrogateescape").split("\0") if name]


def excluded(path: Path, prefixes: list[Path]) -> bool:
    return any(path == prefix or prefix in path.parents for prefix in prefixes)


def _main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Detecta RUT con dígito verificador válido.")
    parser.add_argument("rutas", nargs="*")
    parser.add_argument("--excluir", action="append", default=[], metavar="RUTA")
    parser.add_argument("--stage", action="store_true", help="Revisa solo el contenido añadido al índice de Git.")
    parser.add_argument("--historia", action="store_true", help="Revisa todos los blobs del historial (WP-17).")
    args = parser.parse_args(argv)
    if args.historia and (args.stage or args.rutas or args.excluir):
        parser.error("--historia no acepta --stage, rutas ni exclusiones")
    if args.historia:
        findings = scan_history(load_allowlist())
        for label, line, masked in findings:
            print(f"{label}:{line}: RUT con dígito verificador válido ({masked})")
        print(f"{len(findings)} hallazgos en el historial." if findings else "Sin RUT reales en el historial.")
        return 1 if findings else 0
    if args.stage and args.rutas:
        parser.error("--stage no acepta rutas; revisa los archivos del índice.")
    paths = (
        staged_files()
        if args.stage
        else ([Path(arg).resolve() for arg in args.rutas] if args.rutas else tracked_files())
    )
    if not args.stage and args.rutas:
        if any(not path.exists() for path in paths):
            parser.error("Una ruta solicitada no existe")
        paths = [candidate for path in paths for candidate in (path.rglob("*") if path.is_dir() else [path])]
    prefixes = [Path(os.path.abspath(ROOT / prefix)) for prefix in args.excluir]
    paths = [path for path in paths if not excluded(path, prefixes)]
    findings = scan(paths, load_allowlist(stage=args.stage), stage=args.stage)
    for label, line, masked in findings:
        print(f"{label}:{line}: RUT con dígito verificador válido ({masked})")
    if findings:
        print(
            f"{len(findings)} posibles identidades reales. Reemplázalas por datos sintéticos "
            "o agrega el RUT sintético a tools/real_ids_allowlist.txt."
        )
        return 1
    print("Sin RUT reales detectados.")
    return 0


def main(argv: list[str]) -> int:
    try:
        return _main(argv)
    except (OSError, ValueError, UnicodeError, subprocess.SubprocessError, RuntimeError):
        print("No se pudo completar el control de identidades; no se certifica el contenido.", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
