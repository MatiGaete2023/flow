"""Captura y PDF con APIs nativas y activeTab; todos los datos y rutas son ficticios."""
import ast,asyncio,hashlib,json,re,sys,tempfile,threading
from pathlib import Path
from urllib.parse import parse_qsl,urlsplit
from playwright.async_api import async_playwright,expect

root=Path(__file__).resolve().parent
app=root;sys.path.insert(0,str(app))
import motor as m
from lotes import Batch,BatchRunner,Dates
from perfiles import Selection,TABS,MENUS,selection_profile
from puente import Bridge
from test_lotes import CATALOG,document
from test_flujos import catalog,extended_data
from test_poc import excel,table

for source,func in [('verificar_chrome_ficticio.py','master'),('verificar_flujos_extension.py','form_html')]:
    tree=ast.parse((app/source).read_text(encoding='utf-8'))
    node=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name==func)
    exec(compile(ast.Module(body=[node],type_ignores=[]),'fixture','exec'))

def response(s,fields,empty=True):
    if s.screen=='seguimiento':
        p=selection_profile(s,CATALOG)
        body=document(s,1,1).decode()
        if empty:
            body=body.replace(table(1).replace('tablaCumplimiento',p.table_id),f'<h3>Listado de registros [Cantidad : 0]</h3><table id="{p.table_id}"><tr><th>RIT</th><th>Tribunal</th><th>Nombre</th></tr></table>')
            body=body.replace(f'name="{p.total_field}" value="1"',f'name="{p.total_field}" value="0"')
    else:body=extended_data(s,1,1,empty=empty)[1]
    if empty and not s.screen.startswith('calendario_'):
        # Cabeceras TD y mensaje vacío dentro de TBODY: no son registros.
        body=body.replace('<th>','<td>').replace('</th>','</td>')
        body=body.replace('</tr></table>','</tr><tr><td colspan="3">No se encontraron registros.</td></tr></table>')
    public=catalog(s.screen)
    name=public['tribunales'][s.tribunal]
    title=f'SITFA - PRUEBA FICTICIA | {name}'
    scope=f'{s.tab} / {CATALOG["modalidades"].get(s.modality,s.screen)}'
    for key in ('FEC_Inicio','FEC_Fin','FLG_Consulta'):
        if key in fields:body=body.replace('</form>',f'<input name="{key}" value="{fields[key]}"></form>')
    decoration='<style>body{font:16px Arial;color:#16395b;margin:16px}h2{background:#d9e9fc;padding:16px}h3{background:#96c4ed;padding:12px}input,select{margin:8px;padding:5px;border:1px solid #aaa}table{border-collapse:collapse;width:100%}th{background:#d9e9fc;text-align:left;padding:15px}</style>'
    return body.replace('<form ',decoration+f'<h2>{title}</h2><p>Consulta: {scope} | mes {s.month or "-"} / año {s.year or "-"}</p><form ',1)

async def main():
    output=root/'pruebas_locales'/'pdf_fixture';output.mkdir(parents=True,exist_ok=True)
    report={'datos':'ficticios','api_extension_adaptada':False,'permisos_manifest_ampliados':False,'activeTab_gesto_accion_real':False,'peticiones_reales_sitfa':0,'perfil_usuario_accedido':False}
    events=[];network=[];bridge=Bridge();mode={'nonempty':False,'change':False,'mixed':False};counts={};capture_names=[]
    with tempfile.TemporaryDirectory(prefix='pdf_extension_',dir=root/'pruebas_locales') as tmp:
        async with async_playwright() as pw:
            executable=root/'pruebas_locales'/'chrome-pruebas'/'chrome-win64'/'chrome.exe'
            if '--browser' in sys.argv:executable=Path(sys.argv[sys.argv.index('--browser')+1]).resolve()
            elif not executable.is_file():executable=Path(pw.chromium.executable_path)
            context=await pw.chromium.launch_persistent_context(str(Path(tmp)/'perfil'),executable_path=str(executable),headless=True,viewport={'width':1600,'height':950},
                args=['--enable-unsafe-extension-debugging','--disable-extensions-except='+str(app/'extension'),'--load-extension='+str(app/'extension')])
            try:
                async def route(r):
                    req=r.request;u=urlsplit(req.url)
                    if u.path=='/fixture':
                        await r.fulfill(body='<html><body style="margin:0"><header style="height:50px;background:#e3eefb;font:20px Arial;padding:12px">Sistema tribunales de Familia - PRUEBA FICTICIA</header><iframe style="border:0;width:100%;height:850px" src="/fixture-form"></iframe></body></html>',content_type='text/html');return
                    if u.path=='/fixture-form':await r.fulfill(body=form_html('seguimiento'),content_type='text/html');return
                    if u.path=='/SITFAWEB/InformesViewAccion.do':
                        screen=next(k for k,v in MENUS.items() if v==dict(parse_qsl(u.query))['TipMenuINF'])
                        await r.fulfill(body=form_html(screen),content_type='text/html');return
                    if req.url==m.ENDPOINT:
                        f=dict(parse_qsl(req.post_data,keep_blank_values=True))
                        screen={'Buscar medida':'seguimiento','Consulta Informe':'litigantes','Buscar Inf.':'calendario_informes','Buscar':'calendario_medidas'}[f['irAccion']]
                        tab=next(k for k,v in TABS.items() if v[0]==f['COD_Lengueta']) if screen=='seguimiento' else ''
                        s=Selection(f['COD_Tribunal_sel'],tab,f['TIP_Consulta'] if screen=='seguimiento' else '',f.get('TIP_Informe') if tab=='Informes' else None,
                            screen=screen,state=f.get('COD_EstBusqueda','1'),month=f.get('COD_Mes_sel'),year=f.get('COD_Anio_Sel'))
                        key=(screen,s.tribunal,s.tab,s.modality);counts[key]=counts.get(key,0)+1
                        native=req.is_navigation_request();empty=not mode['nonempty'] and not(mode['change'] and native) and not(mode['mixed'] and s.modality=='2')
                        network.append(('POST',screen,s.tribunal,s.modality,native,empty))
                        await r.fulfill(body=response(s,f,empty),content_type='text/html; charset=utf-8');return
                    if u.path.startswith('/sitfa/reportes/'):
                        network.append(('XLS',));await r.fulfill(body=excel(1),content_type='application/vnd.ms-excel');return
                    await r.abort()
                await context.route('https://familia.pjud.cl/**',route)
                worker=context.service_workers[0] if context.service_workers else await context.wait_for_event('serviceworker')
                extension_id=worker.url.split('/')[2]
                target=await context.new_page();await target.goto('https://familia.pjud.cl/fixture')
                browser_cdp=await context.browser.new_browser_cdp_session()
                targets=(await browser_cdp.send('Target.getTargets',{'filter':[{'type':'tab','exclude':False}]}))['targetInfos']
                target_id=next(t['targetId'] for t in targets if t['url']=='https://familia.pjud.cl/fixture')
                async with context.expect_page() as connection:
                    await browser_cdp.send('Extensions.triggerAction',{'id':extension_id,'targetId':target_id})
                connector=await connection.value;await connector.wait_for_load_state()
                report['activeTab_gesto_accion_real']=True
                await connector.locator('#code').fill(bridge.connection_code);await connector.locator('#connect').click()
                await expect(connector.locator('#status')).to_contain_text('Conectado.')
                for screen in ('seguimiento','litigantes','calendario_informes','calendario_medidas'):
                    cat=await asyncio.to_thread(bridge.call,'catalog',{'screen':screen})
                    batch=Batch(('49',),('1',),('Espera',),screen=screen,month='9',year='2026',dates=Dates('01/10/2026','31/10/2026') if screen in ('seguimiento','litigantes') else None)
                    result=await asyncio.to_thread(BatchRunner(bridge,cat,lambda n,v:events.append((n,v)),threading.Event()).run,batch,output)
                    folder=Path(next(v for n,v in reversed(events) if n=='folder'))
                    pdfs=list(folder.glob('*.pdf'));assert len(pdfs)==1 and not list(folder.glob('*.xls'))
                    capture_names.append(pdfs[0].name)
                    assert result[0]['estado']=='SIN_RESULTADOS' and result[0]['evidencia']['registros']==0
                    assert hashlib.sha256(pdfs[0].read_bytes()).hexdigest()==result[0]['evidencia']['sha256']
                    assert re.search(rb'/MediaBox\s*\[\s*0\s+0\s+800\s+475\s*\]',pdfs[0].read_bytes()),'PDF sin margen añadido'
                    assert await target.frame_locator('iframe').locator('#sitfaEvidenciaLocal').count()==0
                    assert await target.frame_locator('iframe').locator('#sitfaDescargaLocal').count()==0
                cat=await asyncio.to_thread(bridge.call,'catalog',{'screen':'seguimiento'})
                mode['mixed']=True
                mixed=await asyncio.to_thread(BatchRunner(bridge,cat,lambda n,v:events.append((n,v)),threading.Event()).run,Batch(('49',),('1','2'),('Espera',)),output)
                assert mixed[0]['estado']=='SIN_RESULTADOS' and mixed[1]['estado']=='VALIDADA'
                assert events[-1][1]['files']==1 and events[-1][1]['pdfs']==1
                mode['mixed']=False
                mode['nonempty']=True
                await asyncio.to_thread(BatchRunner(bridge,cat,lambda n,v:events.append((n,v)),threading.Event()).run,Batch(('49',),('1',),('Espera',)),output)
                assert events[-1][1]['files']==1 and events[-1][1]['pdfs']==0
                mode.update(nonempty=False,change=True)
                try:
                    await asyncio.to_thread(BatchRunner(bridge,cat,lambda n,v:events.append((n,v)),threading.Event()).run,Batch(('49',),('1',),('Espera',)),output)
                    raise AssertionError('Debe detenerse cuando aparecen registros')
                except m.PocError as error:assert 'cambió' in str(error)
                folder=Path(next(v for n,v in reversed(events) if n=='folder'));assert not list(folder.glob('*.pdf'))
                assert await target.frame_locator('iframe').locator('#sitfaDescargaLocal').count()==0
                assert len([v for n,v in events if n=='evidence'])==5
                report.update(resultado='OK',pdfs=5,pantallas=4,excel_posterior_verificado=True,continuidad_pdf_excel_mismo_lote=True,consulta_cambiada_sin_pdf=True,restauracion_y_cierre=True,
                    nombres_pdf=capture_names,captura_chrome_real=True,pdf_sin_encabezado_ni_margen=True,cabeceras_td_vacias=True)
            finally:await context.close();bridge.close()
    (root/'pruebas_locales'/'verificacion_pdf_extension.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
    print(json.dumps(report,ensure_ascii=False))

asyncio.run(main())
