"""Genera un ZIP ficticio con las lecturas de contexto de Flow Python."""

import hashlib
import json
import sys
from copy import deepcopy
from datetime import date
from io import BytesIO
from pathlib import Path
from threading import Event
from zipfile import ZipFile


def generar_discordancia(captura, salida):
    from nurus.capture import read_capture
    from nurus.capture.contract import write_capture
    from nurus.capture.escritos import _mejores_consultas, evidencia_informe, filas_de_escritos

    metadatos = deepcopy(captura.manifest)
    metadatos.pop("schema_version")
    tablas = {tabla: list(filas) for tabla, filas in captura.tables.items()}
    for pantalla, id_fila, campo, valor, cobertura in (
        ("escritos", "escrito_id", "doc", "DOC-DISTINTO-FICTICIO", "COMPLETA"),
        ("historico_proteccion", "historico_id", "program_name", "Otro programa ficticio", "PARCIAL"),
    ):
        consulta = next(q for q in tablas["consultas"] if q["screen"] == pantalla)
        if pantalla == "escritos":
            i = next(i for i, f in enumerate(tablas[pantalla]) if f["query_id"] == consulta["query_id"])
            tablas[pantalla][i] = dict(tablas[pantalla][i], linked_ingreso_id="201", linked_projection="2026-11-01")
            for i, doc in enumerate(("DOC-1", valor)):
                tablas["historia_tramites"][i] = dict(
                    tablas["historia_tramites"][i],
                    reference=doc, description="Ingreso Informe de Cumplimiento X",
                )
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
    registro = next(r for r in discordante.rows("registros") if r["ingreso_id"] == "201")
    evidencia = evidencia_informe(
        registro, date(2026, 11, 1), filas_de_escritos(discordante, corte=date(2026, 10, 19)),
        discordante.rows("historia_tramites"),
    )
    assert evidencia.resolucion_folio == "" and evidencia.folios_ajenos == ("9119", "9120")


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
    primera = next(f for f in trabajo.rows if trabajo.capture_sources[f.id]["ingreso_id"] == "201")
    assert primera.observation == (
        "Persona PRM: Próximo informe de avance de PRM Centro vence el 1 de noviembre de 2026. "
        "Se hace presente que Persona cuenta con ingreso efectivo al programa PRM Ficticio. "
        "Asimismo, se hace presente que Persona cuenta con ingreso efectivo al programa FAE Otro Ficticio."
    )
    assert "HISTORIA.OTRO_PROGRAMA" in primera.rules
    cronologia = trabajo.capture_context[primera.id]["display"]
    assert cronologia["status"] == "ACREDITADA"
    assert [e["event_at"][:10] for e in cronologia["last_center"]] == ["2026-08-12"]
    generar_discordancia(captura, salida)
    lote_informes = bound_lot(carpeta / "lote-informes", "Informes")
    informe = package_current_lot(
        lote.folder, bitacoras, carpeta / "contexto-e3-informes.zip", extra_folders=[lote_informes.folder],
    )["path"]
    trabajo_informes = import_capture(informe, defaults(), as_of=date(2026, 10, 19))
    avisos = [
        a for f in trabajo_informes.rows
        if trabajo_informes.capture_sources[f.id]["record_type"] == "INFORMES"
        for a in f.warnings if a.startswith("CAL-03")
    ]
    assert avisos == [(
        "CAL-03: la pestaña Informes indica vencimiento el 10 de octubre de 2026 y el calendario del ingreso "
        "proyecta el informe pendiente el 1 de noviembre de 2026; revisar en RUS."
    )]
    texto = "Próximo informe de avance de PRM Centro vence el 1 de noviembre de 2026."
    repetida = _with_calendars(
        diaries(carpeta / "bitacora-reiteracion", text=texto),
        {"201": (("01/11/2026", "", "PENDIENTE"),)},
    )
    paquete_reiteracion = package_current_lot(
        lote.folder, repetida, carpeta / "contexto-e3-reiteracion.zip",
    )["path"]
    reiteracion = import_capture(paquete_reiteracion, defaults(), as_of=date(2026, 10, 19))
    primera, segunda = reiteracion.rows
    assert primera.observation == "Persona PRM: Se reitera observación anterior del 12 de agosto de 2026."
    assert "COMUN.REITERA_OBSERVACION" in primera.rules
    assert "COMUN.REITERA_OBSERVACION" not in segunda.rules

    # El LectorHistoria y el exportador producen el folio; la fecha oficial viene de la nómina de Espera.
    from openpyxl import load_workbook
    from resultados import load_lot

    lote_espera = bound_lot(carpeta / "lote-espera", "Espera")
    verificacion = lote_espera.folder / "verificacion 001.json"
    manifiesto = json.loads(verificacion.read_text())
    archivo = manifiesto["archivos"][0]
    ruta = lote_espera.folder / archivo["archivo"]
    libro = load_workbook(BytesIO(ruta.read_bytes()))
    hoja = libro.active
    hoja["F1"] = "FEC. RESOLUCION"
    hoja["F2"] = date(2026, 10, 13)
    hoja["D2"] = "DCE Ficticio"
    archivo["vinculos_ingreso"][0]["programa"] = "DCE Ficticio"
    libro.save(ruta)
    libro.close()
    datos = ruta.read_bytes()
    archivo.update(sha256=hashlib.sha256(datos).hexdigest(), bytes=len(datos))
    verificacion.write_text(json.dumps(manifiesto), encoding="utf-8")
    lote_espera = load_lot(lote_espera.folder)
    pantalla = cuerpo.replace(b"Tengase presente canario", "Ing.Serv. protección especializada y otras redes".encode("cp1252"))
    # La historia se lee por causa, aunque la nómina de Trabajo sea de otra etapa.
    lector_espera = LectorHistoria(HistoriaFicticia(pantalla), lambda *args: None, Event(), pause=0)
    carpeta_espera = carpeta / "lectura-espera"
    carpeta_espera.mkdir()
    lector_espera.abrir(carpeta_espera)
    lector_espera.pagina(seleccion, QUERY, 1, listado, selection_profile(seleccion, CATALOG))
    lector_espera.cerrar()
    espera = package_current_lot(
        lote_espera.folder, bitacoras, carpeta / "contexto-e3-espera.zip",
        historia=lector_espera.carpeta / "historias.json",
    )["path"]
    espera = import_capture(espera, defaults(), as_of=date(2026, 10, 19))
    assert espera.rows[0].observation == (
        "Persona DCE: Medida revisada, a la espera de ingreso efectivo. "
        "Ingreso al programa DCE Ficticio ordenado en resolución de fecha 13 de octubre de 2026 (folio 9120). "
        "Se remite correo al programa consultando fecha estimada de ingreso."
    )
    assert espera.rows[0].rules == ["HISTORIA.E04_CON_FOLIO", "ESPERA.E05_SOLO_CORREO"]
    return salida


if __name__ == "__main__":
    referencia, carpeta = (Path(ruta).resolve() for ruta in sys.argv[1:])
    print(generar(referencia, carpeta))
