from copy import deepcopy
from datetime import date

from openpyxl import Workbook

from nurus.personal.config import Configuration, LEGACY_TEXTS_REV2, TEXT_REVISION, defaults
from nurus.personal.work import Work
from test_personal_flow import source


APPROVED = {
    'COMUN.MAYORIA_EDAD': '{PNOMBRE} alcanzó la mayoría de edad el {FECHA_MAYORIA}. Se sugiere egresar la medida.',
    'COMUN.PROXIMA_MAYORIA': '{PNOMBRE} alcanzará la mayoría de edad el {FECHA_MAYORIA}.',
    'COMUN.CURADOR': 'No registra curador ad litem en RUS. Se sugiere asociarlo informáticamente.',
    'COMUN.OIDO': 'Oído el {FECHA_OIDO}.',
    'COMUN.PROX_AUDIENCIA': 'Audiencia fijada para el {FECHA_AUDIENCIA}.',
    'ESPERA.E04_RESOLUCION_RECIENTE': 'Medida revisada, a la espera de ingreso efectivo. Ingreso al programa {PROGRAMA} ordenado el {FECHA_RESOLUCION}. (Observación administrativa; no requiere gestión del Tribunal).',
    'ESPERA.E05_SOLO_CORREO': 'Medida revisada, a la espera de ingreso efectivo. Se remite correo al programa consultando fecha estimada de ingreso.',
    'ESPERA.E05_PROYECTO_Y_CORREO': 'Medida revisada, a la espera de ingreso efectivo. Se remite proyecto de resolución pidiendo cuenta del ingreso efectivo y correo al programa consultando fecha estimada de ingreso.',
    'ESPERA.E06_SIN_RESOLUCION': 'Medida revisada, a la espera de ingreso efectivo al programa {PROGRAMA}.',
    'CUMPLIMIENTO.C03_INGRESO_RECIENTE': 'Ingreso efectivo al programa {PROGRAMA} registrado el {FECHA_INGRESO}.',
    'CUMPLIMIENTO.C04_VENCIDA': 'Medida vencida en RUS desde el {FECHA_EGRESO_PROYECTADO}.',
    'CUMPLIMIENTO.C05_VENCE_HOY': 'Medida con vencimiento en RUS hoy, {FECHA_EGRESO_PROYECTADO}.',
    'CUMPLIMIENTO.C05_POR_VENCER': 'Medida próxima a vencer en RUS el {FECHA_EGRESO_PROYECTADO}.',
    'CUMPLIMIENTO.C07_SIN_FICHA': 'No registra ficha individual en RUS. Se sugiere confeccionarla.',
    'CUMPLIMIENTO.C07_FICHA_ANTIGUA': 'Ficha individual actualizada por última vez el {FECHA_FICHA_INDIVIDUAL}. Se sugiere actualizarla.',
    'CUMPLIMIENTO.C07_FICHA_RECIENTE': 'Ficha individual actualizada el {FECHA_FICHA_INDIVIDUAL}.',
    'CUMPLIMIENTO.C08_FICHA_FAE': '{PNOMBRE} no registra ficha FAE en RUS. Se sugiere confeccionarla.',
    'CUMPLIMIENTO.C09_SIN_OBSERVACIONES': 'Medida revisada, sin observaciones.',
    'CUMPLIMIENTO.C10_HOJA2': 'Próximo informe de avance de {PROGRAMA} vence el {FECHA_VENCIMIENTO}.',
    'INFORMES.I01_VENCIDO_GENERAL': 'Se deja constancia de que se remite proyecto de resolución pidiendo cuenta del informe de avance vencido desde el {FECHA_VENCIMIENTO}. De igual manera, se remite correo al programa {PROGRAMA} requiriendo su envío.',
    'INFORMES.I01_VENCIDO_DCE': 'Se deja constancia de que se remite proyecto de resolución pidiendo cuenta del informe diagnóstico vencido desde el {FECHA_VENCIMIENTO}. De igual manera, se remite correo al programa {PROGRAMA} requiriendo su envío.',
    'INFORMES.I02_POR_VENCER_GENERAL': 'Se remite correo al programa {PROGRAMA} informando que el próximo informe de avance vence el {FECHA_VENCIMIENTO}.',
    'INFORMES.I02_POR_VENCER_DCE': 'Se remite correo al programa {PROGRAMA} informando que el informe diagnóstico debe ser remitido a más tardar el {FECHA_VENCIMIENTO}.',
}


def flat(catalog):
    return {scope+'.'+key: entry['texto'] for scope, entries in catalog.items()
            if not scope.startswith('_') for key, entry in entries.items()}


def test_approved_wording_is_exact_and_active_catalogs_are_synchronized():
    import json
    from nurus.personal.config import BASE
    personal=json.loads((BASE/'textos_base.json').read_text(encoding='utf-8'))
    rus=json.loads((BASE.parent/'rus'/'textos_observaciones.json').read_text(encoding='utf-8'))
    p=flat(personal);r=flat(rus)
    for key,text in APPROVED.items():
        assert p[key]==text
        assert r[key]==text
    assert p.keys()==r.keys()


def test_text_revision_migrates_only_unchanged_previous_defaults(tmp_path):
    conf=Configuration(tmp_path/'cfg')
    cfg=deepcopy(conf.data)
    cfg['revision_textos']=TEXT_REVISION-1
    cfg['textos']['COMUN']['MAYORIA_EDAD']['texto']=LEGACY_TEXTS_REV2['COMUN.MAYORIA_EDAD']
    custom='Oído específicamente el {FECHA_OIDO}.'
    cfg['textos']['COMUN']['OIDO']['texto']=custom
    conf.save(cfg)

    migrated=Configuration(conf.directory)
    assert migrated.data['revision_textos']==TEXT_REVISION
    assert migrated.data['textos']['COMUN']['MAYORIA_EDAD']['texto']==APPROVED['COMUN.MAYORIA_EDAD']
    assert migrated.data['textos']['COMUN']['OIDO']['texto']==custom


def test_expired_reports_generate_pc_info_but_upcoming_reports_do_not(tmp_path):
    general=Work(defaults()).analyze(source(tmp_path,'INFORMES',program='PRM CENTRO'),'INFORMES',as_of=date(2026,9,13))
    assert general.rows[0].actions==['programa_vencido','PC_INFO']
    assert 'proyecto de resolución pidiendo cuenta del informe de avance vencido' in general.rows[0].observation

    dce_path=source(tmp_path,'INFORMES',program='DCE CENTRO')
    dce=Work(defaults()).analyze(dce_path,'INFORMES',as_of=date(2026,9,13))
    assert dce.rows[0].actions==['programa_vencido','PC_INFO']
    assert 'informe diagnóstico vencido' in dce.rows[0].observation

    upcoming=Work(defaults()).analyze(dce_path,'INFORMES',as_of=date(2026,9,8))
    assert upcoming.rows[0].actions==['programa_por_vencer']
    assert 'PC_INFO' not in upcoming.rows[0].actions
    assert 'debe ser remitido a más tardar el 12 de septiembre de 2026' in upcoming.rows[0].observation


def test_compliance_combines_state_then_milestones_then_suggestion(tmp_path):
    path=tmp_path/'cumplimiento_orden.xlsx'
    book=Workbook();sheet=book.active;sheet.title='Cumplimiento'
    sheet.append([
        'DERIVACION','TRIBUNAL','NOMBRE','RIT','DIAS DE CUMPLIMIENTO','DIAS PARA EGRESAR',
        'FEC.EGRESO PROYECTADO','FEC. OIDO','PROXS. AUDS.','CURADOR'
    ])
    sheet.append(['PRM CENTRO','LAJA','Persona Ejemplo','X-1',100,17,date(2026,9,30),date(2026,9,5),date(2026,10,15),''])
    book.save(path)
    work=Work(defaults()).analyze(path,'CUMPLIMIENTO',as_of=date(2026,9,13))
    text=work.rows[0].observation
    expected=[
        'Medida próxima a vencer en RUS el 30 de septiembre de 2026.',
        'Oído el 5 de septiembre de 2026.',
        'Audiencia fijada para el 15 de octubre de 2026.',
        'No registra curador ad litem en RUS. Se sugiere asociarlo informáticamente.',
    ]
    positions=[text.index(part) for part in expected]
    assert positions==sorted(positions)
    assert 'Medida revisada.' not in text


def test_compliance_without_findings_keeps_reviewed_without_observations(tmp_path):
    path=tmp_path/'cumplimiento_limpio.xlsx'
    book=Workbook();sheet=book.active;sheet.title='Cumplimiento'
    sheet.append(['DERIVACION','TRIBUNAL','NOMBRE','RIT','DIAS DE CUMPLIMIENTO','DIAS PARA EGRESAR','FEC.EGRESO PROYECTADO'])
    sheet.append(['AFT CENTRO','LAJA','Persona Ejemplo','X-1',100,100,date(2026,12,31)])
    book.save(path)
    work=Work(defaults()).analyze(path,'CUMPLIMIENTO',as_of=date(2026,9,13))
    assert work.rows[0].observation.endswith('Medida revisada, sin observaciones.')
