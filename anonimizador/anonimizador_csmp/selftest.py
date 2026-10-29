"""Autoprueba del programa instalado: solo datos sintéticos, sin leer archivos del usuario ni usar la red."""

from __future__ import annotations

import hashlib
import json
import tempfile
import zipfile
from pathlib import Path

from .engine import run
from .textengine import Options, check_digit

NAME = "Valentina Quintanilla Soto"
SURNAME = "Quintanilla"


def _synthetic_rut():
    return f"17.654.321-{check_digit(17654321)}"


def _build(folder):
    from openpyxl import Workbook

    rut = _synthetic_rut()
    book = Workbook()
    sheet = book.active
    sheet.title = "ESPERA"
    sheet.append(["RIT", "RUT", "NOMBRE", "TRIBUNAL", "T ESPERA", "OBSERVACION"])
    sheet.append(["P-432-2026", rut, NAME, "LAJA", 45, f"Se reitera la observación respecto de {NAME}."])
    book.properties.creator = NAME
    excel = folder / "sintetico.xlsx"
    book.save(excel)
    book.close()
    files = {
        "sintetico.csv": f"RIT;NOMBRE;OBSERVACION\nP-1-2026;{NAME};llamar a {NAME}\n",
        "sintetico.json": json.dumps({"nombre": NAME, "estado": "OK", "texto": f"{NAME} {rut}"}),
        "sintetico.html": f"<table><tr><th>RIT</th><th>NOMBRE</th></tr><tr><td>P-2-2026</td><td>{NAME}</td></tr></table>",
        "sintetico.txt": f"Nombre: {NAME}\n",
        "sintetico.ics": f"BEGIN:VCALENDAR\nBEGIN:VEVENT\nSUMMARY:{NAME}\nEND:VEVENT\nEND:VCALENDAR\n",
    }
    for name, content in files.items():
        (folder / name).write_text(content, encoding="utf-8")
    with zipfile.ZipFile(folder / "sintetico.zip", "w") as archive:
        archive.write(excel, f"{NAME}/ficha.xlsx")
        archive.writestr("datos.json", files["sintetico.json"])
        archive.writestr("documento.pdf", b"%PDF-1.4 FICTICIO")
    return sorted(p for p in folder.iterdir() if p.is_file())


def _text_of(path):
    raw = path.read_bytes()
    chunks = [raw.decode("utf-8", "ignore")]
    if raw[:2] == b"PK":
        with zipfile.ZipFile(path) as archive:
            for name in archive.namelist():
                chunks.append(name)
                chunks.append(archive.read(name).decode("utf-8", "ignore"))
    return "\n".join(chunks)


def run_selftest(report_path, gui_factory=None):
    """Devuelve 0 si PASA, 1 si FALLA, 2 si el informe ya existe. El informe solo lleva etiquetas fijas."""
    try:
        report = Path(report_path).open("x", encoding="utf-8")
    except OSError:
        return 2
    checks, status, code = [], "PASA", 0
    try:
        with tempfile.TemporaryDirectory(prefix="csmp-autoprueba-") as temp:
            folder = Path(temp)
            inputs = _build(folder)
            digests = {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in inputs}
            for mode in ("estricto", "conservador"):
                result = run(inputs, Options(mode=mode), folder / f"salida_{mode}")
                assert [i.ok for i in result.items].count(True) == len(inputs), "procesamiento"
                for output in result.outputs:
                    text = _text_of(output)
                    assert NAME not in text and SURNAME not in text and _synthetic_rut() not in text, "fuga"
                checks.append(f"formatos_{mode}")
            assert all(hashlib.sha256(p.read_bytes()).hexdigest() == d for p, d in digests.items())
            checks.append("originales_intactos")
            pdf = folder / "solo.pdf"
            pdf.write_bytes(b"%PDF-1.4 FICTICIO")
            assert not run([pdf], Options(), folder / "salida_pdf").ok
            checks.append("rechazo_binarios")
            if gui_factory is not None:
                root = gui_factory()
                root.update_idletasks()
                root.destroy()
                checks.append("ventana_tk_cargada")
    except Exception:
        # Nunca persistir excepciones o rutas: el informe usa etiquetas fijas.
        status, code = "FALLA", 1
    with report:
        json.dump({"resultado": status, "pruebas": checks, "datos": "exclusivamente sinteticos"}, report, indent=2)
    return code
