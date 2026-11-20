"""Compara Python y Flow Rust sobre un mismo ZIP sintético del Descargador."""

from datetime import date
import json
import os
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory


def clasificacion_rust(ejecutable, paquete, causa, desde, hasta):
    stdout = subprocess.check_output([str(ejecutable), str(paquete), desde, hasta])
    filas = [f for f in json.loads(stdout) if f["causa"] == causa]
    assert len(filas) == 1, (causa, desde, hasta)
    return filas[0]["resultado"]["clase"]


def comprobar_productos(app, carpeta, trabajo, paquete):
    from openpyxl import load_workbook
    from nurus.personal.revision_bitacoras import exportar, revisar

    desde, hasta = date(2026, 9, 8), date(2026, 10, 8)
    py_excel, py_informe = exportar(trabajo, revisar(trabajo, desde, hasta), desde, hasta, carpeta / "python")
    salida = carpeta / "rust"
    subprocess.run(
        [
            "cargo", "test", "--manifest-path", str(app / "Cargo.toml"), "--locked",
            "--bin", "flow_rust", "revision_ui::tests::exporta_mismo_lote_de_python", "--", "--exact",
        ],
        check=True,
        env=dict(os.environ, FLOW_P2_PAQUETE=str(paquete), FLOW_P2_SALIDA=str(salida)),
    )
    informe_python = json.loads(py_informe.read_text(encoding="utf-8"))
    informe_rust = json.loads((salida / "Revision_bitacoras.json").read_text(encoding="utf-8"))
    for campo in ("tipo", "version", "periodo", "corte", "resumen"):
        assert informe_rust[campo] == informe_python[campo], campo
    assert [r["clasificacion"] for r in informe_rust["registros"]] == [
        r["clasificacion"] for r in informe_python["registros"]
    ]

    def filas(ruta):
        libro = load_workbook(ruta, read_only=True)
        try:
            return list(libro.active.iter_rows(values_only=True))
        finally:
            libro.close()

    python, rust = filas(py_excel), filas(salida / "Revision_bitacoras.xlsx")
    assert python[0] == rust[0]
    # Motivo y observación son textos de los motores respectivos. Aquí se cotejan
    # la clasificación y los datos visibles que identifican cada fila.
    columnas = (0, 2, 3, 4, 5)
    assert [tuple(f[i] for i in columnas) for f in rust[1:]] == [
        tuple(f[i] for i in columnas) for f in python[1:]
    ]
    print("Excel y JSON coinciden en período, clasificaciones, conteos y datos de los registros.")


def main():
    ejecutable = Path(sys.argv[1]).resolve(strict=True)
    referencia = Path(sys.argv[2]).resolve(strict=True) if len(sys.argv) > 2 else Path.cwd()
    sys.path[:0] = [str(referencia / "descargador"), str(referencia / "src")]
    from test_revision_bitacoras_p2 import _paginar, _trabajo, diaries
    from nurus.personal.revision_bitacoras import clasificar

    app = Path(__file__).resolve().parents[1]
    casos_path = app.parent / "docs/migracion_python_rust/CASOS_PRIMERA_ENTREGA.json"
    casos = json.loads(casos_path.read_text(encoding="utf-8"))["casos"]
    with TemporaryDirectory(prefix="paridad-p2-", dir=app / "target") as temporal:
        carpeta = Path(temporal)
        manifiesto = diaries(carpeta / "Bitacoras 2026-10-13", 2)
        _paginar(manifiesto, 1)
        trabajo, filas_python, paquete = _trabajo(carpeta, manifiesto)
        for caso in casos:
            causa = caso["causa_ficticia"]
            desde, hasta = caso["desde"], caso["hasta"]
            python, _ = clasificar(filas_python[causa].revision_bitacora, date.fromisoformat(desde), date.fromisoformat(hasta))
            rust = clasificacion_rust(ejecutable, paquete, causa, desde, hasta)
            assert python == rust == caso["esperado"], (caso, python, rust)
            print(f"{desde} a {hasta}, causa ficticia {causa}: {rust}")
        comprobar_productos(app, carpeta, trabajo, paquete)

        carpeta_ajena = carpeta / "identidad-ajena"
        carpeta_ajena.mkdir()
        manifiesto = diaries(carpeta_ajena / "Bitacoras 2026-10-13", 1)
        control = json.loads(manifiesto.read_text(encoding="utf-8"))
        control["registros"][0]["nombre"] = "Persona Ficticia Distinta"
        manifiesto.write_text(json.dumps(control), encoding="utf-8")
        _, filas, paquete = _trabajo(carpeta_ajena, manifiesto)
        desde, hasta = "2026-08-01", "2026-08-31"
        python, _ = clasificar(filas["100"].revision_bitacora, date.fromisoformat(desde), date.fromisoformat(hasta))
        rust = clasificacion_rust(ejecutable, paquete, "100", desde, hasta)
        assert python == rust == "NO_VERIFICABLE", (python, rust)
        print("Identidad visible discrepante: NO_VERIFICABLE en ambos programas.")
    print(f"{len(casos) + 1} casos P2 coinciden entre Python y Flow Rust.")


if __name__ == "__main__":
    main()
