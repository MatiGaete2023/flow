from pathlib import Path
from playwright.sync_api import sync_playwright
import json
import os
JS=(Path(__file__).resolve().parents[1]/'inspector'/'inspect.js').read_text(encoding='utf-8')
FIXTURE='''<html><body><form name="InformesPpalForm" method="post" action="https://familia.pjud.cl/SITFAWEB/InformesDAction.do">
<table id="TablaInforme"><tr><td>NO_EXPORTAR_TEXTO_HISTORICO</td></tr></table>
<select name="COD_Estado_Detalle_Nuevo" id="estado"><option value="1">Pendiente ficticio</option><option value="2" selected>Estado ficticio</option><option value="3">Otro ficticio</option></select>
<label for="demo-check">Enviar observación al tribunal</label><input type="checkbox" id="demo-check" name="envio" checked>
<textarea id="demo-editor" name="editor" maxlength="2000">NO_EXPORTAR_OBSERVACION</textarea><input type="text" readonly value="NO_EXPORTAR_AUXILIAR"><button name="irAccion" type="submit">Nueva obs</button><button type="button">Cerrar</button>
<div hidden><input type="checkbox"><textarea name="GLS_Observacion" maxlength="2000"></textarea><button name="irAccion">Grabar</button></div>
<input type="hidden" name="COD_Tribunal" value="987654321"><input type="hidden" name="CRR_IdCausa" value="987654322"><input type="hidden" name="ID_Ingreso" value="987654323"><input type="hidden" name="COD_Etapa"><input type="hidden" name="TIP_Consulta">
</form></body></html>'''
with sync_playwright() as p:
 options={'executable_path':os.environ['REGISTRADOR_CHROMIUM']} if os.getenv('REGISTRADOR_CHROMIUM') else {}
 b=p.chromium.launch(headless=True,**options)
 page=b.new_page(); page.route('**/*',lambda r:r.abort())
 def check(change='',reason=None,wrapper=False):
  page.goto('about:blank'); page.set_content(FIXTURE); page.add_script_tag(content=JS)
  if change: page.evaluate(change)
  page.evaluate("() => {window.events=0; for(const e of ['submit','input','change','click']) document.addEventListener(e,()=>window.events++,true);}")
  result=page.evaluate('inspectFocus()' if wrapper else 'inspectFocusDocument(document)')
  assert page.evaluate('window.events')==0
  assert result['write_enabled'] is False
  if reason: assert result==dict(schema='FocusedEvidenceV1',status='BLOCKED',reason=reason,write_enabled=False),result
  else:
   assert result['status']=='CANDIDATE',result
   data=json.dumps(result,ensure_ascii=False)
   assert 'NO_EXPORTAR' not in data and '98765432' not in data and 'selected' not in data and 'checked' not in data
  return result
 r=check();assert r['checkbox']['label_associated'] and r['editor']['maxlength']==2000 and r['create']['label_matches'] and len(r['state']['options'])==3
 assert check("document.querySelector('label').removeAttribute('for')")['checkbox']['label_associated'] is False
 assert check("document.querySelector('textarea').id='ingreso987654321'")['editor']['id']=='OMITIDO'
 assert check("document.querySelector('option').textContent='NO_EXPORTAR_987654321'")['state']['options'][0]['label']=='OMITIDO'
 check("document.querySelector('button[name=irAccion]').textContent='Grabar'",'ACCION_CONTRADICTORIA')
 check("document.querySelector('form').action='https://familia.pjud.cl/wrong'",'ACCION_INESPERADA')
 check("document.querySelector('form').append(document.createElement('textarea'))",'FIRMA_DISTINTA')
 check("document.querySelector('input[name=ID_Ingreso]').remove()",'CONTEXTO_ESTRUCTURAL_INCOMPLETO')
 check(wrapper=True,reason='ORIGEN_NO_PERMITIDO')
 b.close();print('PASS 9 focused inspection checks')
