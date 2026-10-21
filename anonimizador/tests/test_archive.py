import json
import zipfile
from io import BytesIO

import pytest

from anonimizador_csmp import safezip
from anonimizador_csmp.engine import run
from anonimizador_csmp.textengine import Options
from conftest import make_xlsx, valid_rut


def build(path, files):
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        for name, raw in files.items():
            archive.writestr(name, raw)
    return path


def entrega(tmp_path):
    xlsx = make_xlsx(
        tmp_path / "base.xlsx", [["RIT", "NOMBRE", "OBSERVACION"], ["P-1-2026", "Ana Díaz", "Llamó Ana Díaz"]]
    )
    return build(
        tmp_path / "entrega.zip",
        {
            "INDICE.txt": "VERIFICADO\nArchivos",
            "MANIFIESTO.json": json.dumps(
                {
                    "version": 1,
                    "estado": "VERIFICADO",
                    "archivos": [{"archivo": "Nominas/001_Ana Díaz.xlsx", "bytes": 1, "sha256": "0" * 64}],
                }
            ),
            "Nominas/001_Ana Díaz.xlsx": xlsx.read_bytes(),
            "resumen.json": json.dumps({"estado": "OK", "archivo": "001_Ana Díaz.xlsx", "nombre": "Ana Díaz"}),
            "evidencia.pdf": b"%PDF-1.4 x",
        },
    )


def read_zip(path):
    with zipfile.ZipFile(path) as archive:
        return {n: archive.read(n) for n in archive.namelist()}


def test_entrega_renombra_archivos_con_nombres_y_rehace_manifiesto(tmp_path):
    result = run([entrega(tmp_path)], Options(), tmp_path / "o")
    assert result.ok and any("excluyeron" in w for w in result.report.warnings)
    files = read_zip(result.outputs[0])
    assert not any("Díaz" in n or n.endswith(".pdf") for n in files)
    manifest = json.loads(files["MANIFIESTO.json"])
    assert len(manifest["archivos"]) == 1
    item = manifest["archivos"][0]
    assert item["archivo"] in files and item["bytes"] == len(files[item["archivo"]])
    import hashlib

    assert item["sha256"] == hashlib.sha256(files[item["archivo"]]).hexdigest()
    assert b"Ana" not in files["resumen.json"] and item["nombre"].encode() in files["resumen.json"]
    assert all("Ana" not in v.decode("latin-1") for n, v in files.items() if n.endswith((".json", ".txt")))


def test_zip_con_no_admitidos_falla_completo_si_se_pide(tmp_path):
    result = run([entrega(tmp_path)], Options(skip_unsupported=False), tmp_path / "o")
    assert not result.ok and not result.outputs


def test_zip_solo_con_binarios_se_rechaza(tmp_path):
    path = build(tmp_path / "bin.zip", {"a.pdf": b"%PDF-1.4", "b.png": b"\x89PNG\r\n"})
    result = run([path], Options(), tmp_path / "o")
    assert not result.ok and not result.outputs


def test_zip_anidado(tmp_path):
    inner = build(tmp_path / "interno.zip", {"d.json": json.dumps({"nombre": "Ana Díaz"})})
    outer = build(tmp_path / "externo.zip", {"carpeta/interno.zip": inner.read_bytes()})
    result = run([outer], Options(), tmp_path / "o")
    assert result.ok
    inner_out = read_zip(result.outputs[0])
    nested = next(iter(inner_out.values()))
    assert b"Ana" not in b"".join(read_zip_bytes(nested).values())


def read_zip_bytes(raw):
    with zipfile.ZipFile(BytesIO(raw)) as archive:
        return {n: archive.read(n) for n in archive.namelist()}


def test_rutas_inseguras_y_bombas_se_rechazan():
    buffer = BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        archive.writestr("../fuera.txt", "x")
    with pytest.raises(safezip.UnsafeArchive):
        safezip.members(buffer.getvalue())
    buffer = BytesIO()
    with zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("ceros.txt", b"\x00" * (40 * 1024 * 1024))
    with pytest.raises(safezip.UnsafeArchive):
        safezip.members(buffer.getvalue())
    with pytest.raises(safezip.UnsafeArchive):
        safezip.members(b"PK\x03\x04no es un zip")


def test_nombres_de_archivo_con_datos_se_neutralizan(tmp_path):
    path = build(tmp_path / "z.zip", {f"Nómina {valid_rut()}.json": json.dumps({"estado": "OK"})})
    files = read_zip(run([path], Options(), tmp_path / "o").outputs[0])
    assert not any(valid_rut()[:8] in n for n in files)


def test_xlsx_bomba_de_descompresion_se_rechaza(tmp_path):
    from openpyxl import Workbook

    path = make_xlsx(tmp_path / "b.xlsx", [["RIT", "NOMBRE"], ["P-1-2026", "Ana"]])
    buffer = BytesIO()
    with zipfile.ZipFile(path) as source, zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED) as target:
        for name in source.namelist():
            target.writestr(name, source.read(name))
        target.writestr("xl/relleno.bin", b"\x00" * (30 * 1024 * 1024))
    bomb = tmp_path / "bomba.xlsx"
    bomb.write_bytes(buffer.getvalue())
    result = run([bomb], Options(), tmp_path / "o")
    assert not result.ok and not result.outputs
    _ = Workbook


def test_manifiesto_con_datos_anidados_se_anonimiza(tmp_path):
    # Revisión Codex en el PR #51 (21-oct-2026): los valores anidados del manifiesto se copiaban sin tratar.
    manifiesto = {
        "estado": "VERIFICADO",
        "persona": {"nombre": "Ana Díaz Soto", "rut": valid_rut()},
        "lista": ["Ana Díaz Soto"],
        "archivos": [{"archivo": "datos.csv", "bytes": 10}],
    }
    path = build(
        tmp_path / "lote.zip",
        {"datos.csv": "Nombre,Obs\nPersona Uno,ok\n", "MANIFIESTO.json": json.dumps(manifiesto, ensure_ascii=False)},
    )
    files = read_zip(run([path], Options(), tmp_path / "o").outputs[0])
    texto = files["MANIFIESTO.json"].decode("utf-8")
    assert "Díaz" not in texto and valid_rut() not in texto


def test_formulas_se_buscan_solo_despues_de_validar_el_libro(monkeypatch):
    from anonimizador_csmp import formats

    llamadas = []

    def rechaza(data):
        llamadas.append(len(data))
        raise safezip.UnsafeArchive("bomba")

    monkeypatch.setattr(safezip, "validate", rechaza)
    with pytest.raises(safezip.UnsafeArchive):
        formats.has_formulas(b"PK\x05\x06" + b"\x00" * 18)
    assert llamadas
