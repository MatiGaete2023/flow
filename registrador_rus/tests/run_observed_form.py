"""Prueba de detector nuevo sobre reconstrucción ficticia, no DOM de RUS."""
from pathlib import Path
import os
from playwright.sync_api import sync_playwright

JS=(Path(__file__).resolve().parents[1]/'extension'/'observed_form.js').read_text(encoding='utf-8')
HEADERS=('Tipo Observación','Etapa','Fecha Centro','Usuario Centro','Observación Centro','Fecha Tribunal','Usuario Tribunal','Observación Tribunal','Acción')
FIXTURE='''<form name="InformesPpalForm" method="post" action="https://familia.pjud.cl/SITFAWEB/InformesDAction.do">
<table id="TablaInforme"><tr>'''+''.join('<td>'+h+'</td>' for h in HEADERS)+'''</tr><tr><td>NO_EXPORTAR_HISTORIAL</td></tr></table>
<select id="COD_Estado_Detalle_Nuevo" name="COD_Estado_Detalle_Nuevo"><option value="x">...</option><option value="3">Ingreso Efectivo</option><option value="4">Gestión Previa</option></select>
<span>Enviar observación al tribunal</span><input type="checkbox" name="chkBitacora">
<textarea id="GLS_ObsCentro_Nueva" name="id_Obs_centro">NO_EXPORTAR_EDITOR</textarea><input id="Txt_Cont2" name="Txt_Cont2" readonly>
<input type="submit" name="irAccion" value="Nueva obs"><input type="button" name="volv" value="Volver">
<div role="dialog" hidden><input type="checkbox" name="chkBitacoraEdit"><textarea name="GLS_Observacion" maxlength="2000"></textarea><input type="submit" name="irAccion" value="Grabar"></div>'''+''.join('<input type="hidden" name="'+key+'" value="987654321">' for key in ('COD_Tribunal','CRR_IdCausa','ID_Ingreso','COD_Etapa','TIP_Consulta','tipo_popUp'))+'</form>'
with sync_playwright() as p:
    browser=p.chromium.launch(headless=True,**({'executable_path':os.environ['REGISTRADOR_CHROMIUM']} if os.getenv('REGISTRADOR_CHROMIUM') else {}))
    page=browser.new_page();page.route('**/*',lambda r:r.abort())
    count=0
    def check(change='',reason=None,origin='ESPERA',code='4'):
        global count
        page.goto('about:blank');page.set_content(FIXTURE);page.add_script_tag(content=JS)
        if change:page.evaluate(change)
        page.evaluate("() => {window.events=0;for(const e of ['submit','input','change','click'])document.addEventListener(e,()=>window.events++,true)}")
        before=page.content()
        result=page.evaluate("origin => {try{return RegistradorObservedForm.inspect(document,origin)}catch(e){return {reason:e.message}}}",origin)
        assert page.content()==before and page.evaluate('window.events')==0
        if reason:assert result=={'reason':reason},result
        else:
            assert result['states']==[code] and result['write_enabled'] is False,result
            assert result['editor_maxlength'] is None and result['context_fields']==6,result
            assert 'NO_EXPORTAR' not in str(result) and '987654321' not in str(result),result
        count+=1
    check()
    check("document.querySelector('option[value=\"3\"]').disabled=true")
    check("document.querySelector('option[value=\"4\"]').textContent='Otro'",'ESTADO_OBLIGATORIO_NO_DISPONIBLE')
    check("document.querySelector('option[value=\"4\"]').disabled=true",'ESTADO_OBLIGATORIO_NO_DISPONIBLE')
    check("document.querySelector('select').append(document.querySelector('option[value=\"4\"]').cloneNode(true))",'ESTADO_OBLIGATORIO_NO_DISPONIBLE')
    check(origin='CUMPLIMIENTO',reason='ESTADO_OBLIGATORIO_NO_DISPONIBLE')
    check(origin='CALENDARIO',reason='ORIGEN_INVALIDO')
    for origin,code,options in (
        ('CUMPLIMIENTO','6',[('5','Solicita Plazo'),('6','Realizada'),('7','Pide Cuenta')]),
        ('INFORMES_POR_VENCER','9',[('8','Solicita Plazo'),('9','Realizada'),('10','Pide Cuenta')]),
        ('INFORMES_VENCIDOS','9',[('8','Solicita Plazo'),('9','Realizada'),('10','Pide Cuenta')])):
        import json
        markup='<option value="x">...</option>'+''.join(f'<option value="{k}">{v}</option>' for k,v in options)
        change='document.querySelector("select").innerHTML='+json.dumps(markup)
        if origin=='CUMPLIMIENTO': change+=';document.querySelector("div[role=dialog]").removeAttribute("role")'
        check(change,origin=origin,code=code)
        check(change+';document.querySelector("option[value=\\\"'+code+'\\\"]").disabled=true','ESTADO_OBLIGATORIO_NO_DISPONIBLE',origin=origin)
    check("document.querySelector('input[name=chkBitacora]').name='chkBitacoraEdit'",'CONTROL_NO_UNICO')
    check("document.querySelector('span').textContent='Otro destino'",'ETIQUETA_NO_CONFIRMADA')
    check("document.querySelector('input[name=irAccion]').value='Grabar'",'CREACION_DISTINTA')
    check("document.querySelector('div[role=dialog]').hidden=false",'CONTEXTO_AMBIGUO')
    check("document.querySelector('input[name=tipo_popUp]').remove()",'CONTEXTO_NO_UNICO')
    check("document.querySelector('table td').textContent='Otro encabezado'",'ENCABEZADOS_DISTINTOS')
    check("document.querySelector('form').append(document.createElement('textarea'))",'FIRMA_DISTINTA')
    check("document.querySelector('input[name=irAccion]').setAttribute('formaction','https://example.test/')",'CREACION_DISTINTA')
    check("document.querySelector('form').append(document.querySelector('input[name=irAccion]').cloneNode(true))",'CREACION_NO_UNICA')
    browser.close();print(f'PASS {count} observed form checks')
