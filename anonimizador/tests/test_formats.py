import json
import zipfile
from io import BytesIO

from anonimizador_csmp.engine import run
from anonimizador_csmp.textengine import OMITTED, Options
from conftest import valid_rut


def one(tmp_path, name, data, **options):
    path = tmp_path / name
    path.write_bytes(data if isinstance(data, bytes) else data.encode("utf-8"))
    result = run([path], Options(**options), tmp_path / "salida")
    assert result.ok, [i.detail for i in result.items]
    return result.outputs[0]


def test_json_anidado_claves_rutas_e_ids(tmp_path):
    data = {
        "carpeta": "C:\\Users\\Pedro González\\Lote 1",
        "estado": "VALIDADA",
        "persona": {"nombre": "Ana Díaz", "rut": valid_rut(), "ingreso_id": "ing-9", "cause_id": "c-1"},
        "registros": [{"RIT": "P-1-2026", "Usuario Centro": "Pedro Soto", "Observación": "Habló con Luis Mora"}],
        "bytes": 120,
        "sha256": "a" * 64,
        "Juan Pérez Soto": 3,
    }
    out = json.loads(one(tmp_path, "resumen.json", json.dumps(data)).read_text(encoding="utf-8"))
    text = json.dumps(out, ensure_ascii=False)
    for secreto in ("González", "Ana Díaz", "Pedro Soto", "Luis Mora", "Juan Pérez", valid_rut()):
        assert secreto not in text
    assert out["estado"] == "VALIDADA" and out["bytes"] == 120 and out["carpeta"] == "Lote 1"
    assert out["persona"]["ingreso_id"].startswith("ing_anon_") and out["sha256"] != "a" * 64
    assert out["registros"][0]["Observación"] == OMITTED


def test_json_conservador_depura_texto(tmp_path):
    out = json.loads(
        one(
            tmp_path,
            "a.json",
            json.dumps({"nombre": "Ana Díaz", "texto": "Llamó Ana Díaz el 05-10-2026"}),
            mode="conservador",
        ).read_text(encoding="utf-8")
    )
    assert out["texto"] == "Llamó Persona Ficticia 1 el 05-10-2026"


def test_json_en_celdas_y_jsonl(tmp_path):
    lines = json.dumps({"nombre": "Ana Díaz"}) + "\n" + json.dumps({"nombre": "Luis Mora"}) + "\n"
    out = one(tmp_path, "datos.jsonl", lines)
    rows = [json.loads(x) for x in out.read_text(encoding="utf-8").splitlines()]
    assert [r["nombre"] for r in rows] == ["Persona Ficticia 1", "Persona Ficticia 2"]
    assert out.suffix == ".jsonl"


def test_json_invalido_se_rechaza(tmp_path):
    path = tmp_path / "roto.json"
    path.write_text("{no es json")
    assert not run([path], Options(), tmp_path / "o").ok


def test_texto_plano_pares_etiqueta(tmp_path):
    out = one(
        tmp_path,
        "nota.txt",
        "Nombre: Ana Díaz\nRUT: " + valid_rut() + "\nObservación libre con Ana Díaz\n",
        mode="conservador",
    )
    text = out.read_text(encoding="utf-8")
    assert "Ana" not in text and "Persona Ficticia 1" in text and valid_rut() not in text


def test_ics_conserva_estructura_y_oculta_datos(tmp_path):
    ics = (
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:abc-123\r\nDTSTART;VALUE=DATE:20261014\r\n"
        "SUMMARY:Vencimiento SITFA · P-432-2026 · Laja\r\nDESCRIPTION:Fuente: x.xls. Llamar a Marta \r\n"
        " Rojas\r\nATTENDEE:mailto:ana@example.com\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
    )
    text = one(tmp_path, "v.ics", ics, mode="conservador").read_text(encoding="utf-8")
    assert "BEGIN:VCALENDAR" in text and "DTSTART;VALUE=DATE:20261014" in text and "UID:abc-123" not in text
    assert "432" not in text and "Rojas" not in text and "ana@example.com" not in text


def make_docx(parts, extra=()):
    buffer = BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        archive.writestr("[Content_Types].xml", "<Types/>")
        for name, xml in parts.items():
            archive.writestr(name, xml)
        for name, raw in extra:
            archive.writestr(name, raw)
    return buffer.getvalue()


def test_docx_parrafos_partidos_en_runs_y_metadatos(tmp_path):
    body = (
        '<w:document xmlns:w="w"><w:body><w:p><w:r><w:t>Se cita a Ana </w:t></w:r><w:r><w:t>Díaz (</w:t></w:r>'
        "<w:r><w:t>P-9-2026</w:t></w:r></w:p></w:body></w:document>"
    )
    core = "<cp:coreProperties><dc:creator>Ana Díaz</dc:creator><cp:lastModifiedBy>Luis</cp:lastModifiedBy></cp:coreProperties>"
    data = make_docx({"word/document.xml": body, "docProps/core.xml": core})
    out = one(tmp_path, "Proyecto.docx", data, mode="conservador")
    with zipfile.ZipFile(out) as archive:
        document = archive.read("word/document.xml").decode()
        assert (
            "Díaz" not in document and "9-2026" not in document and "Persona" not in document or "Ana" not in document
        )
        assert "Ana Díaz" not in archive.read("docProps/core.xml").decode()
        assert "anonimizado" in archive.read("docProps/core.xml").decode()


def test_docx_con_imagenes_se_rechaza(tmp_path):
    data = make_docx({"word/document.xml": "<w:document xmlns:w='w'/>"}, extra=[("word/media/image1.png", b"\x89PNG")])
    path = tmp_path / "foto.docx"
    path.write_bytes(data)
    assert not run([path], Options(), tmp_path / "o").ok
