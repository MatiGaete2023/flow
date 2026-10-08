from copy import deepcopy
from datetime import date
from pathlib import Path
import sys
import json
import pytest
from openpyxl import load_workbook
from nurus.capture import write_capture,CaptureError
from nurus.personal.capture_input import import_capture
from nurus.personal.config import defaults
from nurus.personal.work import Work
from test_capture_contract import fixture,mutate


def test_four_modalities_edits_reimport_and_recovery(tmp_path):
    m,t=fixture();path=write_capture(tmp_path/'full.zip',m,t)
    before_modules=set(sys.modules)
    work=import_capture(path,defaults(),as_of=date(2026,10,8))
    assert len(work.rows)==4 and all(r.observation for r in work.rows)
    assert not work.activity_reviewed and not work.signed_activity
    work.rows[0].review={'OBSERVACION':'Texto humano','TT':'T','CC':1,'RES':'PC_IE'}
    work.rows[0].decisions['resolution']='PC_IE'
    restored=import_capture(path,defaults(),as_of=date(2026,10,8),previous=work)
    assert restored.rows[0].review==work.rows[0].review
    assert all(ctx['class']=='NO_EVALUABLE' for ctx in restored.capture_context.values())
    restored.save(tmp_path/'session');loaded=Work.load(tmp_path/'session')
    assert loaded.capture_digest==work.capture_digest and loaded.rows[0].review['OBSERVACION']=='Texto humano'
    assert not any(name in set(sys.modules)-before_modules for name in ('nurus.personal.download_link','nurus.personal.bitacora_link'))


def test_corrupt_import_preserves_previous_and_separates_ingresos(tmp_path):
    m,t=fixture();path=write_capture(tmp_path/'full.zip',m,t)
    previous=import_capture(path,defaults());previous.rows[0].review['OBSERVACION']='Edición'
    before=deepcopy(previous.__dict__)
    mutate(path,lambda items:items.update({'escape':b'bad'}))
    with pytest.raises(CaptureError):import_capture(path,defaults(),previous=previous)
    assert previous.__dict__==before
    for rid,record in previous.capture_sources.items():
        assert all(e['ingreso_id']==record['ingreso_id'] for e in previous.capture_context[rid]['recent'])


def test_export_and_refresh_portable_preserve_human_fields(tmp_path):
    m,t=fixture();path=write_capture(tmp_path/'full.zip',m,t)
    work=import_capture(path,defaults());work.rows[0].review={'OBSERVACION':'Humano','TT':'1','CC':'2','RES':'PC_IE'}
    work.export(tmp_path/'out.xlsx',backend='portable',reduced_fidelity=True)
    book=load_workbook(work.output);sheet=book['Trabajo'];headers={c.value:c.column for c in sheet[1]}
    assert sheet.cell(2,headers['OBSERVACION']).value=='Humano'
    assert sheet.cell(2,headers['CC']).value=='2'
    book.close();assert work.refresh() is False


def test_reanalysis_new_cut_preserves_human_decisions_and_invalidates_product(tmp_path):
    from nurus.personal.outputs import Draft
    from nurus.personal.product_state import stamp,stale
    m,t=fixture();path=write_capture(tmp_path/'full.zip',m,t)
    old=import_capture(path,defaults(),as_of=date(2026,10,8));old.rows[0].review['OBSERVACION']='Humano'
    draft=Draft('Ficticio','Ficticio',kind='programa_espera',record_ids=[old.rows[0].id]);stamp(old,draft)
    new=import_capture(path,defaults(),as_of=date(2026,10,9),previous=old)
    assert new.rows[0].review['OBSERVACION']=='Humano' and stale(new,draft)
    assert old.as_of=='2026-10-08'


def test_mixed_modes_export_refresh_and_individual_recalculation(tmp_path):
    m,t=fixture()
    for index,mode in enumerate(('ESPERA','CUMPLIMIENTO','INFORMES','ESPERA')):
        r=t['registros'][index];raw=json.loads(r['normalized_json'])
        if mode=='CUMPLIMIENTO':raw.update({'DIAS DE CUMPLIMIENTO':'50','DIAS PARA EGRESAR':'10','FEC.EGRESO PROYECTADO':'2026-10-18'})
        if mode=='INFORMES':raw['FECHA VENCIMIENTO']='2026-10-15'
        r.update(record_type=mode,raw_json=json.dumps(raw),normalized_json=json.dumps(raw))
    work=import_capture(write_capture(tmp_path/'mix.zip',m,t),defaults(),as_of=date(2026,10,8))
    assert any(rule.startswith('CUMPLIMIENTO.') for rule in work.rows[1].rules)
    assert any(rule.startswith('INFORMES.') for rule in work.rows[2].rules)
    work.export(tmp_path/'mix.xlsx',backend='portable',reduced_fidelity=True)
    book=load_workbook(work.output);sheet=book['Trabajo'];headers={c.value:c.column for c in sheet[1]}
    sheet.cell(work.rows[2].source_row,headers['OBSERVACION'],'Edición informe');book.save(work.output);book.close()
    assert work.refresh() is True
    assert 'vencimiento' in work.mapping and 'dias_egresar' in work.mapping
    from nurus.personal.record_edits import recalculate
    recalculate(work,work.rows[2])
    assert any(rule.startswith('INFORMES.') for rule in work.rows[2].rules)
    assert work.rows[2].review['OBSERVACION']=='Edición informe'
