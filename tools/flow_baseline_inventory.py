"""P0: inventario estático reproducible; no importa la app ni consulta RUS.

Uso: python tools/flow_baseline_inventory.py --output docs/evidencia_p0/inventario
El grafo incluye imports dentro de funciones. Los imports calculados, el
empaquetado y las llamadas entre procesos requieren inspección adicional.
"""
from __future__ import annotations

import argparse
import ast
import csv
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
APPROVED_EXCEPTIONS = {"README.md": "Portada flow; original en docs/historico/README_NURUS_BASELINE.md"}


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def check_baseline(root):
    manifest = json.loads((root / "docs/evidencia_p0/fuentes_baseline.json").read_text(encoding="utf-8"))
    changed = []
    for name, expected in manifest["files_sha256"].items():
        path = root / name
        # También se verifica la copia del README original, cuando existe.
        if name in APPROVED_EXCEPTIONS:
            alternate = root / "docs/historico/README_NURUS_BASELINE.md"
            if alternate.exists():
                path = alternate
        if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            changed.append(name)
    return {"source_sha": manifest["sha"], "files": len(manifest["files_sha256"]),
            "changed": changed, "readme_relocation": APPROVED_EXCEPTIONS,
            "ok": not changed}


def module_name(path):
    parts = list(path.with_suffix("").parts)
    if parts[0] == "src":
        parts = parts[1:]
    if parts[-1] == "__init__":
        parts = parts[:-1]
    return ".".join(parts)


def graph(root):
    files = sorted((root / "src/nurus").rglob("*.py"))
    files += sorted(p for p in (root / "descargador").glob("*.py")
                    if not p.name.startswith(("test_", "verificar_", "prueba_")))
    index = {module_name(p.relative_to(root)): p for p in files}
    nodes = []
    for name, path in sorted(index.items()):
        tree = ast.parse(path.read_text(encoding="utf-8-sig"))
        imports, edges, computed = set(), set(), []
        package = name if path.name == "__init__.py" else name.rpartition(".")[0]
        for node in ast.walk(tree):
            targets = []
            if isinstance(node, ast.Import):
                targets = [a.name for a in node.names]
            elif isinstance(node, ast.ImportFrom):
                base = node.module or ""
                if node.level:
                    levels = package.split(".")
                    base = ".".join(levels[:len(levels) - node.level + 1] + ([base] if base else []))
                targets = [base] + [base + "." + a.name for a in node.names]
            elif isinstance(node, ast.Call) and (
                isinstance(node.func, ast.Name) and node.func.id == "__import__"
                or isinstance(node.func, ast.Attribute) and node.func.attr == "import_module"
            ):
                computed.append(node.lineno)
            for target in targets:
                if not target:
                    continue
                imports.add(target)
                candidates = [target]
                if name.startswith("descargador."):
                    candidates.append("descargador." + target)
                for candidate in candidates:
                    if candidate in index and candidate != name:
                        edges.add(candidate)
        relative = path.relative_to(root).as_posix()
        action = "CONSERVAR"
        phase = "P0"
        if any(part in relative for part in ("download_link", "download_transfer")):
            action, phase = "DESACOPLAR TRAS G3/G5; conservar recuperación", "P6"
        elif name in {"nurus.personal.bitacoras", "nurus.personal.bitacora_lote", "nurus.personal.bitacora_har"}:
            action, phase = "SEPARAR exportación/análisis sin cambiar reglas", "P3"
        elif name == "nurus.bitacora_html" or name == "descargador.csmp_shared":
            action, phase = "CONSERVAR parser común; comprobar paquete autónomo", "P3"
        elif name == "descargador.resultados":
            action, phase = "ADAPTAR con exportador v1 aditivo después de G2", "P4"
        elif name in {"nurus.personal.app_base", "nurus.personal.results_view"}:
            action, phase = "ADAPTAR entrada local después de G3/G5", "P5/P6"
        elif name == "nurus.personal.work":
            action, phase = "AMPLIAR por adaptador; conservar reglas/ediciones; aislar marcas históricas", "P5/P6/P7"
        elif name == "nurus.personal.sitfa":
            action, phase = "CONSERVAR Excel/procedencia; ADAPTAR consumo local v1", "P5"
        elif name in {"nurus.personal.registration"}:
            action, phase = "AISLAR infraestructura de escritura; preservar recuperación de comprobantes", "P6"
        elif name in {"nurus.personal.session", "nurus.personal.product_state", "nurus.personal.sync"}:
            action, phase = "CONSERVAR sesiones/ediciones/huellas; ADAPTAR importación transaccional", "P5/P8"
        nodes.append({"module": name, "path": relative, "imports": sorted(imports),
                      "local_imports": sorted(edges), "dynamic_import_lines": computed,
                      "action": action, "phase": phase})
    by_name = {n["module"]: n for n in nodes}
    incoming = {name: [] for name in by_name}
    for node in nodes:
        for target in node["local_imports"]:
            incoming[target].append(node["module"])
    for node in nodes:
        node["imported_by"] = incoming[node["module"]]
        reached, pending = set(), list(node["local_imports"])
        while pending:
            target = pending.pop()
            if target in reached:
                continue
            reached.add(target)
            pending.extend(by_name[target]["local_imports"])
        node["transitive_imports"] = sorted(reached - {node["module"]})
    return nodes


def test_inventory(root):
    result = []
    files = sorted((root / "tests").glob("*.py")) + sorted((root / "descargador").glob("test_*.py"))
    for path in files:
        tree = ast.parse(path.read_text(encoding="utf-8-sig"))
        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and node.name.startswith("test_"):
                result.append({"path": path.relative_to(root).as_posix(), "line": node.lineno,
                               "test": node.name})
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--output", type=Path, default=ROOT / "docs/evidencia_p0/inventario")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    baseline = check_baseline(args.root)
    write_json(args.output / "integridad_baseline.json", baseline)
    modules = graph(args.root)
    tests = test_inventory(args.root)
    write_json(args.output / "grafo_imports.json", modules)
    with (args.output / "tests.csv").open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=["path", "line", "test"], lineterminator="\n")
        writer.writeheader()
        writer.writerows(tests)
    lines = ["# Inventario estático P0", "", f"Código de origen: `{baseline['source_sha']}`.", "",
             "Incluye imports en funciones y grafo transitivo. No prueba disponibilidad en Windows,",
             "imports calculados ni invocaciones por subprocess. Las acciones son propuestas para fases posteriores.", "",
             "| Módulo | Acción propuesta | Fase | Imports locales directos | Importado por |",
             "|---|---|---|---|---|"]
    for node in modules:
        link = f"[{node['module']}](../../../{node['path']})"
        lines.append(f"| {link} | {node['action']} | {node['phase']} | {len(node['local_imports'])} | {len(node['imported_by'])} |")
    (args.output / "modulos.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(json.dumps({"baseline_ok": baseline["ok"], "modules": len(modules),
                      "test_functions": len(tests)}, ensure_ascii=False))
    if not baseline["ok"]:
        raise SystemExit("Hay diferencias con el origen: " + ", ".join(baseline["changed"]))


if __name__ == "__main__":
    main()
