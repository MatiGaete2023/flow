"""EF-01: el bloque de CONTINUAR.md se genera desde las fuentes de estado y la línea base."""

import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("estado_under_test", ROOT / "tools" / "estado.py")
estado = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(estado)

STATE = {
    "rama_canonica": "feat/ficticia",
    "paquetes": {
        "WP-01": {"estado": "INTEGRADO", "titulo": "Uno"},
        "WP-02": {"estado": "BLOQUEADO_USUARIO", "titulo": "Acta | ficticia"},
    },
    "bloqueos_externos": {
        "a": {"estado": "RESUELTO", "motivo": "ya no"},
        "b": {"estado": "ABIERTO", "motivo": "Soporte ficticio"},
    },
}
BASE = {"commit": "0123456789abcdef", "asistente": {"aprobadas": 5, "omitidas": 1, "xfail": 2}, "descargador": None}


def test_bloque_lista_solo_pendientes_y_cifras_de_la_linea_base():
    block = estado.continuation(STATE, BASE)
    assert "`0123456789ab`" in block and "5 aprobadas, 1 omitidas, 2 xfail" in block and "sin medición" in block
    assert "| WP-02 | BLOQUEADO_USUARIO | Acta / ficticia |" in block and "WP-01 |" not in block
    assert "b: Soporte ficticio" in block and "ya no" not in block
    assert estado.continuation(STATE, BASE) == block


def test_render_inserta_y_luego_reemplaza_sin_tocar_el_resto():
    original = "# Título\n\nTexto manual.\n"
    first = estado.render_continue(original, estado.continuation(STATE, BASE))
    assert first.startswith("# Título\n\n" + estado.BLOCK_START) and first.endswith("Texto manual.\n")
    changed = dict(BASE, commit="fedcba9876543210")
    second = estado.render_continue(first, estado.continuation(STATE, changed))
    assert "`fedcba987654`" in second and "`0123456789ab`" not in second
    assert second.count(estado.BLOCK_START) == 1 and second.endswith("Texto manual.\n")
    assert estado.render_continue(second, estado.continuation(STATE, changed)) == second


def test_validar_detecta_bloque_desactualizado(tmp_path, monkeypatch, capsys):
    sources = tmp_path / "estado"
    sources.mkdir()
    (sources / "global.json").write_text(
        '{"version": 1, "conflictos_abiertos": [], "orden_paquetes": [], "rama_canonica": "feat/ficticia"}',
        encoding="utf-8",
    )
    view = tmp_path / "ESTADO.json"
    continuar = tmp_path / "CONTINUAR.md"
    monkeypatch.setattr(estado, "SOURCES", sources)
    monkeypatch.setattr(estado, "VIEW", view)
    monkeypatch.setattr(estado, "CONTINUE", continuar)
    monkeypatch.setattr(estado, "BASELINE", tmp_path / "linea_base.json")
    monkeypatch.setattr(estado, "problems", lambda state: [])
    monkeypatch.setattr(
        estado,
        "aggregate",
        lambda sources=sources: {
            "version": 1,
            "conflictos_abiertos": [],
            "paquetes": {},
            "rama_canonica": "feat/ficticia",
        },
    )
    view.write_text(
        estado.dump({"version": 1, "conflictos_abiertos": [], "paquetes": {}, "rama_canonica": "feat/ficticia"}),
        encoding="utf-8",
    )
    continuar.write_text("# C\n\n" + estado.BLOCK_START + "\nviejo\n" + estado.BLOCK_END + "\n", encoding="utf-8")
    assert estado.main(["validar"]) == 1
    assert "CONTINUAR.md desactualizado" in capsys.readouterr().out
    assert estado.main(["continuacion"]) == 0
    assert estado.main(["validar"]) == 0
