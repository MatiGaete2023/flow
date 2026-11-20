"""Genera un ZIP ficticio con las lecturas de contexto de Flow Python."""

import sys
from copy import deepcopy
from datetime import date
from pathlib import Path
from threading import Event
from zipfile import ZipFile


def generar_discordancia(captura, salida):
    from nurus.capture import read_capture
    from nurus.capture.contract import write_capture
    from nurus.capture.escritos import _mejores_consultas

    metadatos = deepcopy(captura.manifest)
    metadatos.pop("schema_version")
    tablas = {tabla: list(filas) for tabla, filas in captura.tables.items()}
    for pantalla, id_fila, campo, valor, cobertura in (
        ("escritos", "escrito_id", "doc", "DOC-DISTINTO-FICTICIO", "COMPLETA"),
        ("historico_proteccion", "historico_id", "program_name", "Otro programa ficticio", "PARCIAL"),
    ):
        consulta = next(q for q in tablas["consultas"] if q["screen"] == pantalla)
        filas = [f for f in tablas[pantalla] if f["query_id"] == consulta["query_id"]]
        assert filas
        id_copia = consulta["query_id"] + "-copia"
        # El mismo instante se expresa en UTC en la copia y con otro huso en Escritos.
        fecha = "2026-10-13T12:00:00Z"
        copia = dict(consulta, query_id=id_copia, captured_at=fecha, coverage=cobertura)
        tablas["consultas"].append(copia)
        nuevas = filas if cobertura == "COMPLETA" else filas[:1]
        for i, f in enumerate(nuevas):
            nuevas_filas = {id_fila: f[id_fila] + "-copia"}
            if i == 0:
                nuevas_filas[campo] = valor
            tablas[pantalla].append(dict(f, **nuevas_filas, query_id=id_copia, captured_at=fecha, coverage=cobertura))
        if cobertura == "PARCIAL":
            metadatos["table_states"][pantalla] = "PARCIAL"
    metadatos["queries"] = tablas["consultas"]
    with ZipFile(salida) as original:
        originales = {
            f["path"]: original.read(f["path"]) for f in metadatos["files"] if f["type"] == "original"
        }
    discordante = read_capture(write_capture(
        salida.with_name("contexto-e3-discordante.zip"), metadatos, tablas,
        originals=originales, consolidated=captura.consolidated,
    ))
    for pantalla, filtro in (("escritos", "escritos_causa"), ("historico_proteccion", "historico_persona")):
        consultas = _mejores_consultas(discordante, pantalla, filtro, corte=date(2026, 10, 19))
        assert len(consultas) == 1 and next(iter(consultas.values()))["_discordante"]


def generar(referencia, carpeta):
    sys.path[:0] = [str(referencia / "descargador"), str(referencia / "src")]
    from descargador import package_current_lot
    from historico_manual import _rut_limpio, capturar
    from lectura_bitacoras import persona_por_rut
    from lectura_historia import LectorHistoria
    from nurus.capture import read_capture
    from nurus.personal.capture_input import import_capture
    from nurus.personal.config import defaults
    from perfiles import Selection, selection_profile
    from test_calendario_lectura import _with_calendars
    from test_lectura_historia import LISTADO, QUERY, FakeBridge
    from test_lotes import CATALOG
    from test_paquete_escritos import (
        ESCRITOS_CON_FILAS,
        HISTORICO_HTML,
        _persona,
        con_escritos,
    )
    from test_reutilizacion_local import bound_lot, diaries

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
    salida = Path(package_current_lot(
        lote.folder, bitacoras, carpeta / "contexto-e3.zip", consolidated=True,
        historia=lector.carpeta / "historias.json", historico=carpeta / "historico",
    )["path"])
    captura = read_capture(salida)
    assert captura.manifest["schema_version"] == "1.2.0"
    assert [e["doc"] for e in captura.rows("escritos")] == ["DOC-1", "DOC-2"]
    assert {e["court_code"] for e in captura.rows("escritos")} == {"113"}
    assert {e["person_id"] for e in captura.rows("historico_proteccion")} == {persona["person_id"]}
    assert any(r["person_id"] == persona["person_id"] for r in captura.rows("registros"))
    assert [(r["ingreso_id"], r["event_at"]) for r in captura.rows("calendarios")] == [
        ("201", "2026-08-01"), ("201", "2026-11-01"),
    ]
    trabajo = import_capture(salida, defaults(), as_of=date(2026, 10, 19))
    fechas = {
        trabajo.capture_sources[f.id]["ingreso_id"]: trabajo.cross_dates.get(f.id)
        for f in trabajo.rows
    }
    assert str(fechas["201"]) == "2026-11-01" and fechas["202"] is None
    generar_discordancia(captura, salida)
    return salida


if __name__ == "__main__":
    referencia, carpeta = (Path(ruta).resolve() for ruta in sys.argv[1:])
    print(generar(referencia, carpeta))
