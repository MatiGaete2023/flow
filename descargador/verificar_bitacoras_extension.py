"""Extensión real en Chromium aislado, con servidor RUS enteramente ficticio."""
import ast,asyncio,json,sys,tempfile,threading
from pathlib import Path
from urllib.parse import urlsplit,parse_qsl
from playwright.async_api import async_playwright,expect
import motor as m
from puente import Bridge
from lotes import Batch
from lectura_bitacoras import DiaryRunner
from perfiles import Selection,TABS
from test_lectura_bitacoras import Bridge as FixtureBridge,popup
from test_lotes import CATALOG

root=Path(__file__).resolve().parent
tree=ast.parse((root/'verificar_chrome_ficticio.py').read_text(encoding='utf-8'))
node=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='master')
exec(compile(ast.Module(body=[node],type_ignores=[]),'fixture','exec'))

async def main():
    bridge=Bridge();fixture=FixtureBridge();seen=[]
    try:
        with tempfile.TemporaryDirectory(prefix='bitacoras_extension_') as tmp:
            async with async_playwright() as pw:
                executable=root/'pruebas_locales'/'chrome-pruebas'/'chrome-win64'/'chrome.exe'
                if '--browser' in sys.argv:executable=Path(sys.argv[sys.argv.index('--browser')+1]).resolve()
                elif not executable.is_file():executable=Path(pw.chromium.executable_path)
                context=await pw.chromium.launch_persistent_context(str(Path(tmp)/'perfil'),executable_path=str(executable),headless=True,
                    args=['--disable-extensions-except='+str(root/'extension'),'--load-extension='+str(root/'extension')])
                try:
                    async def route(r):
                        request=r.request;url=urlsplit(request.url)
                        if url.path=='/fixture':await r.fulfill(body='<html><iframe src="/fixture-form"></iframe></html>',content_type='text/html');return
                        if url.path=='/fixture-form':await r.fulfill(body=master(),content_type='text/html');return
                        if url.path=='/SITFAWEB/InformesDAction.do':
                            fields=dict(parse_qsl(request.post_data,keep_blank_values=True))
                            assert fields['irAccion']=='Buscar medida' and request.method=='POST'
                            tab=next(k for k,v in TABS.items() if v[0]==fields['COD_Lengueta'])
                            page_field=TABS[tab][1]
                            fields[page_field]=fields.get(page_field) or '1'
                            fixture.selection=Selection(fields['COD_Tribunal_sel'],tab,fields['TIP_Consulta'])
                            fixture.queries+=1
                            result=fixture.call('search',{'pairs':list(fields.items())})
                            import base64
                            body=base64.b64decode(result['body'])
                            encoding='windows-1252' if fields['TIP_Consulta'] in ('1','3') else 'utf-8'
                            await r.fulfill(body=body.decode('utf-8').encode(encoding),content_type='text/html; charset='+encoding);return
                        if url.path=='/SITFAWEB/IrPopUpInformesAccion.do':
                            params=dict(parse_qsl(url.query));assert request.method=='GET' and params['tipo_popUp']=='12'
                            seen.append(params)
                            await r.fulfill(body=popup(params),content_type='text/html; charset=utf-8');return
                        raise AssertionError('Petición fuera del contrato de lectura ficticio: '+url.path)
                    await context.route('https://familia.pjud.cl/**',route)
                    worker=context.service_workers[0] if context.service_workers else await context.wait_for_event('serviceworker')
                    extension_id=worker.url.split('/')[2]
                    page=await context.new_page();await page.goto('https://familia.pjud.cl/fixture')
                    ids=await worker.evaluate("async()=> (await chrome.tabs.query({url:'https://familia.pjud.cl/*'})).map(t=>t.id)")
                    connector=await context.new_page();await connector.goto(f'chrome-extension://{extension_id}/conexion.html?tab={ids[0]}')
                    await connector.locator('#code').fill(bridge.connection_code);await connector.locator('#connect').click()
                    await expect(connector.locator('#status')).to_contain_text('Conectado.')
                    catalog=await asyncio.to_thread(bridge.call,'catalog',{'screen':'seguimiento'})
                    result=await asyncio.to_thread(DiaryRunner(bridge,catalog,lambda *a:None,threading.Event()).run,
                        Batch(('49','113'),('1','2','3','4'),('Espera','Cumplimiento')),Path(tmp)/'copias')
                    assert result['estado']=='COMPLETA' and result['leidas']==32,result
                    assert len(seen)==32 and {p['TIP_Consulta'] for p in seen}=={'1','2','3','4'}
                    report={'resultado':'OK','datos':'ficticios','modalidades':4,'tribunales':2,'pestanas':2,
                            'paginas_por_consulta':2,'bitacoras':32,'codificaciones':['utf-8','windows-1252'],
                            'peticiones_reales_rus':0,'escrituras_rus':0}
                    out=root/'pruebas_locales';out.mkdir(exist_ok=True)
                    (out/'verificacion_bitacoras_extension.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
                    print(json.dumps(report))
                finally:await context.close()
    finally:bridge.close()

if __name__=='__main__':asyncio.run(main())
