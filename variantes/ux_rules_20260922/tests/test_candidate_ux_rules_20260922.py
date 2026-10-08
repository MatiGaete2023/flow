from pathlib import Path
import json

from nurus.personal.work import actions_for

ROOT=Path(__file__).parents[1]


def _catalog(path):
    return json.loads((ROOT/path).read_text(encoding='utf-8'))


def test_approved_observation_wording_is_synced():
    personal=_catalog('src/nurus/personal/textos_base.json')
    shared=_catalog('src/nurus/rus/textos_observaciones.json')
    expected={
        ('COMUN','MAYORIA_EDAD'):'{PNOMBRE} alcanzó la mayoría de edad el {FECHA_MAYORIA}. Se sugiere egresar la medida.',
        ('COMUN','CURADOR'):'No registra curador ad litem en RUS. Se sugiere asociarlo informáticamente.',
        ('COMUN','OIDO'):'Oído el {FECHA_OIDO}.',
        ('COMUN','PROX_AUDIENCIA'):'Audiencia fijada para el {FECHA_AUDIENCIA}.',
        ('ESPERA','E05_SOLO_CORREO'):'Medida revisada, a la espera de ingreso efectivo. Se remite correo al programa consultando fecha estimada de ingreso.',
        ('CUMPLIMIENTO','C03_INGRESO_RECIENTE'):'Ingreso efectivo al programa {PROGRAMA} registrado el {FECHA_INGRESO}.',
        ('CUMPLIMIENTO','C04_VENCIDA'):'Medida vencida en RUS desde el {FECHA_EGRESO_PROYECTADO}.',
        ('CUMPLIMIENTO','C10_HOJA2'):'Próximo informe de avance de {PROGRAMA} vence el {FECHA_VENCIMIENTO}.',
        ('INFORMES','I01_VENCIDO_GENERAL'):'Se deja constancia de que se remite proyecto de resolución pidiendo cuenta del informe de avance vencido desde el {FECHA_VENCIMIENTO}. De igual manera, se remite correo al programa {PROGRAMA} requiriendo su envío.',
        ('INFORMES','I01_VENCIDO_DCE'):'Se deja constancia de que se remite proyecto de resolución pidiendo cuenta del informe diagnóstico vencido desde el {FECHA_VENCIMIENTO}. De igual manera, se remite correo al programa {PROGRAMA} requiriendo su envío.',
        ('INFORMES','I02_POR_VENCER_GENERAL'):'Se remite correo al programa {PROGRAMA} informando que el próximo informe de avance vence el {FECHA_VENCIMIENTO}.',
        ('INFORMES','I02_POR_VENCER_DCE'):'Se remite correo al programa {PROGRAMA} informando que el informe diagnóstico debe ser remitido a más tardar el {FECHA_VENCIMIENTO}.',
    }
    for (scope,key),text in expected.items():
        assert personal[scope][key]['texto']==text
        assert shared[scope][key]['texto']==text


def test_old_filler_phrases_are_absent_from_approved_catalog():
    catalog=_catalog('src/nurus/personal/textos_base.json')
    changed=[
        catalog['COMUN']['MAYORIA_EDAD']['texto'],
        catalog['COMUN']['PROXIMA_MAYORIA']['texto'],
        catalog['CUMPLIMIENTO']['C03_INGRESO_RECIENTE']['texto'],
        catalog['CUMPLIMIENTO']['C04_VENCIDA']['texto'],
        catalog['CUMPLIMIENTO']['C05_POR_VENCER']['texto'],
        catalog['INFORMES']['I01_VENCIDO_GENERAL']['texto'],
    ]
    joined=' '.join(changed).lower()
    assert 'se hace presente que' not in joined
    assert 'se visualiza' not in joined
    assert 'correo electrónico' not in joined


def test_expired_reports_keep_structured_pc_info_only():
    general=actions_for(['INFORMES.I01_VENCIDO_GENERAL'])
    dce=actions_for(['INFORMES.I01_VENCIDO_DCE'])
    future=actions_for(['INFORMES.I02_POR_VENCER_GENERAL'])
    assert 'PC_INFO' in general and 'programa_vencido' in general
    assert 'PC_INFO' in dce and 'programa_vencido' in dce
    assert 'PC_INFO' not in future and 'programa_por_vencer' in future


def test_compliance_no_longer_inserts_redundant_reviewed_prefix():
    personal=(ROOT/'src/nurus/personal/motor/reglas_cumplimiento.py').read_text(encoding='utf-8')
    shared=(ROOT/'src/nurus/rus/rules.py').read_text(encoding='utf-8')
    assert "render('CUMPLIMIENTO','C09_BASE_BREVE')" not in personal
    assert 'render(catalog, "CUMPLIMIENTO", "C09_BASE_BREVE")' not in shared


def test_ux_contract_is_incremental_not_a_new_main_section():
    app=(ROOT/'src/nurus/personal/app_base.py').read_text(encoding='utf-8')
    mail=(ROOT/'src/nurus/personal/mail_view.py').read_text(encoding='utf-8')
    assert "for name in ('Trabajo','Correos','Resoluciones','Configuración','Enviados')" in app
    assert 'def _move_incident' in app
    assert 'def _case_detail_text' in app
    assert 'ORIGINAL / MOTOR:' in app and 'VERSIÓN EDITADA / FINAL:' in app
    assert 'Procesar / actualizar desde Excel' in app
    assert "text='Generar Word'" in app
    assert 'Básico · Umbrales' in app and 'Avanzado · Observaciones' in app
    assert 'Último trabajo:' in app
    assert "text='Guardar borradores'" in mail
    assert 'dashboard' not in mail.lower()
