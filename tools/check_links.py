"""Informa enlaces relativos rotos en archivos Markdown versionados.

Uso:
    python tools/check_links.py                 # todos, salvo docs/historico/
    python tools/check_links.py --incluir-historico

Código de salida 1 si hay enlaces rotos. Ignora http(s), mailto y anclas internas.
"""
import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent
LINK = re.compile(r"\]\(([^)\s]+?)(?:#[^)]*)?\)")


def markdown_files():
    output = subprocess.run(["git", "ls-files", "-z", "*.md"], cwd=ROOT, capture_output=True, check=True).stdout
    return [ROOT / name for name in output.decode("utf-8").split("\0") if name]


def main(argv):
    include_history = "--incluir-historico" in argv
    broken = []
    for path in markdown_files():
        relative = path.relative_to(ROOT).as_posix()
        if not include_history and relative.startswith("docs/historico/"):
            continue
        text = path.read_text(encoding="utf-8", errors="ignore")
        for match in LINK.finditer(text):
            target = unquote(match.group(1).strip("<>"))
            if not target or target.startswith(("http://", "https://", "mailto:", "#")):
                continue
            if not (path.parent / target).exists():
                line = text.count("\n", 0, match.start()) + 1
                broken.append(f"{relative}:{line}: {target}")
    for item in broken:
        print(item)
    print(f"{len(broken)} enlaces rotos.")
    return 1 if broken else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
