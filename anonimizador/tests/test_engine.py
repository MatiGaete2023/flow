import ast
import hashlib
import os
import threading
from pathlib import Path

from anonimizador_csmp import cli
from anonimizador_csmp.engine import _publish, run
from anonimizador_csmp.textengine import Options
from conftest import make_xlsx

HEAD = ["RIT", "NOMBRE", "OBSERVACION"]


def test_sesion_compartida_entre_archivos(tmp_path):
    a = make_xlsx(tmp_path / "a.xlsx", [HEAD, ["P-1-2026", "Ana Díaz", "x"]])
    b = tmp_path / "b.json"
    b.write_text('{"nombre": "ana diaz", "texto": "Habló Ana"}', encoding="utf-8")
    c = tmp_path / "c.txt"
    c.write_text("Nombre: Luis Mora\n", encoding="utf-8")
    result = run([a, b, c], Options(mode="conservador"), tmp_path / "o")
    assert result.ok and len(result.outputs) == 3
    folder = result.outputs[0].parent
    assert folder.name.startswith("anonimizado_")
    out_json = (folder / result.items[1].output).read_text(encoding="utf-8")
    assert "Persona Ficticia 1" in out_json and "Ana" not in out_json
    report = (folder / "informe_anonimizacion.txt").read_text(encoding="utf-8")
    assert "Díaz" not in report and "a.xlsx" not in report


def test_nombre_de_una_persona_en_otro_archivo_se_detecta(tmp_path):
    """El nombre se conoce por la columna de identidad del Excel y se oculta en la observación de otro archivo."""
    a = make_xlsx(tmp_path / "a.xlsx", [HEAD, ["P-1-2026", "Rosa Contreras", "x"]])
    b = tmp_path / "b.txt"
    b.write_text("Se llamó a Rosa Contreras.\n", encoding="utf-8")
    result = run([a, b], Options(mode="conservador"), tmp_path / "o")
    assert "Contreras" not in (result.outputs[0].parent / result.items[1].output).read_text(encoding="utf-8")


def test_nunca_sobrescribe(tmp_path):
    target = tmp_path / "existe.bin"
    target.write_bytes(b"original")
    try:
        _publish(target, b"nuevo")
    except FileExistsError:
        pass
    else:
        raise AssertionError("sobrescribió")
    assert target.read_bytes() == b"original"
    assert not list(tmp_path.glob(".anon-*"))


def test_sin_enlaces_fisicos_usa_creacion_exclusiva(tmp_path, monkeypatch):
    def no_link(*args, **kwargs):
        raise OSError("sin enlaces")

    monkeypatch.setattr(os, "link", no_link)
    target = tmp_path / "salida.bin"
    _publish(target, b"datos")
    assert target.read_bytes() == b"datos" and not list(tmp_path.glob(".anon-*"))


def test_carpeta_recursiva_ignora_ocultos(tmp_path):
    folder = tmp_path / "lote"
    (folder / "sub").mkdir(parents=True)
    make_xlsx(folder / "sub" / "a.xlsx", [HEAD, ["P-1-2026", "Ana Díaz", "x"]])
    (folder / ".oculto.json").write_text("{}")
    result = run([folder], Options(), tmp_path / "o")
    assert len(result.items) == 1 and result.ok


def test_sin_archivos_y_progreso(tmp_path):
    assert not run([tmp_path / "vacia"], Options()).ok if (tmp_path / "vacia").mkdir() is None else True
    path = make_xlsx(tmp_path / "a.xlsx", [HEAD, ["P-1-2026", "Ana Díaz", "x"]])
    calls = []
    run([path], Options(), tmp_path / "o", lambda label, step, total: calls.append((step, total)))
    assert calls[-1][0] == calls[-1][1]


def test_cli_codigos_de_salida(tmp_path, capsys):
    path = make_xlsx(tmp_path / "a.xlsx", [HEAD, ["P-1-2026", "Ana Díaz", "x"]])
    assert cli.main([str(path), "--salida", str(tmp_path / "o"), "--modo", "conservador", "--semilla", "k"]) == 0
    assert cli.main([str(tmp_path / "no-existe.xlsx")]) == 2
    bad = tmp_path / "x.pdf"
    bad.write_bytes(b"%PDF-1.4")
    assert cli.main([str(bad), "--salida", str(tmp_path / "o")]) == 1
    out = capsys.readouterr().out
    assert "Creado:" in out and "Ana" not in out


def test_la_herramienta_es_independiente_del_asistente_y_del_descargador():
    package = Path(__file__).resolve().parents[1] / "anonimizador_csmp"
    forbidden = {"nurus", "descargador", "motor", "perfiles", "tools"}
    for source in package.glob("*.py"):
        for node in ast.walk(ast.parse(source.read_text(encoding="utf-8"))):
            names = []
            if isinstance(node, ast.Import):
                names = [a.name for a in node.names]
            elif isinstance(node, ast.ImportFrom) and node.level == 0:
                names = [node.module or ""]
            assert not {n.split(".")[0] for n in names} & forbidden, (source.name, names)


def test_original_nunca_cambia(tmp_path):
    path = make_xlsx(tmp_path / "a.xlsx", [HEAD, ["P-1-2026", "Ana Díaz", "x"]])
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    run([path], Options(), tmp_path / "o")
    assert hashlib.sha256(path.read_bytes()).hexdigest() == digest


def test_gui_se_importa_y_construye_si_hay_pantalla():
    tk = __import__("pytest").importorskip("tkinter")
    try:
        root = tk.Tk()
    except tk.TclError:
        __import__("pytest").skip("sin pantalla")
    from anonimizador_csmp.gui import App

    app = App(root)
    app.inputs = []
    app.start()
    assert "al menos un archivo" in app.status.get()
    root.destroy()
    _ = threading


def test_autoprueba_sintetica(tmp_path):
    from anonimizador_csmp.selftest import run_selftest

    report = tmp_path / "informe.json"
    assert run_selftest(report) == 0
    assert '"PASA"' in report.read_text(encoding="utf-8")
    assert run_selftest(report) == 2  # nunca sobrescribe el informe
