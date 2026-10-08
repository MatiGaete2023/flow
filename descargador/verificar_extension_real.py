"""API reales de extension + HTTP real. No usa el perfil ni la sesion del usuario."""
import ast
import asyncio
import json
from pathlib import Path
import sys
import tempfile
import threading
from urllib.parse import parse_qsl

from playwright.async_api import async_playwright,expect

root=Path(__file__).resolve().parent
app=root
sys.path.insert(0,str(app))
import motor as m
from puente import Bridge
from perfiles import TABS,Selection
from lotes import Batch,BatchRunner
from test_lotes import CATALOG,document
from test_poc import excel

# Reutiliza solo el formulario ficticio; no ejecuta los lotes de ese archivo.
tree=ast.parse((app/'verificar_chrome_ficticio.py').read_text(encoding='utf-8'))
master_node=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='master')
exec(compile(ast.Module(body=[master_node],type_ignores=[]),'fixture', 'exec'))

async def main():
    report={'datos':'ficticios','perfil_usuario_accedido':False,'api_extension_adaptada':False,
            'peticiones_reales_sitfa':0}
    bridge=Bridge();old=None;observed=[]
    browser_path=root/'pruebas_locales'/'chrome-pruebas'/'chrome-win64'/'chrome.exe'
    (root/'pruebas_locales').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='extension_fixture_',dir=root/'pruebas_locales') as tmp:
        legacy_scope={'__name__':'puente_anterior'}
        exec((app/'pruebas'/'puente_origin_fixture.py').read_text(encoding='utf-8'),legacy_scope)
        old=legacy_scope['Bridge']()
        async with async_playwright() as pw:
            if not browser_path.is_file():browser_path=Path(pw.chromium.executable_path)
            if not browser_path.is_file():raise RuntimeError('Instala Chromium de pruebas con python -m playwright install chromium')
            context=await pw.chromium.launch_persistent_context(
                user_data_dir=str(Path(tmp)/'perfil'),executable_path=str(browser_path),headless=True,
                args=['--disable-extensions-except='+str(app/'extension'),'--load-extension='+str(app/'extension')])
            try:
                current={'page':1,'selection':None};sequence=[]
                async def route(r):
                    if r.request.url==m.ENDPOINT:
                        fields=dict(parse_qsl(r.request.post_data,keep_blank_values=True))
                        tab=next(k for k,v in TABS.items() if v[0]==fields['COD_Lengueta'])
                        current['selection']=Selection(fields['COD_Tribunal_sel'],tab,fields['TIP_Consulta'])
                        current['page']=int(fields[TABS[tab][1]] or '1')
                        sequence.append(('POST',current['page']))
                        await r.fulfill(body=document(current['selection'],current['page']),content_type='text/html')
                    elif '/sitfa/reportes/' in r.request.url:
                        sequence.append(('XLS',current['page']))
                        await r.fulfill(body=excel(current['page']),content_type='application/vnd.ms-excel')
                    elif r.request.url.endswith('/fixture-form'):
                        await r.fulfill(body=master(),content_type='text/html')
                    elif r.request.url.endswith('/fixture'):
                        await r.fulfill(body='<html><iframe src="/fixture-form"></iframe></html>',content_type='text/html')
                    else:
                        await r.fulfill(body='<html><p>Inicio ficticio</p></html>',content_type='text/html')
                await context.route('https://familia.pjud.cl/**',route)
                def network(request):
                    if request.url.startswith(('http://127.0.0.1:'+str(bridge.port), 'http://127.0.0.1:'+str(old.port))):
                        observed.append({'metodo':request.method,'ruta':request.url.rsplit('/',1)[-1],
                            'origin_presente':'origin' in request.headers,
                            'identidad_presente':'x-sitfa-extension' in request.headers})
                context.on('request',network)
                worker=context.service_workers[0] if context.service_workers else await context.wait_for_event('serviceworker')
                extension_id=worker.url.split('/')[2]
                target=await context.new_page();await target.goto('https://familia.pjud.cl/fixture-inicio')
                tab_ids=await worker.evaluate("async()=> (await chrome.tabs.query({url:'https://familia.pjud.cl/*'})).map(t=>t.id)")
                assert len(tab_ids)==1
                url=f'chrome-extension://{extension_id}/conexion.html?tab={tab_ids[0]}'
                connector=await context.new_page();await connector.goto(url)

                # La version anterior autentica POST pero rechaza GET nativo sin Origin.
                legacy=await connector.evaluate('''async data => {
                  const base='http://127.0.0.1:'+data.port;
                  const post=await fetch(base+'/connect',{method:'POST',headers:{'X-Sitfa-Code':data.code,'Content-Type':'application/json'},body:'{}'});
                  const get=await fetch(base+'/next',{headers:{'X-Sitfa-Code':data.code}});
                  return {post:post.status,get:get.status};
                }''',{'port':old.port,'code':old.code})
                assert legacy=={'post':200,'get':403},'El caso anterior no reproduce el rechazo esperado'
                report['regresion_anterior']=legacy

                # Codigo correcto conecta incluso antes de entrar a Seguimiento.
                await connector.locator('#code').fill(bridge.connection_code)
                await connector.locator('#connect').click()
                await expect(connector.locator('#status')).to_contain_text('Conectado.')
                assert bridge.connected
                report['conexion_antes_de_seguimiento']=True
                try:
                    await asyncio.to_thread(bridge.call,'catalog',timeout=8)
                    raise AssertionError('No debe hallar Seguimiento en el inicio ficticio')
                except m.PocError as exc:
                    assert 'no se encuentra Seguimiento' in str(exc)
                assert bridge.connected
                assert 'no se encuentra' in await connector.locator('#status').text_content()
                report['seguimiento_ausente_no_confunde_codigo']=True

                # Inyeccion real en un iframe y catalogo real, sin adaptar scripting.
                await target.goto('https://familia.pjud.cl/fixture')
                await target.frame_locator('iframe').locator('form').wait_for()
                catalog=await asyncio.to_thread(bridge.call,'catalog',timeout=8)
                assert catalog['tribunales']==CATALOG['tribunales']
                assert set(catalog['modalidades'])=={'1','2','3','4'}
                assert set(catalog['informes'])=={'1','2','3'}
                await asyncio.to_thread(bridge.call,'lock',timeout=8)
                restored=await asyncio.to_thread(bridge.call,'unlock',timeout=8)
                assert restored['restored'] is True
                assert await target.frame_locator('iframe').locator('#sitfaDescargaLocal').count()==0
                report['catalogo_y_bloqueo_api_reales_en_iframe']=True
                # Un filtro que desaparece no debe ocultar la restauración parcial.
                await asyncio.to_thread(bridge.call,'lock',timeout=8)
                await target.frame_locator('iframe').locator('[name="COD_Tribunal_sel"]').evaluate('(e)=>e.remove()')
                restored=await asyncio.to_thread(bridge.call,'unlock',timeout=8)
                assert restored['restored'] is False
                assert await target.frame_locator('iframe').locator('#sitfaDescargaLocal').count()==0
                await target.goto('https://familia.pjud.cl/fixture')
                await target.frame_locator('iframe').locator('form').wait_for()
                await asyncio.to_thread(bridge.call,'catalog',timeout=8)
                report['restauracion_parcial_informada_y_bloqueo_retirado']=True
                events=[]
                runner=BatchRunner(bridge,catalog,lambda n,v:events.append((n,v)),threading.Event())
                await asyncio.to_thread(runner.run,Batch(('49',),('1',),('Espera',)),Path(tmp)/'descargas_ficticias')
                assert len([v for n,v in events if n=='progress'])==3
                assert sequence==[('POST',1),('XLS',1),('POST',2),('XLS',2),('POST',3),('XLS',3)]
                report['lote_tres_paginas_api_reales']=True

                # Codigo incorrecto: mensaje especifico, no error de Seguimiento.
                wrong=Bridge()
                try:
                    p=await context.new_page();await p.goto(url)
                    bad=wrong.connection_code[:-1]+('0' if wrong.connection_code[-1]!='0' else '1')
                    await p.locator('#code').fill(bad);await p.locator('#connect').click()
                    await expect(p.locator('#status')).to_contain_text('rechazó el código')
                    assert not wrong.connected
                    report['codigo_incorrecto_rechazado']=True
                    await p.close()
                finally:wrong.close()

                # Una pestaña cerrada no interrumpe ni confunde el enlace local.
                await target.close()
                try:
                    await asyncio.to_thread(bridge.call,'catalog',timeout=8)
                    raise AssertionError('Debe informar pestaña ausente')
                except m.PocError as exc:
                    assert 'no permite acceder' in str(exc)
                assert bridge.connected
                report['pestana_cerrada_distinguida']=True
                # Descargador cerrado: mensaje de contacto local.
                closed=Bridge();closed_code=closed.connection_code;closed.close()
                p=await context.new_page();await p.goto(url)
                await p.locator('#code').fill(closed_code);await p.locator('#connect').click()
                await expect(p.locator('#status')).to_contain_text('No se pudo contactar')
                report['descargador_cerrado_distinguido']=True
                report['cabeceras_observadas']=observed
                assert any(x['metodo']=='GET' and not x['origin_presente'] and x['identidad_presente'] for x in observed)
                report['resultado']='OK'
            finally:await context.close()
    bridge.close()
    if old:old.close()
    (root/'pruebas_locales'/'verificacion_extension_real.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
    print(json.dumps({k:v for k,v in report.items() if k!='cabeceras_observadas'},ensure_ascii=False))

asyncio.run(main())
