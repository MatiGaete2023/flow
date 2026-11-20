"""Compara Python y Flow Rust sobre un mismo ZIP sintético del Descargador."""

from datetime import date
import json
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory


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
        _, filas_python, paquete = _trabajo(carpeta, manifiesto)
        for caso in casos:
            causa = caso["causa_ficticia"]
            desde, hasta = caso["desde"], caso["hasta"]
            python, _ = clasificar(filas_python[causa].revision_bitacora, date.fromisoformat(desde), date.fromisoformat(hasta))
            stdout = subprocess.check_output([str(ejecutable), str(paquete), desde, hasta])
            filas_rust = [f for f in json.loads(stdout) if f["causa"] == causa]
            assert len(filas_rust) == 1, caso
            rust = filas_rust[0]["resultado"]["clase"]
            assert python == rust == caso["esperado"], (caso, python, rust)
            print(f"{desde} a {hasta}, causa ficticia {causa}: {rust}")
    print(f"{len(casos)} casos P2 coinciden entre Python y Flow Rust.")


if __name__ == "__main__":
    main()
