from pathlib import Path
from html import escape
import json, os
from playwright.sync_api import sync_playwright
ROOT=Path(__file__).resolve().parent
BASE=ROOT/'inspector' if (ROOT/'inspector').exists() else ROOT.parent/'inspector'
JS=(BASE/'integral.js').read_text(encoding='utf-8')
POP=(BASE/'integral_popup.js').read_text(encoding='utf-8')
HEAD='<tr>'+''.join('<th>'+x+'</th>' for x in ['Tipo Observación','Etapa','Fecha Centro','Usuario Centro','Observación Centro','Fecha Tribunal','Usuario Tribunal','Observación Tribunal','Acción'])+'</tr>'
FIXTURE='''<form name="InformesPpalForm" id="mainForm" method="post" action="https://familia.pjud.cl/SITFAWEB/InformesDAction.do"><select name="COD_Estado_Detalle_Nuevo"><option value="1">Gestión Previa</option><option value="2" selected>Estado ficticio</option><option value="3">Otro estado</option></select><label for="check">Enviar observación al tribunal</label><input id="check" type="checkbox" checked><textarea id="editor" maxlength="2000">NO_EXPORTAR_EDITOR</textarea><input type="text" value="NO_EXPORTAR_AUX" readonly><button name="irAccion" onclick="NO_EXPORTAR_HANDLER()">Nueva obs</button><button type="button">Cerrar</button><input type="hidden" name="COD_Tribunal" value="987654321"><input type="hidden" name="CRR_IdCausa"><input type="hidden" name="ID_Ingreso"><input type="hidden" name="COD_Etapa"><input type="hidden" name="TIP_Consulta"><input type="hidden" name="tipo_popUp"></form><table id="TablaInforme">'''+HEAD+'''<tr><td>NO_EXPORTAR_HISTORIA</td><td><input type="hidden" name="HOBS_Centro_987654322" value="NO_EXPORTAR_HIDDEN"></td></tr></table>'''
def frame(body,attrs=''): return '<iframe '+attrs+' srcdoc="'+escape(body,quote=True)+'"></iframe>'
with sync_playwright() as p:
 b=p.chromium.launch(headless=True,**({'executable_path':os.environ['REGISTRADOR_CHROMIUM']} if os.getenv('REGISTRADOR_CHROMIUM') else {}));page=b.new_page();page.route('**/*',lambda r:r.abort())
 count=0
 def check(html,change=''):
  global count
  page.goto('about:blank');page.set_content(html,wait_until='load');page.add_script_tag(content=JS)
  if change:page.evaluate(change)
  before=page.evaluate('document.documentElement.outerHTML')
  for f in page.frames:f.evaluate("() => {window.events=0; for(const e of ['submit','input','change','click']) document.addEventListener(e,()=>window.events++,true);}")
  result=page.evaluate('inspectIntegralTree(document,location.origin)')
  assert result['status']=='DIAGNOSTIC' and result['write_enabled'] is False,result
  assert page.evaluate('document.documentElement.outerHTML')==before
  assert all(f.evaluate('window.events')==0 for f in page.frames)
  data=json.dumps(result,ensure_ascii=False);assert 'NO_EXPORTAR' not in data and '98765432' not in data and '"checked"' not in data and '"selected"' not in data,data
  # Validador real del popup, ejecutado en una página local sin API Chrome.
  assert page.evaluate('(arg)=>{const source=arg[0].split("button.addEventListener")[0];return new Function("document","evidence",source+";return validEvidence(evidence);")({getElementById:()=>({})},arg[1]);}',[POP,result]),result
  count+=1;return result
 r=check(FIXTURE);d=r['documents'][0]['detail'];assert d['tables'][0]['owner']==-1 and d['tables'][0]['headers']['exact'];assert d['tables'][0]['hidden_prefix_counts']==[1,0,0,0,0];assert len(d['controls'])==6 and len(d['context'])==6
 assert next(c for c in d['controls'] if c['kind']=='checkbox')['label_associated']
 assert next(c for c in d['controls'] if c['kind']=='textarea')['maxlength']==2000
 assert next(c for c in d['controls'] if c['button_label']=='NUEVA_OBS')['onclick']
 assert next(c for c in d['controls'] if c['kind']=='select')['options'][0]['label']=='Gestión Previa'
 r=check(frame(FIXTURE)+frame(frame(FIXTURE)));assert [x['path'] for x in r['documents']]==[[],[0],[1],[1,0]]
 r=check(frame(FIXTURE,'sandbox=""'));assert r['documents'][1]['access']=='INACCESIBLE'
 r=check(FIXTURE,"document.querySelector('table').hidden=true;document.forms[0].removeAttribute('name');document.querySelector('textarea').id='ingreso987654321';document.querySelector('select').remove(0)");d=r['documents'][0]['detail'];assert not d['tables'][0]['visible'] and d['form_candidates'][0]['name'] is None and next(c for c in d['controls'] if c['kind']=='textarea')['id']=='OMITIDO' and next(c for c in d['controls'] if c['kind']=='select')['options_count']==2
 r=check(FIXTURE,"document.querySelector('table').id='different';document.body.append(document.createElement('textarea'));document.querySelector('input[name=ID_Ingreso]').remove()");d=r['documents'][0]['detail'];assert d['tables'][0]['headers']['exact'] and d['table_id_nodes']==0 and d['visible_controls']==7 and next(c for c in d['context'] if c['key']=='ID_Ingreso')['total']==0
 r=check(FIXTURE,"document.querySelector('label').removeAttribute('for');document.querySelector('textarea').hidden=true;document.querySelector('form').append(document.querySelector('table'));document.querySelector('table').append(document.createElement('tbody'))");d=r['documents'][0]['detail'];assert d['tables'][0]['owner']==0 and d['hidden_controls']==1 and next(c for c in d['controls'] if c['kind']=='checkbox')['label_nearby']
 r=check(FIXTURE,"for(let i=0;i<60;i++)document.forms[0].append(document.createElement('textarea'))");assert r['truncated'] and len(r['documents'][0]['detail']['controls'])==48
 r=check('<form><input value="NO_EXPORTAR"></form>');assert not r['documents'][0]['detail']['controls']
 r=check(FIXTURE,"document.querySelector('table').id='other';const div=document.createElement('div');div.id='TablaInforme';document.querySelector('table').before(div);div.append(document.querySelector('table'))");assert r['documents'][0]['detail']['table_id_nodes']==1 and len(r['documents'][0]['detail']['tables'])==1
 r=check(FIXTURE,"document.querySelector('select').options[0].value='987654321';document.querySelector('select').options[0].textContent='NO_EXPORTAR_987654321'");assert next(c for c in r['documents'][0]['detail']['controls'] if c['kind']=='select')['options'][0]=={'code':'OMITIDO','label':'OMITIDO','disabled':False}
 # Comprobar rechazo de campos adicionales y contenido no autorizado en la salida.
 r=check(FIXTURE)
 bads=[]
 for mode in ('extra','control_value','state_label','identity','write','frame'):
  v=json.loads(json.dumps(r));d=v['documents'][0]['detail']
  if mode=='extra':v['html']='NO_EXPORTAR'
  elif mode=='control_value':d['controls'][0]['value']='NO_EXPORTAR'
  elif mode=='state_label':next(c for c in d['controls'] if c['kind']=='select')['options'][0]['label']='NO_EXPORTAR_123'
  elif mode=='identity':d['context'][0]['value']='987654321'
  elif mode=='write':v['write_enabled']=True
  elif mode=='frame':v['documents'][0]['path']=[1]
  bads.append(v)
 assert page.evaluate('(arg)=>{const source=arg[0].split("button.addEventListener")[0];const fn=new Function("document","evidence",source+";return validEvidence(evidence);");return arg[1].every(v=>!fn({getElementById:()=>({})},v));}',[POP,bads])
 assert page.evaluate('inspectIntegral()')['reason']=='ORIGEN_NO_PERMITIDO' 
 b.close();print(f'PASS {count} integral DOM checks + 6 output rejections + origin guard + popup schema')
