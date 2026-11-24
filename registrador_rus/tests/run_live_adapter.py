"""Prueba del núcleo de adaptador real en DOM inventado, sin tráfico institucional."""
from pathlib import Path
from hashlib import sha256
import os
from playwright.sync_api import sync_playwright
from run_observed_prepare import load_fixture, CATALOGS, TEXT

JS=(Path(__file__).resolve().parents[1]/'extension'/'live_adapter.js').read_text(encoding='utf-8')
TABS={'ESPERA':('tablaEspera','tdEspera'),'CUMPLIMIENTO':('tablaCumplimiento','tdCumplimiento'),
      'INFORMES_POR_VENCER':('tablaInforme','tdInforme'),'INFORMES_VENCIDOS':('tablaInforme','tdInforme')}
with sync_playwright() as p:
    browser=p.chromium.launch(headless=True,**({'executable_path':os.environ['REGISTRADOR_CHROMIUM']} if os.getenv('REGISTRADOR_CHROMIUM') else {}))
    context=browser.new_context();context.route('**/*',lambda r:r.abort())
    page=context.new_page()
    count=0
    def check(origin='ESPERA',dest='ADMINISTRATIVA',change='',reason=None,phase='locate'):
        global count
        load_fixture(page,origin,dest);page.add_script_tag(content=JS)
        page.evaluate("""({tab,id,opid})=>{
            L.documentElement.dataset.fixture='LIVE_ADAPTER_TEST';
            const t=L.querySelector('table'); t.id=id;
            const form=L.createElement('form');form.name='InformesPpalForm';form.method='post';form.action='https://familia.pjud.cl/SITFAWEB/InformesDAction.do';
            form.innerHTML='<input type="hidden" name="COD_Lengueta" value="'+tab+'"><select name="COD_Tribunal_sel"><option value="900000">Ficticio</option></select><select name="TIP_Consulta"><option value="2">Ficticio</option></select><select name="TIP_Informe"><option value="1">Por vencer (a 25 días)</option><option value="2">Vencidos</option></select>';
            L.body.append(form);form.append(t);
            form.elements.TIP_Informe.value=op.origin==='INFORMES_VENCIDOS'?'2':'1';
            op.operation_id=opid;current=JSON.parse(JSON.stringify(op));
        }""",dict(tab=TABS[origin][1],id=TABS[origin][0],opid=sha256(b'fixture-op').hexdigest()))
        if change:page.evaluate(change)
        result=page.evaluate("""async phase=>{
            try{
                const A=RegistradorLiveAdapter;
                if(phase==='real')return A.locate(L,op);
                const hit=A.fixture.locate(L,op);
                if(phase==='capture'){
                    const html=A.fixture.capture(P,op,hit.context);
                    return {html:html.includes('TablaInforme'),saved:false};
                }
                const s=await A.fixture.session(P,op,hit.context);
                const r=s.prepare(current);s.verify(current);
                try{s.createOnce(current,{operation_id:op.operation_id});return {reason:'WRITE_ESCAPED'};}
                catch(e){if(e.message!=='ESCRITURA_REAL_NO_HABILITADA')throw e;}
                return r;
            }catch(e){return {reason:e.message};}
        }""",phase)
        assert page.evaluate('events.submit===0&&events.click===0')
        if reason:assert result==dict(reason=reason),result
        elif phase=='capture':assert result==dict(html=True,saved=False),result
        else:
            assert result['cc']==int(dest=='AL_TRIBUNAL') and result['state']==CATALOGS[origin][0],result
            assert page.evaluate("P.querySelector('#GLS_ObsCentro_Nueva').value")==TEXT
        count+=1
    for o in TABS:
        for d in ('ADMINISTRATIVA','AL_TRIBUNAL'):check(o,d)
    check(phase='capture')
    check(phase='real',reason='RUTA_NO_LISTADO')
    check(change="L.querySelector('input').value='tdEgreso'",reason='PESTANA_DISTINTA')
    check(change="L.querySelector('select[name=COD_Tribunal_sel]').innerHTML='<option value=999>Otro</option>'",reason='TRIBUNAL_DISTINTO')
    check('INFORMES_VENCIDOS',change="L.querySelector('[name=TIP_Informe]').value='1'",reason='FILTRO_INFORMES_DISTINTO')
    check(change="P.querySelector('[name=ID_Ingreso]').value='999999'",reason='CONTEXTO_NO_COINCIDE')
    check(change="op.approved_text=current.approved_text='modificado'",reason='HUELLA_TEXTO_DISTINTA')
    check(change="P.querySelector('textarea').value='otro borrador'",reason='BORRADOR_EXISTENTE')
    check(change="L.querySelector('tbody').append(L.querySelector('tr').cloneNode(true))",reason='INGRESO_NO_UNICO')
    browser.close()
    print(f'PASS {count} live adapter core checks (no live access)')
