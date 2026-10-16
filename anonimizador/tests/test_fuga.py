"""Búsqueda de fugas: datos sintéticos esparcidos en todos los formatos; ninguno debe sobrevivir."""

import json
import random
import zipfile
from io import BytesIO

import pytest
from openpyxl import load_workbook

from anonimizador_csmp.engine import run
from anonimizador_csmp.textengine import Options
from conftest import make_xlsx, valid_rut

NAMES = ["Valentina", "Cristóbal", "Ximena", "Anselmo", "Pascuala", "Ignacio"]
SURNAMES = ["Quintanilla", "Bahamondes", "Cárcamo", "Urrutia", "Zambrano", "Echeverría"]


def secrets_for(seed):
    rnd = random.Random(seed)
    first, mid, last = rnd.choice(NAMES), rnd.choice(NAMES), rnd.choice(SURNAMES)
    number = rnd.randint(10_000_000, 25_000_000)
    return {
        "nombre": f"{first} {last} {rnd.choice(SURNAMES)}",
        "solo_apellido": last,
        "rut": valid_rut(number),
        "rut_sin_puntos": valid_rut(number, dots=False),
        "fono": f"9{rnd.randint(1000, 9999)}{rnd.randint(1000, 9999)}",
        "mail": f"{first.lower()}.{last.lower()}@correo.cl",
        "otro": f"{mid} {last}",
    }


def flat_text(path):
    """Todo el texto que contiene el archivo, aunque esté comprimido o anidado."""
    raw = path.read_bytes()
    chunks = [raw.decode("utf-8", "ignore")]
    if raw[:2] == b"PK":
        with zipfile.ZipFile(BytesIO(raw)) as archive:
            for name in archive.namelist():
                data = archive.read(name)
                if data[:2] == b"PK":
                    nested = path.with_name("anidado.tmp")
                    nested.write_bytes(data)
                    chunks.append(flat_text(nested))
                else:
                    chunks.append(data.decode("utf-8", "ignore"))
    return "\n".join(chunks)


def forbidden(s):
    return [
        s["nombre"].split()[0],
        s["solo_apellido"],
        s["rut"],
        s["rut"].split("-")[0],
        s["rut_sin_puntos"],
        s["fono"],
        s["mail"],
        s["otro"],
    ]


def assert_clean(path, s, allowed=()):
    text = flat_text(path)
    for secret in forbidden(s):
        assert secret not in text, (secret, path.name)
    assert allowed or True


@pytest.mark.parametrize("seed", range(6))
@pytest.mark.parametrize("mode", ["estricto", "conservador"])
def test_xlsx_con_identidades_dispersas(tmp_path, seed, mode):
    s = secrets_for(seed)
    obs = f"Contactar a {s['otro']} (fono {s['fono']}, {s['mail']}); RUT {s['rut_sin_puntos']}. {s['nombre'].upper()} asistió."
    source = make_xlsx(
        tmp_path / "x.xlsx",
        [
            ["Ficha de " + s["nombre"]],
            ["RIT", "RUT", "NOMBRE", "TRIBUNAL", "OBSERVACION", "DEFENSOR", "CURADOR", "CORREO", "TELEFONO"],
            ["C-55-2026", s["rut"], s["nombre"], "LAJA", obs, s["otro"], f"Abog. {s['otro']}", s["mail"], s["fono"]],
        ],
        title=s["nombre"][:31],
    )
    result = run([source], Options(mode=mode), tmp_path / "o")
    assert result.ok, [i.detail for i in result.items]
    assert_clean(result.outputs[0], s)
    book = load_workbook(result.outputs[0])
    assert s["nombre"] not in book.active.title


@pytest.mark.parametrize("seed", range(4))
def test_json_html_csv_txt_ics(tmp_path, seed):
    s = secrets_for(seed)
    files = {
        "a.json": json.dumps(
            {
                "nombre": s["nombre"],
                "detalle": {"texto": f"{s['otro']} {s['fono']}", "mail": s["mail"], s["nombre"]: 1},
                "ruta": f"C:\\Users\\{s['solo_apellido']}\\x",
            }
        ),
        "b.html": f"<html><head><title>{s['nombre']}</title></head><body><p>Nombre: {s['nombre']}</p><p>{s['otro']} {s['rut']}</p><table><tr><th>RIT</th><th>NOMBRE</th><th>OBSERVACION</th></tr><tr><td>C-5-2026</td><td>{s['nombre']}</td><td>{s['mail']}</td></tr></table><input type='hidden' value='{s['rut']}'><!-- {s['otro']} --></body></html>",
        "c.csv": f"RIT;RUT;NOMBRE;OBSERVACION\nC-5-2026;{s['rut']};{s['nombre']};{s['otro']} {s['fono']}\n",
        "d.txt": f"Nombre: {s['nombre']}\nRUT: {s['rut']}\nLlamar a {s['otro']} {s['mail']}\n",
        "e.ics": f"BEGIN:VCALENDAR\nBEGIN:VEVENT\nSUMMARY:{s['nombre']}\nDESCRIPTION:{s['otro']} {s['fono']}\nATTENDEE:mailto:{s['mail']}\nEND:VEVENT\nEND:VCALENDAR\n",
    }
    paths = []
    for name, content in files.items():
        (tmp_path / name).write_text(content, encoding="utf-8")
        paths.append(tmp_path / name)
    for mode in ("estricto", "conservador"):
        result = run(paths, Options(mode=mode), tmp_path / ("o_" + mode))
        assert result.ok, [i.detail for i in result.items]
        for out in result.outputs:
            assert_clean(out, s)


def test_zip_con_todo_adentro(tmp_path):
    s = secrets_for(99)
    inner = make_xlsx(tmp_path / "i.xlsx", [["RIT", "NOMBRE", "OBSERVACION"], ["C-5-2026", s["nombre"], s["otro"]]])
    with zipfile.ZipFile(tmp_path / "z.zip", "w") as archive:
        archive.write(inner, f"{s['nombre']}/ficha.xlsx")
        archive.writestr("notas/leeme.txt", f"Nombre: {s['nombre']}\n{s['otro']} {s['rut']}\n")
        archive.writestr("datos.json", json.dumps({"autor": s["otro"], "texto": s["mail"]}))
    for mode in ("estricto", "conservador"):
        result = run([tmp_path / "z.zip"], Options(mode=mode), tmp_path / ("o_" + mode))
        assert result.ok
        assert_clean(result.outputs[0], s)
        with zipfile.ZipFile(result.outputs[0]) as archive:
            assert not any(s["solo_apellido"] in n or s["nombre"].split()[0] in n for n in archive.namelist())
