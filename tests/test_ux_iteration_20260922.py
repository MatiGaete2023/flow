from pathlib import Path
from types import SimpleNamespace

from nurus.personal.outputs import Draft, draft_fingerprint
from nurus.personal.ux_support import (
    RES_HELP, case_detail_lines, compact_case_detail_text, incident_ids, product_indicators, resume_available, resume_description
)
from nurus.personal.work import Row


def make_work(tmp_path):
    row=Row(
        'r1',2,
        {'RIT':'X-1-2026','NOMBRE':'Persona Ejemplo','TRIBUNAL':'LAJA','DERIVACION':'PRM CENTRO'},
        'Motor original.',[],['PC_INFO'],['Contacto faltante'],False,
        {'OBSERVACION':'Texto final.','TT':1,'CC':0,'RES':'PC_INFO'},
    )
    output=tmp_path/'revisable.xlsx';output.write_bytes(b'copy')
    work=SimpleNamespace(
        rows=[row],mapping={'rit':'RIT','nombre':'NOMBRE','tribunal':'TRIBUNAL','programa':'DERIVACION'},
        output=str(output),receipts={},path=str(tmp_path/'origen.xlsx'),mode='INFORMES',
    )
    return work,row


def test_case_detail_uses_existing_state_and_products_only(tmp_path):
    work,row=make_work(tmp_path)
    labels=dict(case_detail_lines(work,row,[]))
    assert labels['RIT']=='X-1-2026'
    assert labels['NNA']=='Persona Ejemplo'
    assert labels['Observación final']=='Texto final.'
    assert labels['TT']=='1' and labels['CC']=='0' and labels['RES']=='PC_INFO'
    assert labels['Estado']=='Revisar aviso'
    assert 'Excel ✓' in labels['Productos']
    assert 'RES PC_INFO' in labels['Productos'] and 'Word pendiente' in labels['Productos']
    assert 'Modalidad' not in labels


def test_work_compact_detail_omits_duplicate_observation_and_keeps_context(tmp_path):
    work,row=make_work(tmp_path)
    text=compact_case_detail_text(work,row,[])
    assert 'RIT: X-1-2026' in text
    assert 'NNA: Persona Ejemplo' in text
    assert 'Tribunal: LAJA' in text
    assert 'Programa: PRM CENTRO' in text
    assert 'RES: PC_INFO' in text and 'Productos:' in text
    assert 'Incidencias: Contacto faltante' in text
    assert 'Observación final' not in text
    assert 'Texto final.' not in text


def test_incident_navigation_source_is_only_existing_warnings(tmp_path):
    work,row=make_work(tmp_path)
    work.rows.append(Row('r2',3,{},'',[],[],[],False,{}))
    assert incident_ids(work)==['r1']


def test_saved_draft_indicator_uses_effective_draft_identity(tmp_path):
    work,row=make_work(tmp_path)
    draft=Draft('Asunto','Cuerpo',record_ids=[row.id])
    work.receipts[draft_fingerprint(draft)]={'kind':'draft','state':'created'}
    assert 'Correo ✓' in product_indicators(work,row,[draft])


def test_resume_description_uses_existing_session_metadata(tmp_path):
    saved=tmp_path/'trabajo.json';saved.write_text('{}',encoding='utf-8')
    work,row=make_work(tmp_path)
    text=resume_description(work,saved)
    assert 'INFORMES' in text and 'origen.xlsx' in text
    assert resume_available(work)
    Path(work.output).unlink()
    assert not resume_available(work)


def test_res_help_is_minimal_and_operational():
    assert RES_HELP=={
        'PC_IE':'Pide cuenta por ingreso efectivo',
        'PC_INFO':'Pide cuenta por informe',
        'NOMENCL':'Proyecto de nomenclatura',
    }


def test_ui_source_keeps_secondary_actions_and_exposes_requested_hierarchy():
    root=Path(__file__).parents[1]
    app=(root/'src/nurus/personal/app_base.py').read_text(encoding='utf-8')
    mail=(root/'src/nurus/personal/mail_view.py').read_text(encoding='utf-8')
    work=(root/'src/nurus/personal/work_view.py').read_text(encoding='utf-8')
    assert 'Actualizar Excel · F5' in work
    resolutions=(root/'src/nurus/personal/resolution_view.py').read_text(encoding='utf-8')
    assert 'Generar Word' in resolutions and 'Preparar / actualizar proyectos' in resolutions
    assert 'Guardar borradores' in mail and 'Guardar este' in mail and 'Preparar todos' in mail
    assert "text='Básico'" in app and "text='Avanzado'" in app
    assert 'Siguiente aviso' in work
    assert 'CaseDetailPanel' in work
    assert 'CaseDetailPanel' in mail
    assert "tabs.add(changes,text='Cambios')" not in work
    assert "split.add(table,weight=2);split.add(edit,weight=3)" in work
    assert "messagebox.showinfo('Catastro incorporado'" not in app
    assert "✓ Catastro incorporado" in app
