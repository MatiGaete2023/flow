"""Estado compartido por WP sin conflictos de fusión.

`docs/implementacion/ESTADO.json` pasa a ser una vista generada. Las fuentes son:
  docs/implementacion/estado/global.json      decisiones, conflictos, orquestador, traspasos (solo Orquestador)
  docs/implementacion/estado/WP-XX.json       un archivo por paquete (lo edita el Ejecutor de ese WP)

Uso:
    python tools/estado.py dividir    # una vez: crea las fuentes desde el ESTADO.json actual
    python tools/estado.py agregar    # regenera ESTADO.json desde las fuentes
    python tools/estado.py validar    # código 1 si la vista no coincide o hay incoherencias

Dos PR de WP distintos ya no tocan el mismo archivo, así que pueden ir directo contra la
rama base sin apilarse. La CI corre `validar`.
"""
import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VIEW = ROOT / "docs" / "implementacion" / "ESTADO.json"
SOURCES = ROOT / "docs" / "implementacion" / "estado"
STATES = {"PENDIENTE", "LISTO", "TOMADO", "EN_VERIFICACION", "EN_REVISION", "EN_AUDITORIA", "INTEGRADO",
          "REVERTIDO", "BLOQUEADO", "BLOQUEADO_USUARIO"}
WP_ID = re.compile(r"^WP-\d{2}$")
ORDER = "orden_paquetes"
DEP_TYPES = {"funcional", "archivo"}


def dump(value):
    return json.dumps(value, ensure_ascii=False, indent=2) + "\n"


def split(view=VIEW, sources=SOURCES):
    data = json.loads(view.read_text(encoding="utf-8"))
    sources.mkdir(parents=True, exist_ok=True)
    packages = data.pop("paquetes")
    data[ORDER] = list(packages)
    (sources / "global.json").write_text(dump(data), encoding="utf-8")
    for wp, entry in packages.items():
        (sources / f"{wp}.json").write_text(dump(entry), encoding="utf-8")
    return len(packages)


def aggregate(sources=SOURCES):
    data = json.loads((sources / "global.json").read_text(encoding="utf-8"))
    order = data.pop(ORDER, [])
    found = sorted(p.stem for p in sources.glob("WP-*.json"))
    names = order + [wp for wp in found if wp not in order]
    packages = {}
    for wp in names:
        path = sources / f"{wp}.json"
        if path.is_file():
            packages[wp] = json.loads(path.read_text(encoding="utf-8"))
    result = {}
    for key, value in data.items():
        result[key] = value
        if key == "conflictos_abiertos":
            result["paquetes"] = packages
    result.setdefault("paquetes", packages)
    return result


def dependencies(entry):
    """(wp, tipo) por dependencia. Un texto es `funcional`; {"wp": ..., "tipo": "archivo"} solo choca en archivos (E6)."""
    for dep in entry.get("depende_de", []):
        if isinstance(dep, dict):
            yield dep.get("wp"), dep.get("tipo")
        else:
            yield dep, "funcional"


def problems(state):
    found = []
    packages = state["paquetes"]
    for wp, entry in packages.items():
        if not WP_ID.match(wp):
            found.append(f"{wp}: identificador inválido")
        if entry.get("estado") not in STATES:
            found.append(f"{wp}: estado desconocido {entry.get('estado')!r}")
        for dep, kind in dependencies(entry):
            if kind not in DEP_TYPES:
                found.append(f"{wp}: dependencia {dep} de tipo desconocido {kind!r}")
            if dep == "TODAS_LAS_DEMAS":
                found.append(f"{wp}: dependencia TODAS_LAS_DEMAS; usar una lista explícita")
            elif dep not in packages:
                found.append(f"{wp}: depende de {dep}, que no existe")
        if entry.get("estado") in ("TOMADO", "EN_VERIFICACION", "EN_REVISION", "EN_AUDITORIA"):
            if not entry.get("ejecutor"):
                found.append(f"{wp}: {entry['estado']} sin ejecutor")
            if entry.get("estado") != "TOMADO" and not entry.get("rama"):
                found.append(f"{wp}: {entry['estado']} sin rama")
        if entry.get("estado") in ("EN_REVISION", "EN_AUDITORIA", "INTEGRADO") and not entry.get("verificacion"):
            found.append(f"{wp}: {entry['estado']} sin verificación")
        if entry.get("estado") == "INTEGRADO":
            for dep, kind in dependencies(entry):
                if kind == "funcional" and packages.get(dep, {}).get("estado") != "INTEGRADO":
                    found.append(f"{wp}: INTEGRADO con la dependencia {dep} sin integrar")
    for conflict in state.get("conflictos_abiertos", []):
        if not conflict.get("respuesta") and not conflict.get("dueño"):
            found.append(f"conflicto {conflict.get('id')}: sin dueño ni respuesta")
    return found


def main(argv=None):
    parser = argparse.ArgumentParser(description="Estado compartido por WP.")
    parser.add_argument("accion", choices=("dividir", "agregar", "validar"))
    args = parser.parse_args(argv)
    if args.accion == "dividir":
        print(f"{split()} paquetes en {SOURCES.relative_to(ROOT)}")
        return 0
    state = aggregate()
    if args.accion == "agregar":
        VIEW.write_text(dump(state), encoding="utf-8")
        print(f"{VIEW.relative_to(ROOT)} regenerado ({len(state['paquetes'])} paquetes)")
        return 0
    issues = problems(state)
    if VIEW.read_text(encoding="utf-8") != dump(state):
        issues.insert(0, "ESTADO.json no coincide con las fuentes: ejecuta `python tools/estado.py agregar`")
    for issue in issues:
        print("-", issue)
    print("ESTADO:", "FALLA" if issues else "PASA")
    return 1 if issues else 0


if __name__ == "__main__":
    sys.exit(main())
