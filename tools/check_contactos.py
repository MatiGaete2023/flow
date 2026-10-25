"""Falla si el repositorio contiene correos o RIT que no constan como ficticios.

Complementa tools/check_real_ids.py (RUT) con dos identidades que ese control no ve:

- Correos: se aceptan los dominios reservados para ejemplos (RFC 2606: .example, .test, .invalid, .localhost y
  example.com/.net/.org), los dominios con una etiqueta «example» o «ejemplo» y las direcciones institucionales
  de tools/contactos_permitidos.txt.
- RIT (letra-rol-año): se acepta el año 2099, convención para causas ficticias.

Los correos y RIT presentes el 25 de octubre de 2026 constan como huellas SHA-256 en el mismo archivo para no
bloquear el repositorio actual; eso no los certifica como ficticios. La huella solo evita copiar el valor: el
espacio de los RIT es pequeño y no lo oculta.

Uso:
    python tools/check_contactos.py            # revisa archivos versionados
    python tools/check_contactos.py ruta1 ruta2 # revisa rutas indicadas
    python tools/check_contactos.py --stage  # revisa blobs del índice (hook pre-commit)

Código de salida 1 si hay hallazgos y 2 si la revisión no se completa. Nunca imprime el valor encontrado.
"""

import argparse
import hashlib
import re
import subprocess
import sys
from pathlib import Path

import check_real_ids as ids  # mismo directorio: lectura de texto, Office y ZIP

ROOT = ids.ROOT
ALLOWLIST = ROOT / "tools" / "contactos_permitidos.txt"
_LABEL = r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?"
EMAIL = re.compile(rf"([A-Za-z0-9._%+-]+)@((?:{_LABEL}\.)+[A-Za-z]{{2,24}})(?![A-Za-z0-9])")
RIT = re.compile(r"(?<![\w-])([A-Z])\s*-\s*(\d{1,6})\s*-\s*((?:19|20)\d{2})(?![\w])")
RESERVED_TLDS = {"example", "test", "invalid", "localhost"}
RESERVED_DOMAINS = {"example.com", "example.net", "example.org"}
EXAMPLE_LABELS = {"example", "ejemplo"}
SYNTHETIC_YEAR = "2099"


def fingerprint(value: str) -> str:
    return "sha256:" + hashlib.sha256(value.encode("utf-8")).hexdigest()


def load_allowlist(*, stage: bool = False) -> set[str]:
    if stage:
        relative = ALLOWLIST.relative_to(ROOT).as_posix()
        result = subprocess.run(["git", "show", f":{relative}"], cwd=ROOT, capture_output=True, check=False)
        if result.returncode:
            return set()
        content = ids.decode_text(result.stdout)
    else:
        if not ALLOWLIST.is_file():
            return set()
        content = ALLOWLIST.read_text(encoding="utf-8")
    return {line.strip().lower() for line in content.splitlines() if line.strip() and not line.startswith("#")}


def reserved(domain: str) -> bool:
    labels = domain.split(".")
    return labels[-1] in RESERVED_TLDS or domain in RESERVED_DOMAINS or bool(EXAMPLE_LABELS & set(labels))


def identities(content: str):
    """(tipo, forma normalizada, versión enmascarada, posición) de cada correo o RIT no reservado."""
    for match in EMAIL.finditer(content):
        # El local se recorta: «…@dominio.cl-UCC_x@…» deja un prefijo que no es parte de la dirección.
        local, domain = match.group(1).strip("._%+-").lower(), match.group(2).lower()
        if local and not reserved(domain):
            yield "correo", f"{local}@{domain}", f"{local[0]}***@{domain}", match.start()
    for match in RIT.finditer(content):
        letter, number, year = match.group(1), int(match.group(2)), match.group(3)
        if year != SYNTHETIC_YEAR:
            yield "RIT", f"rit:{letter}-{number}-{year}", f"{letter}-***-{year}", match.start()


def accepted(kind: str, value: str, allowlist: set[str]) -> bool:
    return fingerprint(value) in allowlist or (kind == "correo" and value in allowlist)


def staged_blob(path: Path) -> bytes:
    relative = path.relative_to(ROOT).as_posix()
    result = subprocess.run(["git", "show", f":{relative}"], cwd=ROOT, capture_output=True, check=False)
    if result.returncode:
        raise ValueError("Ruta ausente o en conflicto en el índice; revisión incompleta")
    return result.stdout


def scan(paths: list[Path], allowlist: set[str], *, stage: bool = False) -> list[tuple[str, int, str, str]]:
    findings = []
    for path in paths:
        if not stage and not path.is_file():
            continue
        staged = staged_blob(path) if stage else None
        for label, content in ids.texts(path, staged):
            for kind, value, masked, start in identities(content):
                if not accepted(kind, value, allowlist):
                    line = content.count("\n", 0, start) + 1
                    findings.append((ids.redact_label(label), line, kind, masked))
    return findings


def _main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Detecta correos y RIT sin constancia de ficticios.")
    parser.add_argument("rutas", nargs="*")
    parser.add_argument("--stage", action="store_true", help="Revisa solo el contenido añadido al índice de Git.")
    args = parser.parse_args(argv)
    if args.stage and args.rutas:
        parser.error("--stage no acepta rutas; revisa los archivos del índice.")
    if args.stage:
        paths = ids.staged_files()
    elif args.rutas:
        paths = [Path(arg).resolve() for arg in args.rutas]
        if any(not path.exists() for path in paths):
            parser.error("Una ruta solicitada no existe")
        paths = [candidate for path in paths for candidate in (path.rglob("*") if path.is_dir() else [path])]
    else:
        paths = ids.tracked_files()
    findings = scan(paths, load_allowlist(stage=args.stage), stage=args.stage)
    for label, line, kind, masked in findings:
        print(f"{label}:{line}: {kind} sin constancia de ficticio ({masked})")
    if findings:
        print(
            f"{len(findings)} correos o RIT sin constancia de ficticios. Usa un dominio reservado "
            f"(p. ej. example.test) y el año {SYNTHETIC_YEAR} para RIT, o retira el dato."
        )
        return 1
    print("Sin correos ni RIT nuevos.")
    return 0


def main(argv: list[str]) -> int:
    try:
        return _main(argv)
    except (OSError, ValueError, UnicodeError, subprocess.SubprocessError, RuntimeError):
        print("No se pudo completar el control de correos y RIT; no se certifica el contenido.", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
