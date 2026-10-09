"""Lista los módulos de src/nurus que la app no importa (directa ni indirectamente).

Uso:
    python tools/modulos_no_alcanzables.py            # informe legible
    python tools/modulos_no_alcanzables.py --json     # salida JSON

Raíces: nurus.personal.app (Asistente) y, además, los módulos nurus.* importados
por descargador/ y tools/. Las importaciones bajo `if TYPE_CHECKING:` no cuentan.
Los paquetes (__init__) y los módulos raíz no se informan.
"""
import ast
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "src"


def module_map():
    mods = {}
    for path in SRC.rglob("*.py"):
        if "__pycache__" in path.parts:
            continue
        name = ".".join(path.relative_to(SRC).with_suffix("").parts)
        if name.endswith(".__init__"):
            name = name[: -len(".__init__")]
        mods[name] = path
    return mods


def is_type_checking(node):
    test = node.test
    return (isinstance(test, ast.Name) and test.id == "TYPE_CHECKING") or (
        isinstance(test, ast.Attribute) and test.attr == "TYPE_CHECKING")


def imports_of(path, module, mods):
    tree = ast.parse(path.read_text(encoding="utf-8"))
    skip = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.If) and is_type_checking(node):
            for inner in ast.walk(node):
                skip.add(id(inner))
    package = module if path.name == "__init__.py" else module.rpartition(".")[0]
    found = set()
    for node in ast.walk(tree):
        if id(node) in skip:
            continue
        if isinstance(node, ast.ImportFrom):
            base = node.module or ""
            if node.level:
                parts = package.split(".")
                parts = parts[: len(parts) - (node.level - 1)] if node.level > 1 else parts
                base = ".".join(parts + ([base] if base else []))
            for alias in node.names:
                for candidate in (f"{base}.{alias.name}", base):
                    if candidate in mods:
                        found.add(candidate)
                        break
        elif isinstance(node, ast.Import):
            for alias in node.names:
                if alias.name in mods:
                    found.add(alias.name)
    for name in list(found):
        parts = name.split(".")
        for i in range(1, len(parts)):
            parent = ".".join(parts[:i])
            if parent in mods:
                found.add(parent)
    return found


def external_roots(mods):
    roots = set()
    for folder in (ROOT / "descargador", ROOT / "tools"):
        if not folder.is_dir():
            continue
        for path in folder.rglob("*.py"):
            if "__pycache__" in path.parts or path.name.startswith("test_"):
                continue
            try:
                tree = ast.parse(path.read_text(encoding="utf-8"))
            except (SyntaxError, UnicodeDecodeError):
                continue
            for node in ast.walk(tree):
                if isinstance(node, ast.ImportFrom) and node.module and node.module.startswith("nurus"):
                    for alias in node.names:
                        name = f"{node.module}.{alias.name}"
                        roots.add(name if name in mods else node.module)
                elif isinstance(node, ast.Import):
                    roots.update(a.name for a in node.names if a.name.startswith("nurus"))
    return {r for r in roots if r in mods}


def reachable(starts, mods):
    seen, stack = set(), list(starts)
    while stack:
        name = stack.pop()
        if name in seen or name not in mods:
            continue
        seen.add(name)
        stack.extend(imports_of(mods[name], name, mods))
    return seen


def main(argv):
    mods = module_map()
    app = reachable({"nurus.personal.app"}, mods)
    extra = reachable(external_roots(mods), mods)
    packages = {name for name, path in mods.items() if path.name == "__init__.py"}
    missing = sorted(set(mods) - app - extra - packages - {"nurus"})
    only_tools = sorted((extra - app) - packages)
    result = {"total": len(mods), "alcanzables_app": len(app), "no_alcanzables": missing,
              "solo_herramientas_o_descargador": only_tools}
    if "--json" in argv:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        print(f"Módulos: {result['total']} · alcanzables desde la app: {result['alcanzables_app']}")
        print("No alcanzables:", ", ".join(missing) or "ninguno")
        print("Solo herramientas/Descargador:", ", ".join(only_tools) or "ninguno")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
