"""Contrato 1.1.0 (D6): el ZIP puede llevar solo el XLSX consolidado; todos los datos son ficticios."""

from io import BytesIO
import importlib.util
import json
from pathlib import Path
import stat
import zipfile

import pytest
from openpyxl import Workbook
from nurus.capture import CaptureError, read_capture, write_capture
from nurus.capture.contract import CONSOLIDATED_PATH

spec = importlib.util.spec_from_file_location('fixture', Path(__file__).parents[1] / 'tools/flow_capture_fixture.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def libro(*, extra=None):
    book = Workbook()
    book.active.append(['RIT', 'NOMBRE'])
    book.active.append(['C-1-2026', 'PERSONA FICTICIA'])
    out = BytesIO()
    book.save(out)
    raw = out.getvalue()
    if extra:
        source = zipfile.ZipFile(BytesIO(raw))
        out = BytesIO()
        with zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED) as target:
            for info in source.infolist():
                target.writestr(info.filename, source.read(info))
            for name, data in extra.items():
                target.writestr(name, data)
        raw = out.getvalue()
    return raw


def rewrite(path, change):
    with zipfile.ZipFile(path) as z:
        items = {i.filename: z.read(i) for i in z.infolist()}
    change(items)
    with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as z:
        for name, raw in items.items():
            z.writestr(name, raw)


def test_ida_y_vuelta_1_1_0_y_el_zip_solo_lleva_csv_xlsx_y_manifiesto(tmp_path):
    metadata, tables = module.fixture()
    raw = libro()
    path = write_capture(tmp_path / 'con.zip', metadata, tables, consolidated=raw)
    capture = read_capture(path)
    assert capture.manifest['schema_version'] == '1.1.0'
    assert capture.consolidated == raw
    from jsonschema import Draft202012Validator

    schema = json.loads(
        (Path(__file__).parents[1] / 'contracts/csmp_capture_v1/manifest.schema.json').read_text(encoding='utf-8')
    )
    Draft202012Validator(schema).validate(capture.manifest)
    with zipfile.ZipFile(path) as z:
        names = z.namelist()
    assert CONSOLIDATED_PATH in names
    assert not [n for n in names if n.lower().endswith(('.html', '.htm', '.pdf'))]


def test_paquete_1_0_0_sigue_igual_y_sin_consolidado(tmp_path):
    metadata, tables = module.fixture()
    path = write_capture(tmp_path / 'sin.zip', metadata, tables)
    capture = read_capture(path)
    assert capture.manifest['schema_version'] == '1.0.0'
    assert capture.consolidated is None
    assert CONSOLIDATED_PATH not in zipfile.ZipFile(path).namelist()


@pytest.mark.parametrize(
    'extra',
    [
        {'xl/vbaProject.bin': b'x'},
        {'xl/embeddings/objeto.bin': b'x'},
        {'xl/media/pagina.html': b'<html></html>'},
        {'xl/media/informe.pdf': b'%PDF'},
        {'xl/externalLinks/externalLink1.xml': b'<externalLink/>'},
        {'xl/connections.xml': b'<connections/>'},
        {'xl/queryTables/queryTable1.xml': b'<queryTable/>'},
    ],
)
def test_consolidado_con_macros_incrustados_html_o_pdf_se_rechaza(tmp_path, extra):
    metadata, tables = module.fixture()
    with pytest.raises(CaptureError):
        write_capture(tmp_path / 'malo.zip', metadata, tables, consolidated=libro(extra=extra))
    assert not (tmp_path / 'malo.zip').exists()


def test_consolidado_que_no_es_xlsx_se_rechaza(tmp_path):
    metadata, tables = module.fixture()
    with pytest.raises(CaptureError):
        write_capture(tmp_path / 'malo.zip', metadata, tables, consolidated=b'no es un libro')


def test_manifiesto_1_0_0_no_puede_declarar_consolidado(tmp_path):
    metadata, tables = module.fixture()
    path = write_capture(tmp_path / 'con.zip', metadata, tables, consolidated=libro())

    def bajar_version(items):
        manifest = json.loads(items['manifest.json'])
        manifest['schema_version'] = '1.0.0'
        items['manifest.json'] = json.dumps(manifest).encode()

    rewrite(path, bajar_version)
    with pytest.raises(CaptureError):
        read_capture(path)


def test_hash_alterado_del_consolidado_se_detecta(tmp_path):
    metadata, tables = module.fixture()
    path = write_capture(tmp_path / 'con.zip', metadata, tables, consolidated=libro())
    rewrite(path, lambda items: items.__setitem__(CONSOLIDATED_PATH, libro(extra={'xl/otro.xml': b'<a/>'})))
    with pytest.raises(CaptureError):
        read_capture(path)


def test_consolidado_declarado_con_macros_en_content_types_se_rechaza(tmp_path):
    metadata, tables = module.fixture()
    raw = libro()
    source = zipfile.ZipFile(BytesIO(raw))
    out = BytesIO()
    with zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED) as target:
        for info in source.infolist():
            data = source.read(info)
            if info.filename == '[Content_Types].xml':
                data = data.replace(b'sheet.main+xml', b'sheet.macroEnabled.main+xml')
            target.writestr(info.filename, data)
    with pytest.raises(CaptureError):
        write_capture(tmp_path / 'malo.zip', metadata, tables, consolidated=out.getvalue())


def test_crc_interno_corrupto_no_se_publica_aunque_el_hash_exterior_coincida(tmp_path):
    metadata, tables = module.fixture()
    out = BytesIO()
    with zipfile.ZipFile(BytesIO(libro())) as source, zipfile.ZipFile(out, 'w', zipfile.ZIP_STORED) as target:
        for info in source.infolist():
            target.writestr(info.filename, source.read(info))
    raw = bytearray(out.getvalue())
    with zipfile.ZipFile(BytesIO(raw)) as archive:
        info = archive.getinfo('xl/worksheets/sheet1.xml')
        offset = info.header_offset + 30 + len(info.filename.encode()) + len(info.extra)
    raw[offset + 20] ^= 1
    with pytest.raises(CaptureError, match='corrupto'):
        write_capture(tmp_path / 'malo.zip', metadata, tables, consolidated=bytes(raw))
    assert not (tmp_path / 'malo.zip').exists()


@pytest.mark.parametrize('attack', ['duplicate', 'path', 'symlink', 'compression'])
def test_el_zip_interno_aplica_las_restricciones_del_paquete(tmp_path, attack):
    metadata, tables = module.fixture()
    out = BytesIO(libro())
    with zipfile.ZipFile(out, 'a') as archive:
        if attack == 'duplicate':
            with pytest.warns(UserWarning, match='Duplicate name'):
                archive.writestr('xl/worksheets/sheet1.xml', b'<worksheet/>')
        elif attack == 'path':
            archive.writestr('../fuera.xml', b'<a/>')
        elif attack == 'symlink':
            info = zipfile.ZipInfo('xl/enlace.xml')
            info.external_attr = (stat.S_IFLNK | 0o600) << 16
            archive.writestr(info, b'../fuera.xml')
        else:
            archive.writestr('xl/extra.xml', b'<a/>', compress_type=zipfile.ZIP_BZIP2)
    with pytest.raises(CaptureError):
        write_capture(tmp_path / 'malo.zip', metadata, tables, consolidated=out.getvalue())
    assert not (tmp_path / 'malo.zip').exists()
