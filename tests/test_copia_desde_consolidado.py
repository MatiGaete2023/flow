"""D6: con consolidado en el paquete, la copia del Asistente parte de ese libro; sin él, queda como antes."""

from datetime import date
from io import BytesIO
from pathlib import Path
from openpyxl import Workbook, load_workbook
from nurus.capture import write_capture
from nurus.personal.capture_input import import_capture
from nurus.personal.config import defaults
from nurus.personal.work import _consolidated_base
from test_capture_contract import fixture

ORIGIN = (
    'SITFA_LOTE',
    'SITFA_CONSULTA',
    'SITFA_ARCHIVO',
    'SITFA_PAGINA',
    'SITFA_FILA',
    'SITFA_SHA256',
    'SITFA_FECHA_DESCARGA',
)


def consolidado(registros, *, hojas=1, invertir=True, duplicar=False):
    book = Workbook()
    resumen = book.active
    resumen.title = 'Resumen'
    resumen.append(['Resultado'])
    destinos = [book.create_sheet(f'Datos{i}') for i in range(hojas)]
    for sheet in destinos:
        sheet.append(['RIT', *ORIGIN])
    lista = list(reversed(registros)) if invertir else list(registros)
    for i, r in enumerate(lista):
        destinos[i % hojas].append(
            [r['rit_original'], '', '', r['original_file'], '1', r['original_row'], r['source_sha256'], '']
        )
    if duplicar:
        r = lista[0]
        destinos[0].append(
            [r['rit_original'], '', '', r['original_file'], '1', r['original_row'], r['source_sha256'], '']
        )
    out = BytesIO()
    book.save(out)
    return out.getvalue()


def importar(tmp_path, **opciones):
    metadata, tables = fixture()
    raw = consolidado(tables['registros'], **opciones)
    path = write_capture(tmp_path / 'con.zip', metadata, tables, consolidated=raw)
    return import_capture(path, defaults(), as_of=date(2026, 10, 8)), tables, raw


def test_la_base_apunta_a_la_hoja_y_a_las_filas_del_consolidado(tmp_path):
    work, tables, raw = importar(tmp_path)
    base = _consolidated_base(work)
    assert base and base[0] == raw and base[1] == 'Datos0'
    by_id = {r['record_id']: r for r in tables['registros']}
    book = load_workbook(BytesIO(raw))
    sheet = book['Datos0']
    encabezado = [c.value for c in sheet[1]]
    col_fila = encabezado.index('SITFA_FILA')
    for record_id, numero in base[2].items():
        assert str(sheet.cell(numero, col_fila + 1).value) == by_id[record_id]['original_row']


def test_con_registros_en_varias_hojas_o_duplicados_no_hay_base_y_queda_portable(tmp_path):
    for nombre, opciones in (('hojas', {'hojas': 2}), ('duplicados', {'duplicar': True})):
        carpeta = tmp_path / nombre
        carpeta.mkdir()
        work, _, _ = importar(carpeta, **opciones)
        assert _consolidated_base(work) is None
        destino = carpeta / 'copia.xlsx'
        work.export(destino)
        assert load_workbook(destino).sheetnames[0] == 'Trabajo'


def test_paquete_sin_consolidado_sigue_igual(tmp_path):
    metadata, tables = fixture()
    work = import_capture(write_capture(tmp_path / 'sin.zip', metadata, tables), defaults(), as_of=date(2026, 10, 8))
    assert _consolidated_base(work) is None
    destino = tmp_path / 'copia.xlsx'
    work.export(destino)
    assert load_workbook(destino).sheetnames[0] == 'Trabajo'


def test_reimportar_el_mismo_paquete_movido_conserva_la_base_y_ediciones(tmp_path):
    previous, _, raw = importar(tmp_path)
    old_path = Path(previous.path)
    new_path = old_path.rename(tmp_path / 'movido.zip')
    previous.rows[0].review['OBSERVACION'] = 'Edición humana ficticia'
    restored = import_capture(new_path, defaults(), as_of=date(2026, 10, 8), previous=previous)
    assert restored.path == str(new_path.resolve())
    assert previous.path == str(old_path.resolve())
    assert restored.rows[0].review == previous.rows[0].review
    base = _consolidated_base(restored)
    assert base and base[0] == raw
