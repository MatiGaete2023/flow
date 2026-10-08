"""P0: renderiza la matriz revisada; verifica IDs y rutas de evidencia.

python tools/flow_baseline_matrix.py
La relación con tests es evidencia técnica; no equivale a aceptación de RUS.
"""
import csv
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
STATES = {"IMPLEMENTADO", "PARCIAL", "NO IMPLEMENTADO", "BLOQUEADO POR PRUEBA REAL"}


def main():
    records = json.loads((ROOT / "docs/evidencia_p0/requisitos.json").read_text(encoding="utf-8"))
    by_id = {item["id"]: item for item in records}
    if len(by_id) != len(records):
        raise SystemExit("IDs repetidos en la matriz")
    specs = ROOT / "docs/plan_20261008"
    identified = {}
    for path in sorted(specs.glob("0[1-4]_*.md")):
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            found = re.match(r"^\|\s*((?:D-T|R-T|INT-)\d{2}|F\d{2})\s*\|", line)
            if found:
                code = found.group(1)
                if code in identified:
                    raise SystemExit("Requisito repetido en fuentes: " + code)
                identified[code] = (path.name, number, line.split("|")[2].strip())
    if set(by_id) != set(identified):
        raise SystemExit("IDs sin revisar: " + str(sorted(set(by_id) ^ set(identified))))
    lines = ["# Matriz funcional P0 por SHA", "",
             "Origen: `251d5c4025749edd4767183ebd3d2baf3bb9baed`.", "",
             "IMPLEMENTADO describe evidencia de código/fixtures, no aprobación institucional.",
             "Los tests relacionados se ejecutaron en suites locales; los casos D/R/INT completos",
             "no se dan por aprobados cuando exigen interfaz institucional, Office o contrato nuevo.",
             "La especificación 02 todavía no es accesible: sus A-T01..A-T20 no se reconstruyen.", "",
             "| ID | Requisito de la fuente | Estado | Código | Pruebas relacionadas | Límite / siguiente acción |",
             "|---|---|---|---|---|---|"]
    for item in records:
        if item["state"] not in STATES:
            raise SystemExit("Estado inválido: " + item["id"])
        for field in ("code", "tests"):
            for path in item[field]:
                if not (ROOT / path).is_file():
                    raise SystemExit("Evidencia inexistente: " + path)
        name, line, title = identified[item["id"]]
        item.update(source=name, source_line=line, requirement=title)
        link = f"[{item['id']}](plan_20261008/{name}#L{line})"
        code = "; ".join(f"[{p}](../{p})" for p in item["code"]) or "—"
        tests = "; ".join(f"[{p}](../{p})" for p in item["tests"]) or "—"
        lines.append(f"| {link} | {title.replace('|', '/')} | {item['state']} | {code} | {tests} | {item['limit']} |")
    (ROOT / "docs/MATRIZ_FUNCIONAL_20261008.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    with (ROOT / "docs/evidencia_p0/matriz_funcional.csv").open("w", newline="", encoding="utf-8") as stream:
        fields = ["id", "requirement", "state", "code", "tests", "limit", "source", "source_line"]
        writer = csv.DictWriter(stream, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows({**item, "code": ";".join(item["code"]), "tests": ";".join(item["tests"])} for item in records)
    print(f"{len(records)} requisitos identificados; IDs, estados y rutas comprobados")


if __name__ == "__main__":
    main()
