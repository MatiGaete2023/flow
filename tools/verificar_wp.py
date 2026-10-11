"""Verificación estándar de un paquete de trabajo (WP) contra la línea base.

Uso:
    python tools/verificar_wp.py --linea-base
        Ejecuta todo y guarda docs/implementacion/linea_base.json.
    python tools/verificar_wp.py [--lista-eliminadas ARCHIVO ...] [--salida archivo.json]
        Ejecuta todo y compara con la línea base. Código de salida 1 si algún control FALLA.
        --lista-eliminadas: archivo con un nodeid de pytest por línea (p. ej.
        tests/test_x.py::test_y o descargador/test_z.py::Clase::test_w). Cada nodo debe estar
        ausente ahora; su cantidad reemplaza a los conteos declarados a mano.
    --pruebas-eliminadas-asistente/-descargador N siguen aceptados por compatibilidad, pero
        el control queda marcado «sin lista verificable».

Controles:
  1. Suite del Asistente (pytest en tests/): 0 fallos; aprobadas >= base - eliminadas declaradas.
  2. Suite del Descargador (pytest en descargador/): igual criterio.
  3. Identidades reales (tools/check_real_ids.py): nunca más que la base; si la base es 0, debe ser 0.
  4. Paridad de 17 productos golden (tools/flow_baseline_golden.py): debe decir «PARIDAD PASÓ».
  5. Restricciones absolutas: sin .Send( en src/, CC fijo en policy.py, submit de registro deshabilitado,
     extensión sin acceso a document.cookie.
  6. Ruff E9,F63,F7,F82 en src/: nunca más avisos que la base.
  7. Módulos no alcanzables: nunca más que la base.
  8. Enlaces Markdown rotos: nunca más que la base.
Un control que no se puede medir (herramienta ausente o salida ilegible) da FALLA, no se omite.
En Linux sin pantalla usa xvfb-run si está disponible.
"""
import argparse
import ast
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASE_FILE = ROOT / "docs" / "implementacion" / "linea_base.json"
PY = sys.executable


def run(command, cwd=ROOT, timeout=1800):
    prefix = []
    if sys.platform.startswith("linux") and not os.environ.get("DISPLAY") and shutil.which("xvfb-run"):
        prefix = ["xvfb-run", "-a"]
    result = subprocess.run(prefix + command, cwd=cwd, capture_output=True, text=True, timeout=timeout)
    return result.returncode, result.stdout + result.stderr


def pytest_counts(args):
    code, out = run([PY, "-m", "pytest", "-q", "-p", "no:cacheprovider", *args])
    tail = out.strip().splitlines()[-1] if out.strip() else ""
    def number(word):
        match = re.search(r"(\d+) " + word, tail)
        return int(match.group(1)) if match else 0
    failed = number("failed") + number("error")
    if code not in (0, 1) and not failed:
        # Interrupción, error de recolección o caída al cerrar: no se cuenta como suite sana.
        failed, tail = 1, f"{tail} (pytest terminó con código {code})"
    return {"aprobadas": number("passed"), "omitidas": number("skipped"), "fallidas": failed,
            "subpruebas": number("subtests passed"), "codigo": code, "resumen": tail}


def count_real_ids():
    script = ROOT / "tools" / "check_real_ids.py"
    if not script.exists():
        return None
    code, out = run([PY, str(script)])
    found = sum(1 for line in out.splitlines() if "dígito verificador válido" in line)
    if code != 0 and not found:
        return None  # el script falló sin informar hallazgos: no se midió
    return found


def golden():
    script = ROOT / "tools" / "flow_baseline_golden.py"
    expected = ROOT / "docs" / "evidencia_p0" / "golden" / "semantic.json"
    if not script.exists() or not expected.exists():
        return "NO_DISPONIBLE"
    with tempfile.TemporaryDirectory() as temp:
        code, out = run([PY, str(script), "--output", str(Path(temp) / "golden"), "--expected", str(expected)])
    return "PASA" if code == 0 and "PARIDAD PASÓ" in out else "FALLA"


def restrictions():
    problems = []
    for path in (ROOT / "src").rglob("*.py"):
        if re.search(r"\.Send\(", path.read_text(encoding="utf-8", errors="ignore")):
            problems.append(f"{path.relative_to(ROOT)} usa .Send(")
    policy = ROOT / "src" / "nurus" / "services" / "policy.py"
    if not policy.exists() or 'MANDATORY_CC = "ucc_concepcion@pjud.cl"' not in policy.read_text(encoding="utf-8"):
        problems.append("policy.py no define MANDATORY_CC = ucc_concepcion@pjud.cl")
    registration = ROOT / "src" / "nurus" / "personal" / "registration.py"
    # Sin registration.py (retirado en WP-22) no queda forma de escribir en RUS. Si vuelve, submit debe lanzar error.
    if registration.exists():
        tree = ast.parse(registration.read_text(encoding="utf-8"))
        submits = [n for n in ast.walk(tree) if isinstance(n, ast.FunctionDef) and n.name == "submit"]
        if not submits:
            problems.append("registration.py existe pero no se encontró submit: no se pudo medir")
        for submit in submits:
            body = [n for n in submit.body if not (isinstance(n, ast.Expr) and isinstance(getattr(n, "value", None), ast.Constant))]
            if not (body and isinstance(body[0], ast.Raise)):
                problems.append("registration.submit ya no lanza error al inicio")
    for path in (ROOT / "descargador" / "extension").glob("*.js"):
        if re.search(r"document\.cookie\s*[=;.)\[]", path.read_text(encoding="utf-8", errors="ignore")):
            problems.append(f"{path.relative_to(ROOT)} accede a document.cookie")
    return problems


def ruff_count():
    if not shutil.which("ruff") and run([PY, "-m", "ruff", "--version"])[0] != 0:
        return None
    code, out = run([PY, "-m", "ruff", "check", "--select", "E9,F63,F7,F82", "--output-format", "concise", "src"])
    if code > 1:
        return None  # configuración inválida o fallo de ruff: no se midió
    return sum(1 for line in out.splitlines() if re.match(r"^src[\\/].+:\d+:\d+:", line))


def unreachable_count():
    script = ROOT / "tools" / "modulos_no_alcanzables.py"
    if not script.exists():
        return None
    code, out = run([PY, str(script), "--json"])
    try:
        return len(json.loads(out[out.index("{"):])["no_alcanzables"])
    except (ValueError, KeyError):
        return None


def broken_links():
    script = ROOT / "tools" / "check_links.py"
    if not script.exists():
        return None
    code, out = run([PY, str(script)])
    match = re.search(r"(\d+) enlaces rotos", out)
    return int(match.group(1)) if match else None


def collect():
    return {
        "commit": subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip(),
        "asistente": pytest_counts([]),
        "descargador": pytest_counts(["descargador"]),
        "identidades_reales": count_real_ids(),
        "paridad_golden": golden(),
        "restricciones": restrictions(),
        "ruff_src": ruff_count(),
        "modulos_no_alcanzables": unreachable_count(),
        "enlaces_rotos": broken_links(),
    }


def collected_nodeids(suite):
    """Nodeids presentes ahora, relativos a la raíz (las pruebas del Descargador llevan prefijo descargador/)."""
    args = ["--collect-only", "-q", "-p", "no:cacheprovider"] + (["descargador"] if suite == "descargador" else [])
    code, out = run([PY, "-m", "pytest", *args])
    ids = {line.strip() for line in out.splitlines() if "::" in line and not line.startswith(("=", " "))}
    if suite == "descargador":
        ids |= {"descargador/" + i for i in ids if not i.startswith("descargador/")}
    return ids


def existed_in_base(node, commit):
    """El nodo declarado existía en la línea base: su archivo y su función estaban en `commit`."""
    if not commit:
        return False
    path, _, name = node.partition("::")
    code, source = run(["git", "show", f"{commit}:{path}"])
    function = name.split("::")[-1].split("[")[0]
    return code == 0 and re.search(r"def\s+" + re.escape(function) + r"\s*\(", source) is not None


def removed_from_lists(paths, commit=None):
    """Lee las listas de nodos eliminados; devuelve conteos por suite y problemas.

    Problemas: nodos que siguen presentes, nodos repetidos y nodos que no existían en la línea base."""
    removed = {"asistente": [], "descargador": []}
    for path in paths:
        for line in Path(path).read_text(encoding="utf-8").splitlines():
            node = line.split("#", 1)[0].strip()
            if node:
                removed["descargador" if node.startswith("descargador/") else "asistente"].append(node)
    problems = []
    for suite, nodes in removed.items():
        problems += [f"{node} está repetido" for node in sorted({n for n in nodes if nodes.count(n) > 1})]
        if commit:
            problems += [f"{node} no existía en la línea base {commit}" for node in sorted(set(nodes))
                         if not existed_in_base(node, commit)]
        if nodes:
            present = collected_nodeids(suite)
            problems += [f"{node} sigue presente" for node in nodes if node in present]
    return {suite: len(set(nodes)) for suite, nodes in removed.items()}, problems


def compare(now, base, removed_a, removed_d, lists=None):
    checks = []
    def add(name, ok, detail):
        checks.append({"control": name, "resultado": "PASA" if ok else "FALLA", "detalle": detail})
    listed, problems = lists if lists else ({}, [])
    for key, removed in (("asistente", removed_a), ("descargador", removed_d)):
        n, b = now[key], base[key]
        removed += listed.get(key, 0)
        minimum = b["aprobadas"] - removed
        origin = "" if not removed or listed.get(key) else "; eliminadas sin lista verificable"
        add(f"Suite {key}", n["fallidas"] == 0 and n["aprobadas"] >= minimum,
            f"{n['resumen']} (mínimo {minimum} aprobadas; base {b['aprobadas']}{origin})")
    if lists:
        add("Pruebas eliminadas listadas", not problems,
            "; ".join(problems) or f"{sum(listed.values())} nodos ausentes como se declaró")
    ids_now, ids_base = now["identidades_reales"], base["identidades_reales"]
    if ids_now is None:
        add("Identidades reales", False, "NO_MEDIDO: falta tools/check_real_ids.py o su salida no se pudo leer")
    else:
        limit = 0 if not ids_base else ids_base
        add("Identidades reales", ids_now <= limit, f"{ids_now} hallazgos (límite {limit})")
    add("Paridad golden", now["paridad_golden"] == "PASA", now["paridad_golden"])
    add("Restricciones absolutas", not now["restricciones"], "; ".join(now["restricciones"]) or "sin infracciones")
    for key, label in (("ruff_src", "Ruff E9/F63/F7/F82"), ("modulos_no_alcanzables", "Módulos no alcanzables"), ("enlaces_rotos", "Enlaces rotos")):
        if now[key] is None or base.get(key) is None:
            add(label, False, f"NO_MEDIDO (ahora {now[key]}, base {base.get(key)})")
        else:
            add(label, now[key] <= base[key], f"{now[key]} (base {base[key]})")
    return checks


def main(argv):
    parser = argparse.ArgumentParser(description="Verificación estándar de un WP.")
    parser.add_argument("--linea-base", action="store_true")
    parser.add_argument("--pruebas-eliminadas-asistente", type=int, default=0)
    parser.add_argument("--pruebas-eliminadas-descargador", type=int, default=0)
    parser.add_argument("--lista-eliminadas", type=Path, action="append", default=[])
    parser.add_argument("--salida", type=Path)
    args = parser.parse_args(argv)
    now = collect()
    if args.linea_base:
        BASE_FILE.parent.mkdir(parents=True, exist_ok=True)
        BASE_FILE.write_text(json.dumps(now, ensure_ascii=False, indent=2), encoding="utf-8")
        print(json.dumps(now, ensure_ascii=False, indent=2))
        print(f"Línea base guardada en {BASE_FILE.relative_to(ROOT)}")
        return 0
    if not BASE_FILE.exists():
        print("Falta la línea base: ejecuta primero --linea-base en la rama base.")
        return 1
    base = json.loads(BASE_FILE.read_text(encoding="utf-8"))
    lists = removed_from_lists(args.lista_eliminadas, base.get("commit")) if args.lista_eliminadas else None
    checks = compare(now, base, args.pruebas_eliminadas_asistente, args.pruebas_eliminadas_descargador, lists)
    report = {"commit": now["commit"], "base": base["commit"], "controles": checks,
              "veredicto": "PASA" if all(c["resultado"] == "PASA" for c in checks) else "FALLA"}
    for check in checks:
        print(f"[{check['resultado']}] {check['control']}: {check['detalle']}")
    print("VEREDICTO:", report["veredicto"])
    if args.salida:
        args.salida.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    return 0 if report["veredicto"] == "PASA" else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
