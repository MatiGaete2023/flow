"""Prueba focal de diagnóstico simultáneo; no utiliza datos ni conexión RUS."""
from pathlib import Path
from html import escape
import json
import os
from playwright.sync_api import sync_playwright
JS=(Path(__file__).resolve().parents[1]/'inspector'/'inspect.js').read_text(encoding='utf-8')
FIXTURE='''<form name="InformesPpalForm" method="post" action="https://familia.pjud.cl/SITFAWEB/InformesDAction.do">
<table id="TablaInforme"><tr><td>NO_EXPORTAR_HISTORIA</td></tr></table>
<select name="COD_Estado_Detalle_Nuevo"><option value="1">Estado ficticio</option><option value="2">Otro estado</option><option value="3">Pendiente</option></select>
<label for="check">Enviar observación al tribunal</label><input id="check" type="checkbox">
<textarea maxlength="2000">NO_EXPORTAR_EDITOR</textarea><input type="text" readonly><button name="irAccion">Nueva obs</button><button>Cerrar</button>
<input type="hidden" name="COD_Tribunal" value="987654321"><input type="hidden" name="CRR_IdCausa"><input type="hidden" name="ID_Ingreso"><input type="hidden" name="COD_Etapa"><input type="hidden" name="TIP_Consulta"></form>'''
with sync_playwright() as p:
 b=p.chromium.launch(headless=True,**({'executable_path':os.environ['REGISTRADOR_CHROMIUM']} if os.getenv('REGISTRADOR_CHROMIUM') else {})); page=b.new_page(); page.route('**/*',lambda r:r.abort())
 count=0
 def check(change='',reason=None,evidence=False,**expected):
  global count
  page.goto('about:blank');page.set_content(FIXTURE);page.add_script_tag(content=JS)
  if change: page.evaluate(change)
  page.evaluate("() => {window.events=0; for(const e of ['submit','input','change','click']) document.addEventListener(e,()=>window.events++,true);}")
  r=page.evaluate('inspectFocusDocument(document)'); assert r.get('reason')==reason,r
  assert r['write_enabled'] is False and page.evaluate('window.events')==0
  assert 'NO_EXPORTAR' not in json.dumps(r) and '987654321' not in json.dumps(r)
  if reason:
   assert r['status']=='BLOCKED' and ('control_evidence' in r)==evidence,r
   for k,v in expected.items(): assert r['structure'][k]==v,r
  else: assert r['status']=='CANDIDATE',r
  count+=1
 check()
 check("document.querySelector('table').remove()",'TABLA_NO_UNICA',True,table_id_matches=0,control_checks_ok=1)
 check("document.body.append(document.querySelector('table'))",'TABLA_NO_UNICA',True,table_inside_form=0,table_visible=1)
 check("document.querySelector('table').hidden=true",'TABLA_NO_UNICA',True,table_inside_form=1,table_inside_visible=0)
 check("document.body.append(document.querySelector('table').cloneNode(true))",'TABLA_NO_UNICA',True,table_id_matches=2)
 check("document.querySelector('table').outerHTML='<div id=TablaInforme></div>'",'TABLA_NO_UNICA',True,table_tags=0)
 check("document.querySelector('input[name=ID_Ingreso]').remove()",'CONTEXTO_ESTRUCTURAL_INCOMPLETO',True,context_fields_ok=4)
 check("document.querySelector('form').append(document.createElement('textarea'))",'FIRMA_DISTINTA',False,visible_controls=7,editors=2,signature_ok=0)
 check("document.querySelector('table').remove();document.querySelector('form').append(document.createElement('textarea'));document.querySelector('input[name=ID_Ingreso]').remove()",'TABLA_NO_UNICA',False,table_id_matches=0,context_fields_ok=4,editors=2)
 check("document.querySelector('select').remove(0)",'CONTROLES_AMBIGUOS',False,state_options=2,signature_ok=1)
 check("document.querySelector('button[name=irAccion]').textContent='Grabar'",'ACCION_CONTRADICTORIA',False,control_checks_ok=0)
 check("document.querySelector('label').htmlFor='other';document.querySelector('input[type=text]').id='other'",'ETIQUETA_CONTRADICTORIA',False,control_checks_ok=0)
 page.goto('about:blank');page.set_content('<iframe srcdoc="'+escape(FIXTURE.replace('id="TablaInforme"','id="other"'),quote=True)+'"></iframe>',wait_until='load');page.add_script_tag(content=JS)
 r=page.evaluate('inspectFocusTree(document,location.origin)');assert r['frame_depth']==1 and r['reason']=='TABLA_NO_UNICA' and 'control_evidence' in r,r
 count+=1
 b.close();print(f'PASS {count} structural diagnostic checks')
