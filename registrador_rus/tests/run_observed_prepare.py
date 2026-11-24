"""Preparación simulada en listado y popup separados; sin acceso institucional."""
from pathlib import Path
from hashlib import sha256
import os
from playwright.sync_api import sync_playwright

ROOT=Path(__file__).resolve().parents[1]
SCRIPTS=[(ROOT/'extension'/p).read_text(encoding='utf-8') for p in ('prepare.js','observed_form.js','observed_prepare.js')]
HEADERS=('Tipo Observación','Etapa','Fecha Centro','Usuario Centro','Observación Centro','Fecha Tribunal','Usuario Tribunal','Observación Tribunal','Acción')
CATALOGS={
    'ESPERA': ('4',[('3','Ingreso Efectivo'),('4','Gestión Previa')]),
    'CUMPLIMIENTO': ('6',[('5','Solicita Plazo'),('6','Realizada'),('7','Pide Cuenta')]),
    'INFORMES_POR_VENCER': ('9',[('8','Solicita Plazo'),('9','Realizada'),('10','Pide Cuenta')]),
    'INFORMES_VENCIDOS': ('9',[('8','Solicita Plazo'),('9','Realizada'),('10','Pide Cuenta')]),
}
# Etapa, modalidad y tipo son datos inventados de la vinculación, no inferencias del sitio.
CONTEXT=dict(COD_Tribunal='900000',CRR_IdCausa='900001',ID_Ingreso='900002',COD_Etapa='1',TIP_Consulta='2',tipo_popUp='11')
TEXT='  Constancia ficticia. Áé.\nSegunda línea.  '
def fixture(origin,destination):
    code,options=CATALOGS[origin]
    listed=f'''<table data-current="true"><tbody><tr><td><a href="javascript:ShowObservaciones(900001,900000,900002,11,1,0)">Bitácora ficticia</a></td></tr></tbody></table>'''
    popup='<form name="InformesPpalForm" method="post" action="https://familia.pjud.cl/SITFAWEB/InformesDAction.do">'
    popup+='<table id="TablaInforme"><tr>'+''.join('<td>'+h+'</td>' for h in HEADERS)+'</tr><tr><td>Historial ficticio</td></tr></table>'
    popup+='<select id="COD_Estado_Detalle_Nuevo" name="COD_Estado_Detalle_Nuevo"><option value="x">...</option>'+''.join(f'<option value="{k}">{v}</option>' for k,v in options)+'</select>'
    popup+='<span>Enviar observación al tribunal</span><input type="checkbox" name="chkBitacora" '+('checked' if destination=='ADMINISTRATIVA' else '')+'>'
    popup+='<textarea id="GLS_ObsCentro_Nueva" name="id_Obs_centro"></textarea><input id="Txt_Cont2" name="Txt_Cont2" readonly><input type="submit" name="irAccion" value="Nueva obs"><input type="button" name="volv" value="Volver">'
    popup+='<div hidden><input type="checkbox" name="chkBitacoraEdit" checked><textarea name="GLS_Observacion">Edición ficticia intacta</textarea><input type="submit" name="irAccion" value="Grabar"></div>'
    popup+=''.join(f'<input type="hidden" name="{k}" value="{v}">' for k,v in CONTEXT.items())+'</form>'
    op=dict(schema='RegistroFlowV1',operation_id='fixture-op',source=dict(tool='FLOW',work_id='fixture',row_ref='fixture-row'),
        remote_identity=dict(system='SITFA',tribunal_id=CONTEXT['COD_Tribunal'],causa_id=CONTEXT['CRR_IdCausa'],ingreso_id=CONTEXT['ID_Ingreso']),
        origin=origin,destination=destination,observation_state=code,approved_text=TEXT,approved_text_sha256=sha256(TEXT.encode()).hexdigest(),
        approved_by='fixture',approved_at_utc='2026-11-24T00:00:00Z',approval_version=1)
    return dict(listed=listed,popup=popup,op=op)

def load_fixture(page,origin,destination):
    page.goto('about:blank');page.set_content('<iframe></iframe><iframe></iframe>')
    for js in SCRIPTS:page.add_script_tag(content=js)
    page.evaluate("""data=>{
        window.L=document.querySelectorAll('iframe')[0].contentDocument;
        window.P=document.querySelectorAll('iframe')[1].contentDocument;
        L.documentElement.dataset.fixture='P3';L.body.dataset.menu='89';L.body.dataset.origin=data.op.origin;L.body.dataset.modality='2';L.body.innerHTML=data.listed;
        P.documentElement.dataset.fixture='OBSERVED_PREPARATION';P.body.innerHTML=data.popup;
        window.op=data.op;window.current=JSON.parse(JSON.stringify(op));
        window.events={submit:0,click:0,input:0,change:0};
        for(const d of [L,P])for(const e of Object.keys(events))d.addEventListener(e,ev=>{events[e]++;if(e==='submit')ev.preventDefault()},true);
    }""",fixture(origin,destination))

if __name__=='__main__':
    with sync_playwright() as p:
        browser=p.chromium.launch(headless=True,**({'executable_path':os.environ['REGISTRADOR_CHROMIUM']} if os.getenv('REGISTRADOR_CHROMIUM') else {}))
        context=browser.new_context();context.route('**/*',lambda r:r.abort())
        page=context.new_page()
        count=0
        def check(origin='ESPERA',destination='ADMINISTRATIVA',change='',reason=None,partial=False):
            global count
            load_fixture(page,origin,destination)
            if change:page.evaluate(change)
            before=page.evaluate("()=>({editor:P.querySelector('#GLS_ObsCentro_Nueva').value,checkbox:P.querySelector('[name=chkBitacora]').checked,state:P.querySelector('select').value})")
            result=page.evaluate("()=>{try{return RegistradorObservedPreparation.prepare(L,P,op,current)}catch(e){return {reason:e.message}}}")
            after=page.evaluate("()=>({editor:P.querySelector('#GLS_ObsCentro_Nueva').value,checkbox:P.querySelector('[name=chkBitacora]').checked,state:P.querySelector('select').value})")
            assert page.evaluate("events.submit===0&&events.click===0&&P.querySelector('[name=chkBitacoraEdit]').checked&&P.querySelector('[name=GLS_Observacion]').value==='Edición ficticia intacta'")
            if reason:
                assert result=={'reason':reason},result
                if not partial:assert before==after,(before,after,result)
            else:
                assert result['saved'] is False and result['write_enabled'] is False and result['cc']==int(destination=='AL_TRIBUNAL'),result
                assert after==dict(editor=TEXT,checkbox=destination=='AL_TRIBUNAL',state=CATALOGS[origin][0]),after
            count+=1
        for origin in CATALOGS:
            for destination in ('ADMINISTRATIVA','AL_TRIBUNAL'):check(origin,destination)
        for key in CONTEXT:
            check(change=f"P.querySelector('[name={key}]').value='999999'",reason='CONTEXTO_NO_COINCIDE')
        check(change="current.approved_text='Otra'",reason='APROBACION_INVALIDA')
        check(change="op.observation_state=current.observation_state='3'",reason='APROBACION_INVALIDA')
        check(change="P.querySelector('#GLS_ObsCentro_Nueva').value='Borrador previo'",reason='BORRADOR_EXISTENTE')
        check(change="P.querySelector('#GLS_ObsCentro_Nueva').maxLength=2",reason='TEXTO_NO_REPRESENTABLE')
        check(change="P.querySelector('option[value=\"4\"]').disabled=true",reason='ESTADO_OBLIGATORIO_NO_DISPONIBLE')
        check(change="P.documentElement.dataset.fixture='wrong'",reason='ENTORNO_NO_SINTETICO')
        check(change="L.querySelector('tbody').append(L.querySelector('tr').cloneNode(true))",reason='INGRESO_NO_UNICO')
        check(change="P.querySelector('textarea').addEventListener('change',()=>P.querySelector('[name=ID_Ingreso]').value='999999')",reason='CONTEXTO_NO_COINCIDE',partial=True)
        check(change="P.querySelector('textarea').addEventListener('input',e=>e.target.value='Alterado')",reason='VALORES_NO_COINCIDEN',partial=True)
        browser.close();print(f'PASS {count} observed preparation checks')
