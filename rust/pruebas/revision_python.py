"""Contrasta el ZIP producido por el Descargador Python con Excel y JSON Rust."""

import json
import subprocess
import sys
from datetime import date
from pathlib import Path
from tempfile import TemporaryDirectory

from openpyxl import load_workbook

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "descargador"))

from test_revision_bitacoras_p2 import _paginar, _trabajo, diaries  # noqa: E402

CASOS = ROOT / "docs/migracion_python_rust/CASOS_PRIMERA_ENTREGA.json"
ETIQUETAS = {
    "CON_OBSERVACION": ("ConObservacion", "Con observación", 1),
    "SIN_OBSERVACION": ("SinObservacion", "Sin observación", 0),
    "NO_VERIFICABLE": ("NoVerificable", "No verificables", 2),
}
CORTE = "2026-10-13"


def comprobar_productos(carpeta, causa, esperado):
    clase, hoja, posicion = ETIQUETAS[esperado]
    [producto] = carpeta.iterdir()
    [json_path] = producto.glob("*.json")
    [excel_path] = producto.glob("*.xlsx")
    informe = json.loads(json_path.read_text(encoding="utf-8"))
    [fila] = [f for f in informe["filas"] if f["identidad"]["causa"] == causa]
    assert fila["revision"]["clasificacion"] == clase, fila["revision"]
    assert fila["lecturas"][0]["evidencia"]["version"] == 2
    libro = load_workbook(excel_path, read_only=True)
    try:
        [coincidencia] = [f for f in libro[hoja].values if f[2] == causa]
        assert coincidencia[8] == hoja
        assert list(libro["Resumen"].values)[14 + posicion][2] == informe["conteos_exportados"][posicion]
    finally:
        libro.close()


def verificar():
    casos = json.loads(CASOS.read_text(encoding="utf-8"))["casos"]
    with TemporaryDirectory(prefix="revision-python-") as temporal:
        raiz = Path(temporal)
        manifest = diaries(raiz / "Bitacoras 2026-10-13", 2)
        _paginar(manifest, 1)
        _, filas_python, paquete = _trabajo(raiz, manifest)
        from nurus.personal.revision_bitacoras import clasificar

        carpetas = {}
        for caso in casos:
            causa, desde, hasta = (caso[k] for k in ("causa_ficticia", "desde", "hasta"))
            esperado = caso["esperado"]
            clasificacion, _ = clasificar(
                filas_python[causa].revision_bitacora, date.fromisoformat(desde), date.fromisoformat(hasta)
            )
            assert clasificacion == esperado
            periodo = (desde, hasta)
            if periodo not in carpetas:
                salida = raiz / f"revision-{desde}_{hasta}"
                salida.mkdir()
                subprocess.run(
                    [
                        "cargo", "run", "--locked", "-p", "flow-productos", "--example",
                        "contrastar_revision_python", "--", str(paquete), str(salida), desde, hasta, CORTE,
                    ],
                    cwd=ROOT / "rust",
                    check=True,
                )
                carpetas[periodo] = salida
            comprobar_productos(carpetas[periodo], causa, esperado)
    print(f"{len(casos)}/{len(casos)} casos coinciden entre Python, JSON Rust y XLSX Rust")


if __name__ == "__main__":
    verificar()
