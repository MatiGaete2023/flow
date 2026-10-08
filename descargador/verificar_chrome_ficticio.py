"""Chrome real, SITFA ficticio y adaptadores para las API de extension."""
import asyncio
import base64
from http.client import HTTPConnection
import json
from pathlib import Path
import sys
import threading
from urllib.parse import parse_qsl

root=Path(__file__).resolve().parent
app=root;sys.path.insert(0,str(app))
import motor as m
from lotes import Batch,BatchRunner,Dates
from perfiles import Selection,TABS
from puente import Bridge
from test_lotes import CATALOG,document
from test_poc import excel
from playwright.async_api import async_playwright

class FastBridge(Bridge):
    def call(self,command,payload=None,timeout=75):return super().call(command,payload,min(timeout,12))
bridge=FastBridge();ready=threading.Event();stop=threading.Event();fault=[];network=[];checked=[];paths=[];last_status=[]
destination=root/'pruebas_locales'/'v2_fixture'
destination.mkdir(parents=True,exist_ok=True)


def master():
    fields={'COD_Lengueta':'','FLG_Consulta':'0','FEC_Inicio':'01/09/2026','FEC_Fin':'30/09/2026',
            'RUT_Consulta':'12345678','RUT_DvConsulta':'9','ROL_Causa':'','ERA_Causa':'0','TIP_Causa':'0'}
    fields.update({k:'' for k in m.PAGINATION if k in m.ALLOWED_FIELDS})
    source='<html><meta charset="utf-8"><body><form name="InformesPpalForm" method="post" action="/SITFAWEB/InformesDAction.do">'
    source+=''.join(f'<input name="{k}" value="{v}">' for k,v in fields.items())
    source+='<input type="checkbox" name="CHK_Consulta"><select name="COD_CentroResidencial"><option value="-1">Todos</option><option value="7" selected>Centro ficticio</option></select>'
    for name,key in [('COD_Tribunal_sel','tribunales'),('TIP_Consulta','modalidades'),('TIP_Informe','informes')]:
        source+=f'<select name="{name}" id="{name}" onchange="document.InformesPpalForm.COD_CentroResidencial.value=\'-1\';">'+''.join(f'<option value="{k}">{v}</option>' for k,v in CATALOG[key].items())+'</select>'
    source+='<input type="submit" name="irAccion" value="Buscar medida"></form>'
    source+=''.join(f'<a id="{cfg[0]}" onclick="Lengueta(\'{cfg[0]}\')">{label}</a><table id="{cfg[3]}"></table>' for label,cfg in TABS.items())
    source+='''<script>
    var idLengueta='tdEspera';
    function Lengueta(id){idLengueta=id;var s=document.getElementById('TIP_Informe');s.innerHTML='';
      var values=id==='tdInforme'? [['1','Por vencer'],['2','Vencidos'],['3','Recibidos']]:[['0','...'],['4','Medidas por vencer'],['5','Medidas vencidas']];
      for(var x of values)s.add(new Option(x[1],x[0]));s.disabled=id==='tdEspera';}
    function Seleccion(){var f=document.InformesPpalForm;f.FEC_Inicio.disabled=f.FEC_Fin.disabled=!f.CHK_Consulta.checked;}
    function Envio(){var f=document.InformesPpalForm;
      if(f.RUT_Consulta.value==='0'){alert('fixture: debe normalizarse después');return false;}
      f.COD_Lengueta.value=idLengueta;f.FLG_Consulta.value=f.CHK_Consulta.checked?'1':'0';
      if(!f.CHK_Consulta.checked){f.FEC_Inicio.disabled=false;f.FEC_Fin.disabled=false;}
      if(!f.RUT_Consulta.value){f.RUT_Consulta.value='0';f.RUT_DvConsulta.value='0';}
      if(!f.ROL_Causa.value)f.ROL_Causa.value='0';
    }
    Lengueta('tdEspera');Seleccion();
    </script></body></html>'''
    return source


async def actor():
    async with async_playwright() as pw:
        browser=await pw.chromium.launch(executable_path=pw.chromium.executable_path,headless=True)
        context=await browser.new_context()
        current={'number':1,'selection':None}
        async def route(request_route):
            request=request_route.request
            if request.url.endswith('/fixture'):
                await request_route.fulfill(body=master(),content_type='text/html');return
            if request.url==m.ENDPOINT:
                # La unica cookie es una constante ficticia; no se exporta a informes.
                if 'fixture_session=local_only' not in request.headers.get('cookie',''):raise AssertionError('fixture session')
                fields=dict(parse_qsl(request.post_data,keep_blank_values=True));tab=next(k for k,v in TABS.items() if v[0]==fields['COD_Lengueta'])
                report=fields.get('TIP_Informe') if tab=='Informes' or (tab=='Cumplimiento' and fields.get('TIP_Informe') in ('4','5')) else None
                selection=Selection(fields['COD_Tribunal_sel'],tab,fields['TIP_Consulta'],report)
                number=int(fields[TABS[tab][1]] or '1');current.update(number=number,selection=selection)
                assert fields['irAccion']=='Buscar medida'
                if fields['FLG_Consulta']=='1':assert (fields['FEC_Inicio'],fields['FEC_Fin'],fields['CHK_Consulta'])==('01/10/2026','31/10/2026','on')
                else:assert 'CHK_Consulta' not in fields
                assert fields['RUT_Consulta'] in ('0','12345678')
                if fields['RUT_Consulta']=='0':assert fields['COD_CentroResidencial']=='-1'
                else:assert fields['COD_CentroResidencial']=='7'
                checked.append((fields['FLG_Consulta'],fields['RUT_Consulta']=='0'))
                network.append(('POST',selection.tab,selection.tribunal,selection.modality,number))
                await request_route.fulfill(body=document(selection,number),content_type='text/html');return
            if '/sitfa/reportes/' in request.url:
                assert 'fixture_session=local_only' in request.headers.get('cookie','')
                s=current['selection'];network.append(('XLS',s.tab,s.tribunal,s.modality,current['number']))
                await request_route.fulfill(body=excel(current['number']),content_type='application/vnd.ms-excel');return
            await request_route.abort()
        await context.route('https://familia.pjud.cl/**',route)
        await context.add_cookies([{'name':'fixture_session','value':'local_only','domain':'familia.pjud.cl','path':'/','httpOnly':True,'secure':True}])
        page=await context.new_page();await page.goto('https://familia.pjud.cl/fixture')
        await page.add_script_tag(content=(app/'extension'/'pagina.js').read_text(encoding='utf-8'))

        # La interfaz de conexion se ejecuta completa; solo las API privilegiadas
        # chrome.scripting y el transporte desde origen extension tienen adaptadores.
        connector=await context.new_page()
        await connector.route('https://fixture.invalid/**',lambda r:r.fulfill(body='<html><body><input id="code"><button id="connect">Conectar</button><button id="disconnect">Desconectar</button><p id="status"></p></body></html>',content_type='text/html'))
        await connector.goto('https://fixture.invalid/?tab=1')
        async def execute_task(source,data):
            result=await page.evaluate('(data)=>sitfaTask(data.command,data.payload)',data)
            return [{'frameId':0,'result':result}]
        def http_call(url,init):
            port=bridge.port;path=url.split(f':{port}',1)[1]
            paths.append(path)
            c=HTTPConnection('127.0.0.1',port,timeout=75)
            headers={**init.get('headers',{}),'Origin':'chrome-extension://'+'a'*32}
            body=init.get('body');body=body.encode('utf-8') if isinstance(body,str) else body
            c.request(init.get('method','GET'),path,body=body,headers=headers)
            res=c.getresponse();data=res.read().decode();status=res.status;c.close();return {'status':status,'data':data}
        async def request_bridge(source,args):return await asyncio.to_thread(http_call,args['url'],args['init'])
        await connector.expose_binding('fixtureExecute',execute_task)
        await connector.expose_binding('fixtureHTTP',request_bridge)
        await connector.evaluate('''() => {
          window.chrome={runtime:{id:'a'.repeat(32)},scripting:{executeScript:opts=>fixtureExecute({command:opts.args[0],payload:opts.args[1]})}};
          window.fetch=async (url,init)=>{const r=await fixtureHTTP({url,init});return {ok:r.status===200,status:r.status,json:async()=>JSON.parse(r.data)}};
        }''')
        await connector.add_script_tag(content=(app/'extension'/'pagina.js').read_text(encoding='utf-8'))
        await connector.add_script_tag(content=(app/'extension'/'conexion.js').read_text(encoding='utf-8'))
        await connector.locator('#code').fill(bridge.connection_code);await connector.locator('#connect').click()
        for _ in range(100):
            if bridge.connected:ready.set();break
            await asyncio.sleep(.1)
        if not bridge.connected:raise RuntimeError('No se conecto el fixture')
        while not stop.is_set():
            last_status[:]=[await connector.locator('#status').text_content()]
            await asyncio.sleep(.1)
        # Los filtros iniciales fueron restaurados después del lote.
        restored=await page.evaluate("() => document.InformesPpalForm.RUT_Consulta.value==='12345678' && document.InformesPpalForm.COD_CentroResidencial.value==='7' && !window.__sitfaRun")
        assert restored
        await browser.close()


def run_actor():
    try:asyncio.run(actor())
    except BaseException as exc:fault.append(exc);ready.set()


thread=threading.Thread(target=run_actor,daemon=True);thread.start()
try:
    assert ready.wait(30)
    if fault:raise fault[0]
    catalog=bridge.call('catalog');assert set(catalog['informes'])=={'1','2','3'}
    events=[];runner=BatchRunner(bridge,catalog,lambda n,v:events.append((n,v)),threading.Event())
    runner.run(Batch(('49',),('1',),('Espera',)),destination)
    runner.run(Batch(('49','113'),('2',),('Cumplimiento',),dates=Dates('01/10/2026','31/10/2026')),destination)
    runner.run(Batch(('49',),('1','2'),('Informes',),'2',keep_filters=True),destination)
    runner.run(Batch(('49',),('4',),('Cumplimiento',),'4'),destination)
    assert len([v for n,v in events if n=='progress'])==18
    assert len([v for n,v in events if n=='done'])==4
    for tab,t,mod in [('Espera','49','1'),('Cumplimiento','49','2'),('Cumplimiento','113','2'),('Informes','49','1'),('Informes','49','2'),('Cumplimiento','49','4')]:
        assert [(kind,number) for kind,name,trib,modal,number in network if (name,trib,modal)==(tab,t,mod)]==[
            ('POST',1),('XLS',1),('POST',2),('XLS',2),('POST',3),('XLS',3)]
    assert set(checked)=={('0',True),('1',True),('0',False)}
    report={'resultado':'OK','chrome_real':True,'datos':'ficticios','lotes':4,'consultas':6,'paginas':18,
            'fechas_activadas':True,'filtros_limpiados_y_conservados':True,'informes_opciones_dinamicas':True,
            'conexion_js_ejecutada':True,'api_extension_adaptada':True,'peticiones_sitfa':0}
    (root/'pruebas_locales'/'integracion_v2.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    print(json.dumps(report))
except BaseException:
    print(json.dumps({'conexion_estado':last_status,'rutas_locales':paths}))
    raise
finally:
    stop.set();thread.join(10);bridge.close()
    if fault:raise fault[0]
