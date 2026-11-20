"""Genera un ZIP ficticio con las lecturas de contexto de Flow Python."""

from pathlib import Path
import sys


def generar(referencia, carpeta):
    sys.path[:0] = [str(referencia / "descargador"), str(referencia / "src")]
    from descargador import package_current_lot
    from historico_manual import _rut_limpio, capturar
    from lectura_bitacoras import persona_por_rut
    from lectura_historia import LectorHistoria
    from nurus.capture import read_capture
    from perfiles import Selection, selection_profile
    from test_calendario_lectura import _with_calendars
    from test_lectura_historia import FakeBridge, LISTADO, QUERY
    from test_lotes import CATALOG
    from test_paquete_escritos import (
        ESCRITOS_CON_FILAS, HISTORICO_HTML, _persona, con_escritos,
    )
    from test_reutilizacion_local import bound_lot, diaries
    from threading import Event

    carpeta.mkdir(parents=True, exist_ok=True)
    persona = _persona(carpeta)
    # El lote ficticio omite el identificador de persona y conserva un RUT sintético.
    rut = "00111111-1"
    persona.update(rut=rut, person_id=persona_por_rut(_rut_limpio(rut).lstrip("0")))
    historico = HISTORICO_HTML.replace("11.111.111-1", rut)
    capturar(carpeta / "historico", historico, persona, capturada="2026-10-13T12:00:00+00:00")

    # El listado del caso histórico original usa tribunal 49. El lote tiene tribunal 113.
    listado = LISTADO.replace(b"49", b"113").replace(b"C-99999-2099", b"X-1-2026")
    cuerpo = con_escritos(ESCRITOS_CON_FILAS).replace(b"C-99999-2099", b"X-1-2026").replace(b"2099", b"2026")

    class HistoriaFicticia(FakeBridge):
        def call(self, *args, **kwargs):
            return dict(super().call(*args, **kwargs), startedAt="2026-10-13T12:00:00Z")

    seleccion = Selection("113", "Cumplimiento", "2")
    lector = LectorHistoria(HistoriaFicticia(cuerpo), lambda *args: None, Event(), pause=0)
    lector.abrir(carpeta)
    lector.pagina(seleccion, QUERY, 1, listado, selection_profile(seleccion, CATALOG))
    lector.cerrar()
    lote = bound_lot(carpeta / "lote-contexto")
    bitacoras = _with_calendars(
        diaries(carpeta / "bitacoras", 2),
        {"201": (("01/08/2026", "30/07/2026", "ENVIADO"), ("01/11/2026", "", "PENDIENTE"))},
    )
    salida = package_current_lot(
        lote.folder, bitacoras, carpeta / "contexto-e3.zip",
        historia=lector.carpeta / "historias.json", historico=carpeta / "historico",
    )["path"]
    captura = read_capture(salida)
    assert captura.manifest["schema_version"] == "1.2.0"
    assert [e["doc"] for e in captura.rows("escritos")] == ["DOC-1", "DOC-2"]
    assert {e["court_code"] for e in captura.rows("escritos")} == {"113"}
    assert {e["person_id"] for e in captura.rows("historico_proteccion")} == {persona["person_id"]}
    assert any(r["person_id"] == persona["person_id"] for r in captura.rows("registros"))
    assert [(r["ingreso_id"], r["event_at"]) for r in captura.rows("calendarios")] == [
        ("201", "2026-08-01"), ("201", "2026-11-01"),
    ]
    return salida


if __name__ == "__main__":
    referencia, carpeta = map(lambda p: Path(p).resolve(), sys.argv[1:])
    print(generar(referencia, carpeta))
