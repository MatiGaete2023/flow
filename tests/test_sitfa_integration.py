from datetime import date
from hashlib import sha256
import io
import json
from pathlib import Path

from openpyxl import Workbook
import pytest

from nurus.personal.config import defaults
from nurus.personal.sitfa import inspect_input,startup_arguments,verified_source
from nurus.personal.work import Work
from nurus.rus.reader import read_workbook,WorkbookReadError


def make_source(tmp_path,suffix='.xlsx',missing=False):
    original=tmp_path/'lote'/'pagina.xls';original.parent.mkdir(exist_ok=True);original.write_bytes(b'origen ficticio')
    book=Workbook();sheet=book.active;sheet.title='ESPERA'
    headers=['RIT','TRIBUNAL','NOMBRE','RUT','NOMBRE CENTRO','T ESPERA','SITFA_LOTE','SITFA_ARCHIVO','SITFA_SHA256','SITFA_FECHA_DESCARGA']
    values=['X-1-2026','LAJA','Persona ficticia','11111111-1','AFT FICTICIO',40,'lote',original.name,sha256(original.read_bytes()).hexdigest(),'2026-09-01T09:00:00']
    if missing:headers=headers[:3];values=values[:3]
    sheet.append(headers);sheet.append(values)
    sources=book.create_sheet('SITFA_ARCHIVOS');sources.append(['Carpeta origen','Archivo origen','SHA256']);sources.append([str(original.parent),original.name,sha256(original.read_bytes()).hexdigest()])
    path=tmp_path/('entrada'+suffix);book.save(path);book.close();return path,original


def test_ooxml_with_xls_suffix_is_read_by_content(tmp_path):
    source,_=make_source(tmp_path,'.xls')
    batch=read_workbook(source,'ESPERA');assert len(batch.records)==1
    assert batch.column_mapping['espera']=='T ESPERA'


def test_html_has_specific_conversion_message(tmp_path):
    source=tmp_path/'sitfa.xls';source.write_text('<html><table></table></html>')
    with pytest.raises(WorkbookReadError,match='Exportar para CSMP'):read_workbook(source,'ESPERA')


def test_column_check_and_age_warning(tmp_path):
    source,_=make_source(tmp_path)
    report=inspect_input(source,'ESPERA');assert report['estado']=='Compatible'
    assert any('descarga tiene' in message for message in report['mensajes'])
    source,_=make_source(tmp_path,missing=True)
    report=inspect_input(source,'ESPERA');assert report['estado']=='Sin datos suficientes'
    assert 'programa' in report['faltan'] and 'espera' in report['faltan']


def test_origin_survives_session_and_rejects_mutation(tmp_path):
    source,original=make_source(tmp_path)
    work=Work(defaults()).analyze(source,'ESPERA',as_of=date(2026,10,1))
    assert verified_source(work,work.rows[0])==original.resolve()
    work.save(tmp_path/'sesion');reloaded=Work.load(tmp_path/'sesion')
    assert verified_source(reloaded,reloaded.rows[0])==original.resolve()
    original.write_bytes(b'cambiado')
    with pytest.raises(ValueError,match='hash'):verified_source(reloaded,reloaded.rows[0])


def test_startup_is_explicit_and_invalid_file_rejected(tmp_path):
    source,_=make_source(tmp_path)
    args=startup_arguments(['--archivo',str(source),'--modo','ESPERA','--hoja','ESPERA'])
    assert args.archivo==source.resolve() and args.hoja=='ESPERA'
    with pytest.raises(SystemExit):startup_arguments(['--archivo',str(tmp_path/'ausente.xlsx')])


def test_origin_escape_rejected(tmp_path):
    source,_=make_source(tmp_path);work=Work(defaults()).analyze(source,'ESPERA')
    work.rows[0].values['SITFA_ARCHIVO']='../afuera.xls'
    with pytest.raises(ValueError):verified_source(work,work.rows[0])
