"""Verificación estándar de un paquete de trabajo (WP) contra la línea base.

Uso:
    python tools/verificar_wp.py --linea-base
        Ejecuta todo y guarda docs/implementacion/linea_base.json.
    python tools/verificar_wp.py [--lista-eliminadas ARCHIVO ...] [--salida archivo.json]
        Ejecuta todo y compara con la línea base. Código de salida 1 si algún control FALLA.
        --lista-eliminadas: archivo con un nodeid de pytest por línea (p. ej.
        tests/test_x.py::test_y o descargador/test_z.py::Clase::test_w). Cada nodo debe estar
        ausente ahora; su cantidad reemplaza a los conteos declarados a mano.
        La salida es .json; dentro del repo solo en docs/implementacion/verificacion/.
        Código y configuración nunca se excluyen de la comprobación de procedencia.
    --pruebas-eliminadas-asistente/-descargador N siguen aceptados por compatibilidad, pero
        un valor positivo da FALLA; usar --lista-eliminadas con nodeids medidos.

Controles:
  1. Suite del Asistente (pytest en tests/): 0 fallos/XPASS/deselecciones; aprobadas >= base - bajas.
  2. Suite del Descargador: igual; sin aumentar omisiones/xfail ni perder subpruebas frente a la base.
  3. Identidades reales (tools/check_real_ids.py): nunca más que la base; si la base es 0, debe ser 0.
  4. Paridad de 17 productos golden (tools/flow_baseline_golden.py): debe decir «PARIDAD PASÓ».
  5. Restricciones absolutas: sin .Send( en src/, CC fijo en policy.py, submit de registro deshabilitado,
     extensión sin acceso a document.cookie.
  6. Ruff E9,F63,F7,F82,F401,F841 en src/descargador/tools: mismo alcance que CI, motor intacto.
  7. Módulos no alcanzables: nunca más que la base.
  8. Enlaces Markdown rotos: nunca más que la base.
  9. Inventario de pruebas: ninguna baja sin declaración exacta, aunque se agreguen otras pruebas.
Reutilización (opcional, nunca en CI):
    --reutilizar-medicion toma la medición guardada en .verificacion/ (ignorado por git) si coinciden
    el árbol Git limpio, el intérprete, la plataforma, los paquetes instalados, PYTHONPATH, la
    disponibilidad de pantalla y esta herramienta. Cualquier diferencia vuelve a medir todo. Los
    controles se recalculan siempre contra la línea base; el informe marca «medicion_reutilizada».
Un control que no se puede medir (herramienta ausente o salida ilegible) da FALLA, no se omite.
En Linux sin pantalla usa xvfb-run si está disponible.
"""

import argparse
import ast
import copy
import json
import hashlib
import importlib.metadata
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASE_FILE = ROOT / "docs" / "implementacion" / "linea_base.json"
PY = sys.executable
CACHE_DIR = ROOT / ".verificacion"
CACHE_VERSION = 1
MEASURED_KEYS = (
    "asistente",
    "descargador",
    "identidades_reales",
    "paridad_golden",
    "restricciones",
    "ruff_src",
    "modulos_no_alcanzables",
    "enlaces_rotos",
)


def run(command, cwd=None, timeout=1800):
    prefix = []
    if sys.platform.startswith("linux") and not os.environ.get("DISPLAY") and shutil.which("xvfb-run"):
        prefix = ["xvfb-run", "-a"]
    try:
        result = subprocess.run(prefix + command, cwd=cwd or ROOT, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return 124, ""
    except (OSError, UnicodeError):
        return 127, ""
    return result.returncode, result.stdout + result.stderr


NODEID = re.compile(r"[\w./\\-]+\.py(?:::[\w\[\]().,=\- ]+?)*")
MAX_NODEIDS = 20


def failed_nodeids(output):
    """Nodeids de FAILED/ERROR del resumen corto de pytest (revisión externa H8).

    Solo el identificador de la prueba, nunca el mensaje: el detalle puede contener datos de la falla.
    """
    found = []
    for line in output.splitlines():
        match = re.match(r"(?:FAILED|ERROR) (\S.*?)(?: - .*)?$", line.strip())
        if match and NODEID.fullmatch(match.group(1)) and match.group(1) not in found:
            found.append(match.group(1))
    return found[:MAX_NODEIDS]


def pytest_counts(args):
    code, out = run([PY, "-m", "pytest", "-q", "-p", "no:cacheprovider", *args])
    tail = out.strip().splitlines()[-1].strip("= ") if out.strip() else ""
    # Parse the complete summary: a subtest is not a parent test, and xfailed
    # must not match failed. Unknown/truncated output cannot certify a suite.
    match = re.fullmatch(r"(.+?) in \d+(?:\.\d+)?s(?: \([^\n]+\))?", tail)
    counts = {}
    valid = bool(match)
    if match:
        for item in match.group(1).split(", "):
            token = re.fullmatch(
                r"(\d+) (passed|skipped|failed|errors?|xfailed|xpassed|deselected|warnings?|subtests passed|subtests failed|subtests skipped)",
                item,
            )
            if not token or token.group(2) in counts:
                valid = False
                break
            counts[token.group(2)] = int(token.group(1))
    valid = valid and any(
        key in counts for key in ("passed", "skipped", "failed", "error", "errors", "xfailed", "xpassed")
    )
    result = {
        "aprobadas": counts.get("passed", 0),
        "omitidas": counts.get("skipped", 0),
        "fallidas": sum(counts.get(key, 0) for key in ("failed", "error", "errors", "subtests failed")),
        "subpruebas": counts.get("subtests passed", 0),
        "xfail": counts.get("xfailed", 0),
        "xpass": counts.get("xpassed", 0),
        "deseleccionadas": counts.get("deselected", 0),
        "subpruebas_omitidas": counts.get("subtests skipped", 0),
        "codigo": code,
        "medicion_valida": bool(valid),
    }
    result["fallidas_nodeids"] = failed_nodeids(out)
    result["resumen"] = (
        f"{result['aprobadas']} aprobadas, {result['omitidas']} omitidas, "
        f"{result['fallidas']} fallidas, {result['subpruebas']} subpruebas, "
        f"{result['xfail']} xfail, {result['xpass']} xpass, "
        f"{result['deseleccionadas']} deseleccionadas, "
        f"{result['subpruebas_omitidas']} subpruebas omitidas; código {code}"
    )
    return result


def count_real_ids():
    script = ROOT / "tools" / "check_real_ids.py"
    if not script.exists():
        return None
    code, out = run([PY, str(script)])
    lines = out.strip().splitlines()
    if code == 0 and lines == ["Sin RUT reales detectados."]:
        return 0
    if code != 1 or not lines:
        return None
    match = re.fullmatch(r"(\d+) posibles identidades reales\. Reemplázalas por datos sintéticos .*", lines[-1])
    findings = sum("dígito verificador válido" in line for line in lines[:-1])
    if match and int(match.group(1)) == findings and findings > 0 and len(lines) == findings + 1:
        return findings
    return None


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
    if not (ROOT / "src").is_dir():
        return ["No disponible: código fuente de restricciones"]
    for path in (ROOT / "src").rglob("*.py"):
        if re.search(r"\.Send\(", path.read_text(encoding="utf-8", errors="ignore")):
            problems.append(f"{path.relative_to(ROOT)} usa .Send(")
    policy = ROOT / "src" / "nurus" / "services" / "policy.py"
    if not policy.exists() or 'MANDATORY_CC = "ucc_concepcion@pjud.cl"' not in policy.read_text(encoding="utf-8"):
        problems.append("policy.py no define MANDATORY_CC = ucc_concepcion@pjud.cl")
    registration = ROOT / "src" / "nurus" / "personal" / "registration.py"
    # Sin registration.py (retirado en WP-22) no queda forma de escribir en RUS. Si vuelve, submit debe lanzar error.
    if registration.exists():
        try:
            tree = ast.parse(registration.read_text(encoding="utf-8"))
        except (SyntaxError, UnicodeError, OSError):
            return problems + ["No se pudo analizar registration.py"]
        submits = [n for n in ast.walk(tree) if isinstance(n, ast.FunctionDef) and n.name == "submit"]
        if not submits:
            problems.append("registration.py existe pero no se encontró submit: no se pudo medir")
        for submit in submits:
            body = [
                n
                for n in submit.body
                if not (isinstance(n, ast.Expr) and isinstance(getattr(n, "value", None), ast.Constant))
            ]
            if not (body and isinstance(body[0], ast.Raise)):
                problems.append("registration.submit ya no lanza error al inicio")
    # Sin automatización, red ni procesos en el Asistente: no escribe en RUS/SATURNO,
    # no navega SITFA y no inicia el Descargador (control negativo; vale aunque falte registration.py).
    banned = (
        "subprocess",
        "webbrowser",
        "playwright",
        "selenium",
        "pywinauto",
        "requests",
        "httpx",
        "urllib.request",
        "http.client",
        "socket",
    )
    for path in (ROOT / "src").rglob("*.py"):
        try:
            tree = ast.parse(path.read_text(encoding="utf-8", errors="ignore"))
        except (SyntaxError, UnicodeError, OSError):
            problems.append(f"No se pudo analizar {path.relative_to(ROOT)}")
            continue
        for node in ast.walk(tree):
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute) and node.func.attr == "Send":
                problems.append(f"{path.relative_to(ROOT)} llama .Send()")
            names = (
                [a.name for a in node.names]
                if isinstance(node, ast.Import)
                else [node.module or ""]
                if isinstance(node, ast.ImportFrom) and not node.level
                else []
            )
            for name in names:
                if any(name == b or name.startswith(b + ".") for b in banned):
                    problems.append(f"{path.relative_to(ROOT)} importa {name}")
    for path in (ROOT / "descargador" / "extension").glob("*.js"):
        if re.search(r"document\.cookie\s*[=;.)\[]", path.read_text(encoding="utf-8", errors="ignore")):
            problems.append(f"{path.relative_to(ROOT)} accede a document.cookie")
    return problems


def ruff_count():
    code, out = run(
        [
            PY,
            "-m",
            "ruff",
            "check",
            "--select",
            "E9,F63,F7,F82,F401,F841",
            "--output-format",
            "json",
            "src",
            "descargador",
            "tools",
            "--extend-exclude",
            "src/nurus/personal/motor",
        ]
    )
    try:
        findings = json.loads(out)
    except (ValueError, TypeError):
        return None
    if not isinstance(findings, list) or any(
        not isinstance(item, dict) or not isinstance(item.get("code"), str) for item in findings
    ):
        return None
    return len(findings) if code == bool(findings) else None


def unreachable_count():
    script = ROOT / "tools" / "modulos_no_alcanzables.py"
    if not script.exists():
        return None
    code, out = run([PY, str(script), "--json"])
    try:
        result = json.loads(out)
        missing = result["no_alcanzables"]
    except (ValueError, KeyError, TypeError):
        return None
    if code != 0 or not isinstance(missing, list) or any(not isinstance(item, str) for item in missing):
        return None
    return len(missing)


def broken_links():
    script = ROOT / "tools" / "check_links.py"
    if not script.exists():
        return None
    code, out = run([PY, str(script)])
    lines = out.strip().splitlines()
    match = re.fullmatch(r"(\d+) enlaces rotos\.", lines[-1]) if lines else None
    if not match:
        return None
    count = int(match.group(1))
    return count if code == bool(count) and len(lines) == count + 1 else None


def collect():
    code, commit = run(["git", "rev-parse", "--short", "HEAD"])
    result = {"commit": commit.strip() if code == 0 and re.fullmatch(r"[a-f0-9]+\s*", commit) else "NO_DISPONIBLE"}
    for key, measure in (
        ("asistente", lambda: pytest_counts([])),
        ("descargador", lambda: pytest_counts(["descargador"])),
        ("identidades_reales", count_real_ids),
        ("paridad_golden", golden),
        ("restricciones", restrictions),
        ("ruff_src", ruff_count),
        ("modulos_no_alcanzables", unreachable_count),
        ("enlaces_rotos", broken_links),
    ):
        try:
            result[key] = measure()
        except (OSError, UnicodeError, ValueError, TypeError, subprocess.SubprocessError):
            # Never copy exceptions: filenames, source lines or subprocess output
            # can contain personal data. Missing measurement fails its control.
            result[key] = None
    inventory = {}
    for suite in ("asistente", "descargador"):
        nodes = collected_nodeids(suite)
        inventory[suite] = sorted(nodes) if nodes is not None else None
        if nodes is None and isinstance(result.get(suite), dict):
            result[suite]["medicion_valida"] = False
    result["pruebas_recolectadas"] = inventory
    return result


def collected_nodeids(suite):
    """Nodeids presentes ahora, relativos a la raíz (las pruebas del Descargador llevan prefijo descargador/)."""
    args = ["--collect-only", "-q", "-p", "no:cacheprovider"] + (["descargador"] if suite == "descargador" else [])
    code, out = run([PY, "-m", "pytest", *args])
    ids = {line.strip() for line in out.splitlines() if "::" in line and not line.startswith(("=", " "))}
    if suite == "descargador":
        ids = {i if i.startswith("descargador/") else "descargador/" + i for i in ids}
    return ids if code == 0 and ids else None


def removed_from_lists(paths, commit=None, baseline_nodes=None):
    """Lee las listas de nodos eliminados; devuelve conteos por suite y problemas.

    Problemas: nodos que siguen presentes, nodos repetidos y nodos que no existían en la línea base."""
    removed = {"asistente": [], "descargador": []}
    for path in paths:
        for line in Path(path).read_text(encoding="utf-8").splitlines():
            node = line.split("#", 1)[0].strip()
            if node:
                removed["descargador" if node.startswith("descargador/") else "asistente"].append(node)
    problems = []
    valid_inventory = inventory_valid(baseline_nodes)
    if not valid_inventory:
        problems.append("La línea base no registra un inventario medido de nodeids; no se certifican las bajas")
    for suite, nodes in removed.items():
        problems += [f"{node} está repetido" for node in sorted({n for n in nodes if nodes.count(n) > 1})]
        if valid_inventory:
            problems += [
                f"{node} no existía en la línea base {commit}"
                for node in sorted(set(nodes))
                if node not in baseline_nodes[suite]
            ]
        if nodes:
            present = collected_nodeids(suite)
            if present is None:
                problems.append(f"No se pudo recolectar la suite {suite} para verificar bajas")
            else:
                problems += [f"{node} sigue presente" for node in nodes if node in present]
    return (
        {suite: len(set(nodes)) for suite, nodes in removed.items()},
        problems,
        {suite: set(nodes) for suite, nodes in removed.items()},
    )


def inventory_valid(value):
    return (
        isinstance(value, dict)
        and set(value) == {"asistente", "descargador"}
        and all(
            isinstance(nodes, list)
            and all(isinstance(node, str) and "::" in node for node in nodes)
            and len(nodes) == len(set(nodes))
            for nodes in value.values()
        )
    )


def compare(now, base, removed_a, removed_d, lists=None):
    now = now if isinstance(now, dict) else {}
    base = base if isinstance(base, dict) else {}
    checks = []

    def add(name, ok, detail):
        checks.append({"control": name, "resultado": "PASA" if ok else "FALLA", "detalle": detail})

    listed, problems, declared_nodes = lists if lists else ({}, [], {})
    for key, removed in (("asistente", removed_a), ("descargador", removed_d)):
        numeric_valid = nonnegative(removed) and removed == 0
        removed = listed.get(key, 0) if numeric_valid else None
        n, b = now.get(key), base.get(key)
        valid = suite_valid(n) and suite_valid(b) and nonnegative(removed)
        if valid:
            valid = removed <= b["aprobadas"]
        if valid:
            minimum = b["aprobadas"] - removed
            add(
                f"Suite {key}",
                n["aprobadas"] >= minimum
                and n["omitidas"] <= b["omitidas"]
                and n.get("xfail", 0) <= b.get("xfail", 0)
                and n["subpruebas"] >= b["subpruebas"],
                f"{n['aprobadas']} aprobadas; mínimo {minimum}; base {b['aprobadas']}; "
                f"omitidas {n['omitidas']}/{b['omitidas']}; "
                f"xfail {n.get('xfail', 0)}/{b.get('xfail', 0)} (máximos); "
                f"subpruebas {n['subpruebas']}/{b['subpruebas']} (mínimo); código 0",
            )
        else:
            failed = (
                [x for x in (n or {}).get("fallidas_nodeids", []) if isinstance(x, str)] if isinstance(n, dict) else []
            )
            if isinstance(n, dict) and nonnegative(n.get("fallidas")) and n["fallidas"]:
                detail = f"{n['fallidas']} prueba(s) fallida(s)" + (": " + ", ".join(failed) if failed else "")
            elif isinstance(n, dict) and any(
                nonnegative(n.get(field)) and n[field] > 0
                for field in ("xpass", "deseleccionadas", "subpruebas_omitidas")
            ):
                detail = "Suite con XPASS, pruebas deseleccionadas o subpruebas omitidas; revisar antes de certificar"
            else:
                detail = (
                    "Medición, línea base o declaración de bajas inválida; usar --lista-eliminadas con nodeids medidos"
                )
            add(f"Suite {key}", False, detail)
    baseline_inventory, current_inventory = base.get("pruebas_recolectadas"), now.get("pruebas_recolectadas")
    inventory_ok = inventory_valid(baseline_inventory) and inventory_valid(current_inventory)
    inventory_details = []
    if inventory_ok:
        for suite in ("asistente", "descargador"):
            missing = set(baseline_inventory[suite]) - set(current_inventory[suite])
            undeclared = missing - declared_nodes.get(suite, set())
            if undeclared:
                inventory_ok = False
                inventory_details.append(f"{suite}: {len(undeclared)} bajas sin declarar")
    else:
        inventory_details.append("Inventario actual o de línea base ausente, ilegible o inválido")
    add("Inventario de pruebas", inventory_ok, "; ".join(inventory_details) or "Sin bajas no declaradas")
    if lists:
        add(
            "Pruebas eliminadas listadas",
            not problems,
            "; ".join(problems) or f"{sum(listed.values())} nodos ausentes como se declaró",
        )
    n, b = now.get("identidades_reales"), base.get("identidades_reales")
    valid = nonnegative(n) and nonnegative(b)
    add(
        "Identidades reales",
        valid and n <= b,
        f"{n} hallazgos; límite {b}" if valid else "Medición o línea base no disponible",
    )
    add(
        "Paridad golden",
        now.get("paridad_golden") == base.get("paridad_golden") == "PASA",
        "Paridad válida" if now.get("paridad_golden") == "PASA" else "Paridad fallida o no disponible",
    )
    n, b = now.get("restricciones"), base.get("restricciones")
    add(
        "Restricciones absolutas",
        isinstance(n, list) and not n and isinstance(b, list) and not b,
        "Sin infracciones" if n == [] else "Infracciones o análisis no disponible",
    )
    for key, label in (
        ("ruff_src", "Ruff E9/F63/F7/F82/F401/F841"),
        ("modulos_no_alcanzables", "Módulos no alcanzables"),
        ("enlaces_rotos", "Enlaces rotos"),
    ):
        n, b = now.get(key), base.get(key)
        valid = nonnegative(n) and nonnegative(b)
        add(label, valid and n <= b, f"{n}; base {b}" if valid else "Medición o línea base no disponible")
    return checks


def nonnegative(value):
    return type(value) is int and value >= 0


def suite_valid(suite):
    return (
        isinstance(suite, dict)
        and all(nonnegative(suite.get(key)) for key in ("aprobadas", "omitidas", "fallidas", "subpruebas"))
        # Historical baselines predate these fields; absent means zero, malformed never does.
        and all(nonnegative(suite.get(key, 0)) for key in ("xfail", "xpass", "deseleccionadas", "subpruebas_omitidas"))
        and all(suite.get(key, 0) == 0 for key in ("xpass", "deseleccionadas", "subpruebas_omitidas"))
        and type(suite.get("codigo")) is int
        and suite["codigo"] == 0
        and suite["fallidas"] == 0
        and suite.get("medicion_valida", True) is True
    )


def nonnegative_argument(value):
    number = int(value)
    if number < 0:
        raise argparse.ArgumentTypeError("El número de pruebas eliminadas no puede ser negativo")
    return number


def write_json(path, value):
    # Replace atomically so an interrupted write cannot destroy the baseline.
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=path.parent, delete=False) as stream:
            temporary = Path(stream.name)
            json.dump(value, stream, ensure_ascii=False, indent=2)
            stream.flush()
            os.fsync(stream.fileno())
        temporary.replace(path)
    finally:
        # Windows requires the file handle to close before it can be removed.
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def report_path(output):
    """A report cannot replace code/configuration or hide it from provenance."""
    try:
        target = Path(output).resolve()
    except (OSError, ValueError, RuntimeError):
        raise ValueError("No se pudo resolver la ruta del informe") from None
    if target == BASE_FILE.resolve():
        raise ValueError("--salida no puede sobrescribir la línea base")
    if target.suffix.lower() != '.json':
        raise ValueError("--salida debe ser un informe .json")
    root = ROOT.resolve()
    reports = root / 'docs' / 'implementacion' / 'verificacion'
    if target.is_relative_to(root) and not target.is_relative_to(reports):
        raise ValueError("Dentro del repositorio, --salida solo admite docs/implementacion/verificacion/*.json")
    return target


def provenance(output=None):
    """Identify the committed tree and detect any worktree change during tests.

    Paths/content are hashed in memory and never included in the report. Only
    the explicitly requested report is excluded, so writing it cannot invalidate
    its own measurement. Other documentation and untracked files still count.
    """
    excluded = None
    if output:
        try:
            output = report_path(output)
        except ValueError:
            return None
        try:
            excluded = output.relative_to(ROOT.resolve()).as_posix()
        except ValueError:
            pass

    def git(*args):
        result = subprocess.run(["git", *args], cwd=ROOT, capture_output=True, timeout=30)
        if result.returncode:
            raise ValueError("Git no disponible")
        return result.stdout

    try:
        head = git("rev-parse", "HEAD").decode("ascii").strip()
        tree = git("rev-parse", "HEAD^{tree}").decode("ascii").strip()
        if not re.fullmatch(r"[a-f0-9]{40,64}", head) or not re.fullmatch(r"[a-f0-9]{40,64}", tree):
            return None
        pathspec = ["--", "."] + ([f":(exclude,literal){excluded}"] if excluded else [])
        status = git("status", "--porcelain=v1", "-z", "--untracked-files=all", *pathspec)
        digest = hashlib.sha256(status)
        digest.update(git("diff", "--cached", "--binary", *pathspec))
        names = git("ls-files", "--cached", "--others", "--exclude-standard", "-z", *pathspec)
        for name in sorted(set(names.split(b"\0")) - {b""}):
            digest.update(name + b"\0")
            path = ROOT / os.fsdecode(name)
            if path.is_symlink():
                digest.update(b"link:" + os.fsencode(os.readlink(path)))
            elif path.is_file():
                with path.open("rb") as stream:
                    for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                        digest.update(chunk)
            else:
                digest.update(b"missing")
        return {
            "commit": head,
            "arbol_git": tree,
            "cambios_locales": bool(status),
            "huella_trabajo": digest.hexdigest(),
        }
    except (OSError, UnicodeError, ValueError, subprocess.SubprocessError):
        return None


def environment_key(tree):
    """Huella de todo lo que puede cambiar una medición además del código versionado."""
    packages = sorted(
        f"{(dist.metadata['Name'] or '').lower()}=={dist.version}" for dist in importlib.metadata.distributions()
    )
    display = bool(os.environ.get("DISPLAY")) or bool(shutil.which("xvfb-run"))
    payload = {
        "version": CACHE_VERSION,
        "arbol_git": tree,
        "python": sys.version,
        "interprete": str(Path(PY).resolve()),
        "plataforma": [sys.platform, platform.machine(), platform.release()],
        "paquetes": packages,
        "pythonpath": os.environ.get("PYTHONPATH", ""),
        "pantalla": display,
        "herramienta": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    }
    return hashlib.sha256(json.dumps(payload, sort_keys=True).encode("utf-8")).hexdigest()


def complete(measurement):
    """Solo se guarda o reutiliza una medición sin controles ausentes."""
    if not isinstance(measurement, dict) or any(measurement.get(key) is None for key in MEASURED_KEYS):
        return False
    inventory = measurement.get("pruebas_recolectadas")
    return (
        all(suite_valid(measurement.get(suite)) for suite in ("asistente", "descargador"))
        and isinstance(inventory, dict)
        and all(isinstance(inventory.get(suite), list) for suite in ("asistente", "descargador"))
    )


def load_cached(key):
    try:
        data = json.loads((CACHE_DIR / f"{key}.json").read_text(encoding="utf-8"))
    except (OSError, UnicodeError, ValueError):
        return None
    if not isinstance(data, dict) or data.get("clave") != key or not complete(data.get("medicion")):
        return None
    return data["medicion"]


def store_cached(key, measurement):
    if not complete(measurement):
        return
    try:
        write_json(CACHE_DIR / f"{key}.json", {"clave": key, "medicion": measurement})
    except OSError:
        pass  # La caché es una optimización: fallar al escribirla no cambia el veredicto.


def public_provenance(value):
    return {key: value[key] for key in ("commit", "arbol_git", "cambios_locales")} if value else None


def suite_report(value):
    """Publish diagnostic metrics, never subprocess output or arbitrary fields."""
    value = value if isinstance(value, dict) else {}
    result = {
        key: value.get(key) if nonnegative(value.get(key)) else None
        for key in (
            "aprobadas",
            "omitidas",
            "fallidas",
            "subpruebas",
            "xfail",
            "xpass",
            "deseleccionadas",
            "subpruebas_omitidas",
        )
    }
    result["codigo"] = value.get("codigo") if type(value.get("codigo")) is int else None
    measured = value.get("medicion_valida", True)
    result["medicion_valida"] = measured if type(measured) is bool else False
    nodeids = value.get("fallidas_nodeids", [])
    result["fallidas_nodeids"] = (
        [x for x in nodeids if isinstance(x, str) and NODEID.fullmatch(x)][:MAX_NODEIDS]
        if isinstance(nodeids, list)
        else []
    )
    return result


def main(argv):
    parser = argparse.ArgumentParser(description="Verificación estándar de un WP.")
    parser.add_argument("--linea-base", action="store_true")
    parser.add_argument("--pruebas-eliminadas-asistente", type=nonnegative_argument, default=0)
    parser.add_argument("--pruebas-eliminadas-descargador", type=nonnegative_argument, default=0)
    parser.add_argument("--lista-eliminadas", type=Path, action="append", default=[])
    parser.add_argument("--salida", type=Path)
    parser.add_argument("--reutilizar-medicion", action="store_true")
    args = parser.parse_args(argv)
    if args.salida:
        try:
            args.salida = report_path(args.salida)
        except ValueError as exc:
            parser.error(str(exc))
    base = None
    baseline_exists = BASE_FILE.exists()
    if baseline_exists:
        try:
            base = json.loads(BASE_FILE.read_text(encoding="utf-8"))
        except (ValueError, UnicodeError, OSError):
            base = {}
    before = provenance(args.salida)
    clean = bool(before and not before["cambios_locales"])
    key = None
    if clean:
        try:
            key = environment_key(before["arbol_git"])
        except (OSError, ValueError, TypeError):
            key = None
    cached = load_cached(key) if key and args.reutilizar_medicion and not os.environ.get("CI") else None
    reused = cached is not None
    now = copy.deepcopy(cached) if reused else collect()
    after = provenance(args.salida)
    eligible = bool(before and after and before == after and not before["cambios_locales"])
    if eligible and key and not reused:
        store_cached(key, now)
    if before:
        now["commit"] = before["commit"]
    # Only an absent baseline may be initialized against the current healthy
    # measurements. An existing corrupt baseline must never be silently reset.
    reference = now if not baseline_exists and args.linea_base else base
    lists = None
    if args.lista_eliminadas:
        try:
            lists = removed_from_lists(
                args.lista_eliminadas,
                reference.get("commit") if isinstance(reference, dict) else None,
                reference.get("pruebas_recolectadas") if isinstance(reference, dict) else None,
            )
        except (OSError, UnicodeError, ValueError):
            lists = ({}, ["No se pudieron verificar las listas de pruebas retiradas"], {})
    checks = compare(now, reference, args.pruebas_eliminadas_asistente, args.pruebas_eliminadas_descargador, lists)
    report = {
        "commit": now.get("commit", "NO_DISPONIBLE"),
        "base": reference.get("commit", "NO_DISPONIBLE") if isinstance(reference, dict) else "NO_DISPONIBLE",
        "controles": checks,
        "suites": {suite: suite_report(now.get(suite)) for suite in ("asistente", "descargador")},
        "procedencia_inicio": public_provenance(before),
        "procedencia_fin": public_provenance(after),
        "procedencia_verificada": eligible,
        "elegible_integracion": eligible and all(c["resultado"] == "PASA" for c in checks),
        "medicion_reutilizada": reused,
        "veredicto": "PASA" if eligible and all(c["resultado"] == "PASA" for c in checks) else "FALLA",
    }
    if reused:
        print("[INFO] Medición reutilizada: mismo árbol limpio y mismo entorno (caché local).")
    for check in checks:
        print(f"[{check['resultado']}] {check['control']}: {check['detalle']}")
    if not eligible:
        report["motivo_procedencia"] = "Árbol sin commit, modificado durante la medición o identidad Git no disponible"
        print("[FALLA] Procedencia:", report["motivo_procedencia"])
    print("VEREDICTO:", report["veredicto"])
    if args.salida:
        write_json(args.salida, report)
    if args.linea_base and report["veredicto"] == "PASA":
        write_json(BASE_FILE, now)
        print("Línea base guardada.")
    return 0 if report["veredicto"] == "PASA" else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
