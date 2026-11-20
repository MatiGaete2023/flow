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
    from openpyxl.utils.cell import range_boundaries
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

    libro_python = load_workbook(py_excel, read_only=True)
    libro_rust = load_workbook(salida / "Revision_bitacoras.xlsx")
    libro_general = load_workbook(next(carpeta.glob("Observaciones_*.xlsx")))
    try:
        general, bitacoras = libro_general.active, libro_rust.active
        comunes = general.max_column
        filas_generales = list(general.values)
        filas_bitacoras = list(bitacoras.values)
        assert bitacoras.title == general.title
        assert [f[:comunes] for f in filas_bitacoras] == filas_generales
        assert filas_bitacoras[0][comunes:] == ("CLASIFICACION_BITACORA", "MOTIVO_BITACORA")
        assert bitacoras.freeze_panes == general.freeze_panes == "A2"
        assert range_boundaries(bitacoras.auto_filter.ref) == (1, 1, comunes + 2, bitacoras.max_row)
        for fila in general.iter_rows():
            for celda in fila:
                assert bitacoras[celda.coordinate].style_id == celda.style_id, celda.coordinate
        for columna, dimension in general.column_dimensions.items():
            assert bitacoras.column_dimensions[columna].width == dimension.width, columna

        indices = [filas_bitacoras[0].index(c) for c in (
            "CLASIFICACION_BITACORA", "TRIBUNAL", "RIT", "NOMBRE", "DERIVACION",
        )]
        python = list(libro_python.active.values)
        assert [tuple(f[i] for i in indices) for f in filas_bitacoras[1:]] == [
            tuple(f[i] for i in (0, 2, 3, 4, 5)) for f in python[1:]
        ]
    finally:
        libro_general.close()
        libro_rust.close()
        libro_python.close()
    print("Bitácoras conserva columnas, valores y formato del Excel general de Flow Rust.")
    print("Excel y JSON conservan las clasificaciones y los datos de la revisión de Flow Python.")


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
