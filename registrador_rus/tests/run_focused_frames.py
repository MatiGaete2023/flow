"""DOM ficticio; prueba del recorrido, no prueba de acceso institucional."""
import json
from pathlib import Path
from html import escape
from playwright.sync_api import sync_playwright
import os
ROOT=Path(__file__).resolve().parents[1]
JS=(ROOT/'inspector'/'inspect.js').read_text(encoding='utf-8')
FIXTURE='''<form name="InformesPpalForm" method="post" action="https://familia.pjud.cl/SITFAWEB/InformesDAction.do">
<table id="TablaInforme"><tr><td>NO_EXPORTAR_HISTORIA</td></tr></table>
<select name="COD_Estado_Detalle_Nuevo"><option value="1">Estado ficticio</option><option value="2">Otro estado</option><option value="3">Pendiente</option></select>
<label for="check">Enviar observación al tribunal</label><input id="check" type="checkbox">
<textarea maxlength="2000">NO_EXPORTAR_EDITOR</textarea><input type="text" readonly><button name="irAccion">Nueva obs</button><button>Cerrar</button>
<input type="hidden" name="COD_Tribunal" value="987654321"><input type="hidden" name="CRR_IdCausa"><input type="hidden" name="ID_Ingreso"><input type="hidden" name="COD_Etapa"><input type="hidden" name="TIP_Consulta"></form>'''

def frame(content,attrs=''):
    return '<iframe '+attrs+' srcdoc="'+escape(content,quote=True)+'"></iframe>'

with sync_playwright() as p:
    browser=p.chromium.launch(headless=True,**({'executable_path':os.environ['REGISTRADOR_CHROMIUM']} if os.getenv('REGISTRADOR_CHROMIUM') else {}))
    page=browser.new_page();page.route('**/*',lambda r:r.abort())
    count=0
    def check(html,reason=None,depth=None):
        global count
        page.goto('about:blank');page.set_content(html,wait_until='load');page.add_script_tag(content=JS)
        for f in page.frames:
            f.evaluate("() => {window.events=0; for(const e of ['submit','input','change','click']) document.addEventListener(e,()=>window.events++,true);}")
        result=page.evaluate("inspectFocusTree(document,location.origin)")
        assert result['write_enabled'] is False,result
        assert result.get('reason')==reason,result
        assert result.get('frame_depth')==depth,result
        assert all(f.evaluate('window.events')==0 for f in page.frames)
        assert 'NO_EXPORTAR' not in json.dumps(result) and '987654321' not in json.dumps(result)
        if reason is None: assert result['status']=='CANDIDATE' and result['editor']['maxlength']==2000,result
        else: assert result['status']=='BLOCKED',result
        count+=1
    check(FIXTURE)
    check(frame(FIXTURE),depth=1)
    check(frame(frame(FIXTURE)),depth=2)
    check(frame(FIXTURE)+frame(FIXTURE),reason='MARCOS_AMBIGUOS')
    check(frame(FIXTURE,'sandbox=""'),reason='MARCO_NO_ACCESIBLE')
    check(frame(FIXTURE,'hidden'),reason='FORMULARIO_NO_UNICO')
    check(frame('<p>Sin formulario</p>'),reason='FORMULARIO_NO_UNICO',depth=1)
    check(frame(FIXTURE.replace('Nueva obs','Grabar')),reason='ACCION_CONTRADICTORIA',depth=1)
    deep=FIXTURE
    for _ in range(5): deep=frame(deep)
    check(deep,reason='PROFUNDIDAD_MARCOS_EXCEDIDA',depth=4)
    check('<form></form>'+frame(FIXTURE),reason='FORMULARIO_NO_UNICO')
    assert page.evaluate('inspectFocus()')['reason']=='ORIGEN_NO_PERMITIDO'
    print(f'PASS {count} frame checks + origin guard')
    browser.close()
