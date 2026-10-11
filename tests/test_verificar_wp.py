"""Regresiones de la puerta de verificación, con procesos y datos ficticios."""

import copy
import importlib.util
import json
import subprocess
from pathlib import Path

import pytest

SPEC = importlib.util.spec_from_file_location(
    "verificar_wp_under_test", Path(__file__).resolve().parents[1] / "tools" / "verificar_wp.py"
)
verifier = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verifier)


@pytest.fixture(autouse=True)
def clean_provenance(monkeypatch):
    monkeypatch.setattr(
        verifier,
        "provenance",
        lambda output=None: {
            "commit": "a" * 40,
            "arbol_git": "b" * 40,
            "cambios_locales": False,
            "huella_trabajo": "c" * 64,
        },
    )


@pytest.fixture
def healthy():
    suite = {
        "aprobadas": 10,
        "omitidas": 1,
        "fallidas": 0,
        "subpruebas": 2,
        "codigo": 0,
        "resumen": "10 passed, 1 skipped",
    }
    return {
        "commit": "abcdef0",
        "pruebas_recolectadas": {"asistente": ["tests/test_fictitious.py::test_known"], "descargador": []},
        "asistente": suite.copy(),
        "descargador": suite.copy(),
        "identidades_reales": 12,
        "paridad_golden": "PASA",
        "restricciones": [],
        "ruff_src": 0,
        "modulos_no_alcanzables": 12,
        "enlaces_rotos": 26,
    }


def passes(checks):
    assert len(checks) == 9
    return all(check["resultado"] == "PASA" for check in checks)


@pytest.mark.parametrize("code", [1, 2, 3, 4, 5, 124, 127])
def test_pytest_error_with_passed_tests_cannot_pass(monkeypatch, healthy, code):
    monkeypatch.setattr(verifier, "run", lambda *a, **k: (code, "10 passed, 1 skipped in 0.10s"))
    now = copy.deepcopy(healthy)
    now["asistente"] = verifier.pytest_counts([])
    assert now["asistente"]["aprobadas"] == 10
    assert not passes(verifier.compare(now, healthy, 0, 0))


@pytest.mark.parametrize(
    "output,passed,failed,subtests,valid",
    [
        ("10 passed, 2 xfailed, 3 subtests passed in 0.10s", 10, 0, 3, True),
        ("2 passed, 1 failed, 2 errors, 1 subtests failed in 1.20s", 2, 4, 0, True),
        ("3 subtests passed in 0.10s", 0, 0, 3, False),
        ("10 passed", 0, 0, 0, False),
        ("private-marker", 0, 0, 0, False),
        ("10 passed, unknown summary in 0.10s", 10, 0, 0, False),
    ],
)
def test_pytest_summary_is_complete_and_distinguishes_xfail(monkeypatch, output, passed, failed, subtests, valid):
    monkeypatch.setattr(verifier, "run", lambda *a, **k: (0, output))
    result = verifier.pytest_counts([])
    assert (result["aprobadas"], result["fallidas"], result["subpruebas"], result["medicion_valida"]) == (
        passed,
        failed,
        subtests,
        valid,
    )
    assert "private-marker" not in result["resumen"]
    published = verifier.suite_report(result | {"stdout": "private-marker", "traceback": "private-marker"})
    assert "private-marker" not in json.dumps(published)


@pytest.mark.parametrize(
    "key",
    [
        "asistente",
        "descargador",
        "pruebas_recolectadas",
        "identidades_reales",
        "paridad_golden",
        "restricciones",
        "ruff_src",
        "modulos_no_alcanzables",
        "enlaces_rotos",
    ],
)
@pytest.mark.parametrize("bad", [None, "malformed", {}, True])
def test_missing_or_malformed_control_never_disappears(healthy, key, bad):
    now = copy.deepcopy(healthy)
    now[key] = bad
    assert not passes(verifier.compare(now, healthy, 0, 0))
    assert not passes(verifier.compare(healthy, now, 0, 0))
    del now[key]
    assert not passes(verifier.compare(now, healthy, 0, 0))


@pytest.mark.parametrize("removed", [-1, 1, 11, True])
def test_invalid_declared_test_removals_fail(healthy, removed):
    assert not passes(verifier.compare(healthy, healthy, removed, 0))


@pytest.fixture
def scanner_root(tmp_path, monkeypatch):
    (tmp_path / "tools").mkdir()
    for filename in ("check_real_ids.py", "check_links.py", "modulos_no_alcanzables.py"):
        (tmp_path / "tools" / filename).touch()
    monkeypatch.setattr(verifier, "ROOT", tmp_path)
    return tmp_path


@pytest.mark.parametrize("method", ["count_real_ids", "ruff_count", "unreachable_count", "broken_links"])
@pytest.mark.parametrize("code,output", [(1, "Traceback private-marker"), (2, ""), (124, ""), (0, "malformed")])
def test_scanner_crash_or_malformed_output_not_zero(monkeypatch, scanner_root, method, code, output):
    monkeypatch.setattr(verifier, "run", lambda *a, **k: (code, output))
    assert getattr(verifier, method)() is None


@pytest.mark.parametrize(
    "method,code,output,expected",
    [
        ("count_real_ids", 0, "Sin RUT reales detectados.\n", 0),
        (
            "count_real_ids",
            1,
            "fixture:1: RUT con dígito verificador válido (ficticio)\n1 posibles identidades reales. Reemplázalas por datos sintéticos de prueba.\n",
            1,
        ),
        ("broken_links", 1, "fixture:1: missing\n1 enlaces rotos.\n", 1),
        ("ruff_count", 1, '[{"code":"F821"}]', 1),
        ("unreachable_count", 0, '{"no_alcanzables": ["ficticio"]}', 1),
    ],
)
def test_valid_findings_are_measurements_not_process_errors(monkeypatch, scanner_root, method, code, output, expected):
    monkeypatch.setattr(verifier, "run", lambda *a, **k: (code, output))
    assert getattr(verifier, method)() == expected


def test_timeout_is_safe_failure(monkeypatch):
    def timeout(*args, **kwargs):
        raise subprocess.TimeoutExpired("private-marker", 1, output="private-marker")

    monkeypatch.setattr(verifier.subprocess, "run", timeout)
    assert verifier.run(["ficticio"], timeout=1) == (124, "")


def test_syntax_error_in_restrictions_fails_without_source(monkeypatch, tmp_path, capsys):
    personal = tmp_path / "src" / "nurus" / "personal"
    personal.mkdir(parents=True)
    (personal / "registration.py").write_text("def submit(private_marker invalid syntax", encoding="utf-8")
    monkeypatch.setattr(verifier, "ROOT", tmp_path)
    problems = verifier.restrictions()
    assert "No se pudo analizar registration.py" in problems
    assert "private_marker" not in json.dumps(problems)
    assert "private_marker" not in capsys.readouterr().out


@pytest.fixture
def baseline_file(monkeypatch, tmp_path):
    target = tmp_path / "linea_base.json"
    monkeypatch.setattr(verifier, "BASE_FILE", target)
    return target


@pytest.mark.parametrize("bad", ["process", "increased_debt", "fewer_tests", "missing"])
def test_failed_update_preserves_baseline_bytes(monkeypatch, baseline_file, healthy, bad, capsys):
    original = json.dumps(healthy).encode()
    baseline_file.write_bytes(original)
    now = copy.deepcopy(healthy)
    if bad == "process":
        now["asistente"]["codigo"] = 2
        now["asistente"]["resumen"] = "private-marker"
    elif bad == "increased_debt":
        now["identidades_reales"] += 1
    elif bad == "fewer_tests":
        now["descargador"]["aprobadas"] -= 1
    else:
        now["ruff_src"] = None
    monkeypatch.setattr(verifier, "collect", lambda: now)
    assert verifier.main(["--linea-base"]) == 1
    assert baseline_file.read_bytes() == original
    assert "private-marker" not in capsys.readouterr().out


@pytest.mark.parametrize("contents", [b"not JSON", b"{}", b"null"])
def test_existing_corrupt_baseline_not_replaced(monkeypatch, baseline_file, healthy, contents):
    baseline_file.write_bytes(contents)
    monkeypatch.setattr(verifier, "collect", lambda: healthy)
    assert verifier.main(["--linea-base"]) == 1
    assert baseline_file.read_bytes() == contents


def test_initial_baseline_needs_all_healthy_controls(monkeypatch, baseline_file, healthy):
    healthy["ruff_src"] = None
    monkeypatch.setattr(verifier, "collect", lambda: healthy)
    assert verifier.main(["--linea-base"]) == 1
    assert not baseline_file.exists()


def test_valid_historical_debt_can_initialize_and_update(monkeypatch, baseline_file, healthy):
    monkeypatch.setattr(verifier, "collect", lambda: healthy)
    assert verifier.main(["--linea-base"]) == 0
    healthy["asistente"]["aprobadas"] += 1
    assert verifier.main(["--linea-base"]) == 0
    assert json.loads(baseline_file.read_text(encoding="utf-8"))["asistente"]["aprobadas"] == 11


def test_output_cannot_overwrite_baseline(monkeypatch, baseline_file, healthy):
    baseline_file.write_text(json.dumps(healthy), encoding="utf-8")
    monkeypatch.setattr(verifier, "collect", lambda: pytest.fail("must reject before running"))
    with pytest.raises(SystemExit) as exc:
        verifier.main(["--salida", str(baseline_file)])
    assert exc.value.code == 2


def test_negative_cli_removal_rejected_before_running(monkeypatch):
    monkeypatch.setattr(verifier, "collect", lambda: pytest.fail("must reject before running"))
    with pytest.raises(SystemExit) as exc:
        verifier.main(["--pruebas-eliminadas-asistente", "-1"])
    assert exc.value.code == 2


@pytest.mark.parametrize("change", ["commit", "huella_trabajo", "cambios_locales", "unavailable"])
def test_changed_or_dirty_worktree_not_integrable(monkeypatch, baseline_file, healthy, change, tmp_path):
    before = {"commit": "a" * 40, "arbol_git": "b" * 40, "cambios_locales": False, "huella_trabajo": "c" * 64}
    after = before.copy()
    if change == "unavailable":
        after = None
    elif change == "cambios_locales":
        before[change] = after[change] = True
    else:
        after[change] = "d" * len(after[change])
    identities = iter([before, after])
    monkeypatch.setattr(verifier, "provenance", lambda output=None: next(identities))
    monkeypatch.setattr(verifier, "collect", lambda: healthy)
    report_file = tmp_path / "report.json"
    assert verifier.main(["--linea-base", "--salida", str(report_file)]) == 1
    assert not baseline_file.exists()
    report = json.loads(report_file.read_text(encoding="utf-8"))
    assert report["elegible_integracion"] is False
    assert report["veredicto"] == "FALLA"
    assert len(report["controles"]) == 9
    assert "huella_trabajo" not in json.dumps(report)


@pytest.mark.parametrize('declared,expected', [('test_known[synthetic]', 0), ('test_known[invented]', 1)])
def test_cli_removed_list_uses_exact_baseline_inventory(
    monkeypatch, baseline_file, healthy, tmp_path, declared, expected
):
    node = 'tests/test_synthetic.py::test_known[synthetic]'
    healthy['pruebas_recolectadas'] = {'asistente': [node], 'descargador': []}
    baseline_file.write_text(json.dumps(healthy), encoding="utf-8")
    now = copy.deepcopy(healthy)
    now['asistente']['aprobadas'] -= 1
    now['pruebas_recolectadas']['asistente'] = []
    monkeypatch.setattr(verifier, 'collect', lambda: now)
    monkeypatch.setattr(verifier, 'collected_nodeids', lambda *_: set())
    manifest = tmp_path / 'removed.txt'
    manifest.write_text('tests/test_synthetic.py::' + declared, encoding="utf-8")
    assert verifier.main(['--lista-eliminadas', str(manifest)]) == expected


def test_clean_source_with_failed_control_is_not_integrable(monkeypatch, baseline_file, healthy, tmp_path):
    baseline_file.write_text(json.dumps(healthy), encoding="utf-8")
    now = copy.deepcopy(healthy)
    now['asistente']['fallidas'] = 1
    monkeypatch.setattr(verifier, 'collect', lambda: now)
    report_file = tmp_path / 'report.json'
    assert verifier.main(['--salida', str(report_file)]) == 1
    report = json.loads(report_file.read_text(encoding="utf-8"))
    assert report['procedencia_verificada'] is True
    assert report['elegible_integracion'] is False
    assert report['veredicto'] == 'FALLA'
    assert report['suites']['asistente']['fallidas'] == 1
    assert report['suites']['asistente']['codigo'] == 0
    assert 'resumen' not in report['suites']['asistente']
    assert 'controles' not in report['suites']['asistente']


@pytest.mark.parametrize(
    'scenario,expected',
    [
        ('unchanged', 'PASA'),
        ('undeclared', 'FALLA'),
        ('new_tests_mask_deletions', 'FALLA'),
        ('partially_declared', 'FALLA'),
        ('fully_declared', 'PASA'),
    ],
)
def test_inventory_prevents_new_tests_from_hiding_unreported_deletions(healthy, scenario, expected):
    base = copy.deepcopy(healthy)
    nodes = [f'tests/test_synthetic.py::test_case_{index}' for index in range(10)]
    base['pruebas_recolectadas']['asistente'] = nodes
    now = copy.deepcopy(base)
    lists = None
    if scenario != 'unchanged':
        now['pruebas_recolectadas']['asistente'] = nodes[:6]
        now['asistente']['aprobadas'] = 6
        if scenario != 'undeclared':
            now['pruebas_recolectadas']['asistente'] += [f'tests/test_new.py::test_added_{i}' for i in range(5)]
            now['asistente']['aprobadas'] = 11
        if scenario in {'partially_declared', 'fully_declared'}:
            declared = set(nodes[6:9] if scenario == 'partially_declared' else nodes[6:])
            lists = ({'asistente': len(declared), 'descargador': 0}, [], {'asistente': declared, 'descargador': set()})
    checks = verifier.compare(now, base, 0, 0, lists)
    inventory = next(check for check in checks if check['control'] == 'Inventario de pruebas')
    assert inventory['resultado'] == expected
    if scenario == 'new_tests_mask_deletions':
        assert next(check for check in checks if check['control'] == 'Suite asistente')['resultado'] == 'PASA'
