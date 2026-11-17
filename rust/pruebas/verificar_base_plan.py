"""Conserva el veredicto del verificador y compara sus hallazgos con la punta inicial."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
BASE = "cfc12ee2af5e4fb5623671fd17ccf7feb9e1cc11"


def medir(root, salida, nombre):
    env = dict(os.environ, PYTHONPATH=str(root / "src"))
    reporte = root / "docs/implementacion/verificacion/FLOW_D.json"
    proceso = subprocess.run([sys.executable, str(root / "tools/verificar_wp.py"), "--salida", str(reporte)], cwd=root, env=env)
    datos = json.loads(reporte.read_text())
    (salida / f"{nombre}.json").write_text(json.dumps(datos, indent=2), encoding="utf-8")
    spec = importlib.util.spec_from_file_location(f"verificador_{nombre}", root / "tools/verificar_wp.py")
    modulo = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(modulo)
    anterior = os.environ.get("PYTHONPATH")
    os.environ["PYTHONPATH"] = str(root / "src")
    try:
        base = json.loads((root / "docs/implementacion/linea_base.json").read_text())
        bajas = {}
        for suite in ["asistente", "descargador"]:
            presentes = modulo.collected_nodeids(suite)
            if presentes is None:
                raise RuntimeError(f"No se pudo medir el inventario {suite}")
            bajas[suite] = sorted(set(base["pruebas_recolectadas"][suite]) - presentes)
        diagnostico = {"restricciones": modulo.restrictions(), "bajas": bajas}
    finally:
        if anterior is None:
            os.environ.pop("PYTHONPATH", None)
        else:
            os.environ["PYTHONPATH"] = anterior
    (salida / f"{nombre}_diagnostico.json").write_text(json.dumps(diagnostico, indent=2), encoding="utf-8")
    return proceso.returncode, datos, diagnostico


def main():
    salida = Path(os.environ["RUNNER_TEMP"]) / "flow-d-verificacion"
    salida.mkdir(exist_ok=False)
    codigo, actual, diagnostico = medir(ROOT, salida, "candidato")
    with tempfile.TemporaryDirectory(prefix="flow-base-") as temporal:
        clon = Path(temporal) / "base"
        subprocess.run(["git", "clone", "--shared", "--no-checkout", str(ROOT), str(clon)], check=True)
        subprocess.run(["git", "-C", str(clon), "checkout", "--detach", BASE], check=True)
        _, base, diagnostico_base = medir(clon, salida, "base")
    comparacion = {
        "sha_base": BASE,
        "sha_candidato": actual["commit"],
        "veredicto_candidato": actual["veredicto"],
        "veredicto_base": base["veredicto"],
        "hallazgos_identicos_a_base": diagnostico == diagnostico_base,
        "controles_fallidos_candidato": [c["control"] for c in actual["controles"] if c["resultado"] == "FALLA"],
        "controles_fallidos_base": [c["control"] for c in base["controles"] if c["resultado"] == "FALLA"],
        "diagnostico": diagnostico,
    }
    (salida / "comparacion.json").write_text(json.dumps(comparacion, indent=2), encoding="utf-8")
    print("FLOW_WP_COMPARACION_BEGIN")
    print(json.dumps(comparacion))
    print("FLOW_WP_COMPARACION_END")
    # No convertir FALLA en PASA aunque los hallazgos sean heredados.
    return codigo


if __name__ == "__main__":
    sys.exit(main())

