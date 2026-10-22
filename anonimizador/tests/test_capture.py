import csv
import json
import zipfile
from hashlib import sha256
from io import StringIO

import pytest

from anonimizador_csmp import capture
from anonimizador_csmp.engine import run
from anonimizador_csmp.textengine import Options
from conftest import make_xlsx, valid_rut

TIME = "2026-10-08T15:00:00-03:00"


def row(table, **values):
    base = {key: "" for key in capture.TABLES[table]}
    base.update(values)
    return base


def build_capture(path, *, originals=None, consolidated=None, bitacora_text=None):
    """Paquete CSMP_CAPTURE_V1 sintético construido sin usar el Asistente."""
    cid = "ficticia-1"
    common = dict(
        capture_id=cid,
        query_id="q1",
        source_system="RUS",
        court_code="FICTICIO",
        court_name="Laja",
        cause_id="causa-1",
        ingreso_id="ing-1",
        person_id="per-1",
        program_id="prog-1",
        modality_code="1",
        modality_raw="1",
        tab="Espera",
        captured_at=TIME,
        original_file="fuente.xls",
        original_sheet="Hoja1",
        original_row="2",
        source_sha256="a" * 64,
        coverage="COMPLETA",
        rit_original="P-77-2026",
    )
    query = row(
        "consultas",
        query_id="q1",
        source_system="RUS",
        screen="seguimiento",
        tab="Espera",
        court_code="FICTICIO",
        court_name="Laja",
        modality_code="1",
        coverage="COMPLETA",
        captured_at=TIME,
        filters_json=json.dumps(
            {
                "remote_identity": {
                    "source_system": "RUS",
                    "court_code": "FICTICIO",
                    "cause_id": "causa-1",
                    "ingreso_id": "ing-1",
                },
                "capture_tables": ["registros", "bitacoras"],
            }
        ),
        expected_pages="1",
        saved_pages="1",
        time_zone="America/Santiago",
        history_complete="false",
        institutional_verified="false",
    )
    raw = {"RIT": "P-77-2026", "RUT": valid_rut(), "NOMBRE": "Ana Díaz Soto", "TRIBUNAL": "Laja", "T ESPERA": "40"}
    tables = {
        "consultas": [query],
        "registros": [
            row(
                "registros",
                **common,
                record_id="r1",
                record_type="ESPERA",
                identity_status="VERIFICADA",
                raw_json=json.dumps(raw, ensure_ascii=False),
                normalized_json=json.dumps(raw, ensure_ascii=False),
            )
        ],
        "bitacoras": [
            row(
                "bitacoras",
                **common,
                event_id="e1",
                event_at="2026-09-10",
                time_precision="DIA",
                time_zone="DESCONOCIDA",
                origin="CENTRO",
                origin_verified="true",
                author="Pedro Mora",
                event_type="Observación",
                text=bitacora_text or "Habló con Ana Díaz Soto y con Luis Pinto",
                identity_status="VERIFICADA",
            )
        ],
        "incidencias": [],
    }
    payload, files = {}, []
    for name, rows in tables.items():
        stream = StringIO(newline="")
        writer = csv.DictWriter(stream, fieldnames=capture.TABLES[name], lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
        data = stream.getvalue().encode()
        payload[f"data/{name}.csv"] = data
        files.append(
            {
                "path": f"data/{name}.csv",
                "table": name,
                "type": "normalized_table",
                "rows": len(rows),
                "bytes": len(data),
                "sha256": sha256(data).hexdigest(),
            }
        )
    for name, data in (originals or {}).items():
        payload[name] = data
        files.append({"path": name, "type": "original", "bytes": len(data), "sha256": sha256(data).hexdigest()})
    if consolidated:
        payload[capture.CONSOLIDATED_PATH] = consolidated
        files.append(
            {
                "path": capture.CONSOLIDATED_PATH,
                "type": "consolidated_workbook",
                "bytes": len(consolidated),
                "sha256": sha256(consolidated).hexdigest(),
            }
        )
    states = {t: "NO_SOLICITADA" for t in capture.TABLES}
    states.update(consultas="COMPLETA", registros="COMPLETA", bitacoras="COMPLETA", incidencias="VACIA_COMPROBADA")
    manifest = {
        "schema_id": "CSMP_CAPTURE",
        "schema_version": "1.1.0" if consolidated else "1.0.0",
        "capture_id": cid,
        "captured_at": TIME,
        "downloader": {"version": "x", "commit": "y"},
        "contains_sensitive_data": True,
        "originals_authorized": bool(originals),
        "queries": [query],
        "table_states": states,
        "mapping_version": "baseline-20261008-v1",
        "required_features": [],
        "files": files,
    }
    payload["manifest.json"] = json.dumps(manifest).encode()
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        for name, data in payload.items():
            archive.writestr(name, data)
    return path


def tables_of(path):
    manifest, tables, consolidated, originals = capture.parse(
        __import__("anonimizador_csmp.safezip", fromlist=["x"]).members(path.read_bytes())
    )
    return manifest, {k: v[1] for k, v in tables.items()}, consolidated, originals


@pytest.mark.parametrize("mode", ["estricto", "conservador"])
def test_captura_coherente_y_sin_datos(tmp_path, mode):
    consolidated = make_xlsx(
        tmp_path / "c.xlsx",
        [
            ["RIT", "NOMBRE", "T ESPERA", "SITFA_CAUSA_RUS", "SITFA_FILA"],
            ["P-77-2026", "Ana Díaz Soto", 40, "causa-1", 2],
        ],
    ).read_bytes()
    source = build_capture(tmp_path / "cap.zip", consolidated=consolidated)
    result = run([source], Options(mode=mode), tmp_path / "o")
    assert result.ok and result.items[0].kind == "captura"
    manifest, tables, new_consolidated, _ = tables_of(result.outputs[0])
    blob = json.dumps(tables, ensure_ascii=False)
    for secreto in ("Díaz", "Soto", "Pinto", "Mora", "P-77-2026", valid_rut()):
        assert secreto not in blob
    assert manifest["capture_id"] != "ficticia-1" and manifest["downloader"]["commit"] == "COPIA_LOCAL"
    assert manifest["queries"] == tables["consultas"]
    reg, bit = tables["registros"][0], tables["bitacoras"][0]
    assert reg["cause_id"] == bit["cause_id"] != "causa-1" and reg["court_code"] == "FICTICIO"
    assert json.loads(tables["consultas"][0]["filters_json"])["remote_identity"]["cause_id"] == reg["cause_id"]
    assert json.loads(reg["raw_json"])["T ESPERA"] == "40"
    from openpyxl import load_workbook
    from io import BytesIO

    sheet = load_workbook(BytesIO(new_consolidated)).active
    assert sheet["D2"].value == reg["cause_id"] and sheet["E2"].value == 2
    if mode == "estricto":
        assert bit["text"] == "[texto omitido]"
    else:
        assert "Persona Ficticia" in bit["text"] or "[identidad omitida]" in bit["text"]


def test_originales_no_anonimizables_se_excluyen_y_filas_quedan_no_disponibles(tmp_path):
    source = build_capture(tmp_path / "cap.zip", originals={"originales/doc.pdf": b"%PDF-1.4 x"})
    result = run([source], Options(), tmp_path / "o")
    assert result.ok and any("excluyeron" in w for w in result.report.warnings)
    manifest, _, _, originals = tables_of(result.outputs[0])
    assert not originals and manifest["originals_authorized"] is False


def test_originales_no_admitidos_pueden_rechazar_el_paquete(tmp_path):
    source = build_capture(tmp_path / "cap.zip", originals={"originales/doc.pdf": b"%PDF-1.4 x"})
    result = run([source], Options(skip_unsupported=False), tmp_path / "o")
    assert not result.ok and not result.outputs


def test_original_tabular_se_anonimiza_y_se_reincluye(tmp_path):
    xlsx = make_xlsx(tmp_path / "o.xlsx", [["RIT", "NOMBRE"], ["P-77-2026", "Ana Díaz Soto"]]).read_bytes()
    source = build_capture(tmp_path / "cap.zip", originals={"originales/l1/fuente.xlsx": xlsx})
    result = run([source], Options(), tmp_path / "o")
    manifest, _, _, originals = tables_of(result.outputs[0])
    ((path, data),) = originals.items()
    assert path.startswith("originales/") and b"Soto" not in data and manifest["originals_authorized"] is True


def test_manifiesto_alterado_se_rechaza(tmp_path):
    source = build_capture(tmp_path / "cap.zip")
    with zipfile.ZipFile(source) as archive:
        files = {n: archive.read(n) for n in archive.namelist()}
    files["data/registros.csv"] += b"\n"
    broken = tmp_path / "roto.zip"
    with zipfile.ZipFile(broken, "w") as archive:
        for n, d in files.items():
            archive.writestr(n, d)
    result = run([broken], Options(), tmp_path / "o")
    assert not result.ok and not result.outputs


def test_compatible_con_el_lector_del_asistente(tmp_path):
    reader = pytest.importorskip("nurus.capture", reason="el Asistente es opcional: la herramienta no depende de él")
    source = build_capture(
        tmp_path / "cap.zip",
        consolidated=make_xlsx(tmp_path / "c.xlsx", [["RIT", "NOMBRE"], ["P-77-2026", "Ana"]]).read_bytes(),
    )
    result = run([source], Options(mode="conservador"), tmp_path / "o")
    assert result.ok
    parsed = reader.read_capture(result.outputs[0])
    assert len(parsed.rows("bitacoras")) == 1 and parsed.consolidated
