"""Recorrido de la extensión por cinco pantallas enteramente ficticias."""
import ast,asyncio,json
from pathlib import Path
import sys,tempfile,threading
from urllib.parse import parse_qsl,urlsplit
from playwright.async_api import async_playwright,expect

root=Path(__file__).resolve().parent
app=root;sys.path.insert(0,str(app))
import motor as m
from perfiles import Selection,TABS,MENUS
from lotes import Batch,BatchRunner,Dates
from puente import Bridge
from test_flujos import catalog,extended_data,workbook_bytes
from test_flujo_csmp import row as carga_row
from test_lotes import document as seguimiento_response,CATALOG
from test_poc import excel

tree=ast.parse((app/'verificar_chrome_ficticio.py').read_text(encoding='utf-8'))
master_node=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='master')
exec(compile(ast.Module(body=[master_node],type_ignores=[]),'fixture','exec'))

def form_html(screen):
    if screen=='seguimiento':return master()
    cat=catalog(screen)
    def options(name,key):return '<select name="'+name+'">'+''.join(f'<option value="{k}">{v}</option>' for k,v in cat[key].items())+'</select>'
    if screen=='carga':
        return '<html><meta charset="utf-8"><form name="MaoPpalForm" method="post" action="/SITFAWEB/MaoDAction.do">'+options('COD_TribunalDist','tribunales')+'''<input name="GLS_Tribunal" value="Tribunal ficticio"><input name="COD_Rus" value="">
          <select name="TIP_Cargo" disabled><option value="46">Cargo visual</option></select>
          <input name="FEC_Desde" value="01/09/2026"><input name="FEC_Hasta" value="30/09/2026">
          <input name="irAccion" type="submit" value="Aud.Carga Func.-"></form></html>'''
    fields=options('COD_Tribunal_sel','tribunales');scripts=''
    if screen=='litigantes':
        fields+=options('COD_EstBusqueda','estados')+'<select name="COD_Medida"><option value="-1">Todas</option><option value="4" selected>Programa ficticio</option></select>'
        fields+='<input name="TIP_Consulta" value=""><input name="RUT_Consulta" value="12345678"><input name="RUT_DvConsulta" value="9"><input name="ROL_Causa" value=""><input name="FLG_Consulta" value="0">'
        fields+='<input name="CHK_Consulta" type="checkbox"><input name="FEC_Inicio" value="01/09/2026"><input name="FEC_Fin" value="30/09/2026">'
        fields+='<input name="NUM_PaginaInforme" value="1"><input name="NUM_Total" value="3">'
        scripts='''function Seleccion(){var f=document.InformesPpalForm;f.FEC_Inicio.disabled=f.FEC_Fin.disabled=!f.CHK_Consulta.checked;}
        function Envio(){var f=document.InformesPpalForm;f.TIP_Consulta.value='2';f.FLG_Consulta.value=f.CHK_Consulta.checked?'1':'0';
          if(!f.RUT_Consulta.value){f.RUT_Consulta.value='0';f.RUT_DvConsulta.value='0';}if(!f.ROL_Causa.value)f.ROL_Causa.value='0';
          f.FEC_Inicio.disabled=f.FEC_Fin.disabled=false;return true;}Seleccion();'''
        action='Consulta Informe'
    else:
        fields=fields.replace('</select>','<option value="-1">Todos</option></select>',1)
        fields+=options('COD_Mes_sel','meses')+options('COD_Anio_Sel','anios')
        fields+=''.join(f'<input name="TIP_Consulta" type="radio" value="{i}"'+(' checked' if i==2 else '')+'>' for i in (1,2,3))
        fields+='<input name="CHK_Contrae" type="checkbox" checked><input name="FLG_Busqueda" value="1">'
        kind='Informes' if screen=='calendario_informes' else 'Medidas'
        fields+=f'<input name="Excel" type="button" id="Excel{kind}PorVencer1"><input name="Excel" type="button">'
        scripts='function cambiacheck(){var c=document.InformesPpalForm.CHK_Contrae;c.value=c.checked?1:0;}'
        action='Buscar Inf.' if kind=='Informes' else 'Buscar'
    fields+=f'<input name="irAccion" type="submit" value="{action}">'
    return '<html><meta charset="utf-8"><form name="InformesPpalForm" action="/SITFAWEB/InformesDAction.do" method="post">'+fields+'</form><script>'+scripts+'</script></html>'

async def main():
    (root/'pruebas_locales').mkdir(exist_ok=True)
    report={'datos':'ficticios','api_extension_adaptada':False,'perfil_usuario_accedido':False,'peticiones_reales_sitfa':0}
    bridge=Bridge();network=[];events=[];wire=[];current={}
    with tempfile.TemporaryDirectory(prefix='flujos_extension_',dir=root/'pruebas_locales') as tmp:
        async with async_playwright() as pw:
            executable=root/'pruebas_locales'/'chrome-pruebas'/'chrome-win64'/'chrome.exe'
            if not executable.is_file():executable=Path(pw.chromium.executable_path)
            context=await pw.chromium.launch_persistent_context(str(Path(tmp)/'perfil'),executable_path=str(executable),headless=True,
                args=['--disable-extensions-except='+str(app/'extension'),'--load-extension='+str(app/'extension')])
            try:
                async def route(r):
                    request=r.request;u=urlsplit(request.url)
                    if u.path=='/fixture':await r.fulfill(body='<html><iframe src="/fixture-form"></iframe></html>',content_type='text/html');return
                    if u.path=='/fixture-form':await r.fulfill(body=form_html('seguimiento'),content_type='text/html');return
                    if u.path=='/SITFAWEB/InformesViewAccion.do':
                        screen=next(k for k,v in MENUS.items() if v==dict(parse_qsl(u.query))['TipMenuINF'])
                        network.append(('MENU',screen));await r.fulfill(body=form_html(screen),content_type='text/html');return
                    if u.path=='/SITFAWEB/MAOViewAccion.do':
                        assert dict(parse_qsl(u.query))=={'TipMenuMAO':'29'}
                        network.append(('MENU','carga'));await r.fulfill(body=form_html('carga'),content_type='text/html');return
                    if u.path=='/SITFAWEB/MaoDAction.do':
                        fields=dict(parse_qsl(request.post_data,keep_blank_values=True))
                        assert fields['irAccion']=='Aud.Carga Func.-' and 'TIP_Cargo' not in fields
                        assert (fields['FEC_Desde'],fields['FEC_Hasta'])==('06/09/2026','05/10/2026')
                        selection=Selection(fields['COD_TribunalDist'],'','',screen='carga',start=fields['FEC_Desde'],end=fields['FEC_Hasta'])
                        current.update(selection=selection,page=1)
                        values=carga_row();values[1]=CATALOG['tribunales'][selection.tribunal]
                        inputs=''.join(f'<input name="{k}" value="{v}">' for k,v in fields.items())
                        listing='<table><tr>'+''.join(f'<td>{v}</td>' for v in values)+'</tr></table>'
                        body=f'<meta charset="utf-8"><form name="MaoPpalForm" action="/SITFAWEB/MaoDAction.do" method="post">{inputs}{listing}</form><script>function ShowExcel(){{window.open("https://familia.pjud.cl/sitfa/reportes/demo_antr.xls");}}</script>'
                        network.append(('POST','carga',selection.tribunal,'1',1))
                        await r.fulfill(body=body,content_type='text/html');return
                    if request.url==m.ENDPOINT:
                        assert 'fixture_session=local_only' in request.headers.get('cookie','')
                        fields=dict(parse_qsl(request.post_data,keep_blank_values=True));action=fields['irAccion']
                        screen={'Buscar medida':'seguimiento','Consulta Informe':'litigantes','Buscar Inf.':'calendario_informes','Buscar':'calendario_medidas'}[action]
                        assert fields['COD_Tribunal_sel'] in ('49','113')
                        tab=next(k for k,v in TABS.items() if v[0]==fields['COD_Lengueta']) if screen=='seguimiento' else ''
                        selection=Selection(fields['COD_Tribunal_sel'],tab,fields['TIP_Consulta'] if screen=='seguimiento' else '',
                            screen=screen,state=fields.get('COD_EstBusqueda','1'),month=fields.get('COD_Mes_sel'),year=fields.get('COD_Anio_Sel'))
                        page=int(fields.get(TABS[tab][1] if tab else 'NUM_PaginaInforme') or '1')
                        if screen=='litigantes':
                            if selection.state=='2':
                                assert fields['RUT_Consulta']=='0' and fields['COD_Medida']=='-1'
                                assert fields['FLG_Consulta']=='1' and fields['FEC_Inicio']=='01/10/2026' and fields['FEC_Fin']=='31/10/2026'
                            else:assert fields['RUT_Consulta']=='12345678' and fields['COD_Medida']=='4'
                        if screen.startswith('calendario_'):assert fields['TIP_Consulta']=='1' and fields['CHK_Contrae']=='1' and fields['COD_Anio_Sel']=='2026'
                        current.update(selection=selection,page=page)
                        network.append(('POST',screen,selection.tribunal,selection.state,page))
                        wire.append((screen,selection.tribunal,selection.state,fields.get('FLG_Consulta')))
                        body=seguimiento_response(selection,page) if screen=='seguimiento' else extended_data(selection,page)[1]
                        await r.fulfill(body=body,content_type='text/html');return
                    if u.path.startswith('/sitfa/reportes/'):
                        assert 'fixture_session=local_only' in request.headers.get('cookie','')
                        selection=current['selection'];page=current['page'];data=excel(page)
                        if selection.screen.startswith('calendario_'):
                            title='FECHA VENCIMIENTO' if selection.screen=='calendario_informes' else 'FEC.EGRESO PROYECTADO'
                            data=workbook_bytes(['RIT','NOMBRE MENOR',title],[[f'X-{page}-2026',f'Persona ficticia {page}',f'10/{int(selection.month):02}/{selection.year}']])
                        if selection.screen=='carga':
                            values=carga_row();values[1]=CATALOG['tribunales'][selection.tribunal]
                            data=workbook_bytes([], [values])
                        if selection.screen=='litigantes':data=data.replace(b'<th>RIT</th>',b'<th>Tipo Causa</th><th>Rol Causa</th><th>Era Causa</th>').replace(f'<td>X-{page}-2026</td>'.encode(),f'<td>X</td><td>{page}</td><td>2026</td>'.encode())
                        network.append(('XLS',selection.screen,selection.tribunal,selection.state,page))
                        await r.fulfill(body=data,content_type='application/vnd.ms-excel');return
                    await r.abort()
                await context.route('https://familia.pjud.cl/**',route)
                await context.add_cookies([{'name':'fixture_session','value':'local_only','domain':'familia.pjud.cl','path':'/','httpOnly':True,'secure':True}])
                worker=context.service_workers[0] if context.service_workers else await context.wait_for_event('serviceworker')
                extension_id=worker.url.split('/')[2]
                target=await context.new_page();await target.goto('https://familia.pjud.cl/fixture')
                ids=await worker.evaluate("async()=> (await chrome.tabs.query({url:'https://familia.pjud.cl/*'})).map(t=>t.id)");assert len(ids)==1
                connector=await context.new_page();await connector.goto(f'chrome-extension://{extension_id}/conexion.html?tab={ids[0]}')
                await connector.locator('#code').fill(bridge.connection_code);await connector.locator('#connect').click()
                await expect(connector.locator('#status')).to_contain_text('Conectado.')
                cat=await asyncio.to_thread(bridge.call,'catalog',{'screen':'seguimiento'})
                await asyncio.to_thread(BatchRunner(bridge,cat,lambda n,v:events.append((n,v)),threading.Event()).run,
                    Batch(('49','113'),('1',),('Espera',)),Path(tmp)/'descargas')
                cat=await asyncio.to_thread(bridge.call,'catalog',{'screen':'litigantes'})
                assert set(cat['estados'])=={'1','2','3'}
                runner=BatchRunner(bridge,cat,lambda n,v:events.append((n,v)),threading.Event())
                await asyncio.to_thread(runner.run,Batch(('49','113'),(),(),screen='litigantes',state='2',dates=Dates('01/10/2026','31/10/2026')),Path(tmp)/'descargas')
                await asyncio.to_thread(runner.run,Batch(('49',),(),(),screen='litigantes',state='1',keep_filters=True),Path(tmp)/'descargas')
                restored=await target.frame_locator('iframe').locator('[name="RUT_Consulta"]').input_value();assert restored=='12345678'
                for screen,month in [('calendario_informes','9'),('calendario_medidas','10')]:
                    cat=await asyncio.to_thread(bridge.call,'catalog',{'screen':screen});assert cat['pantalla']==screen and '-1' not in cat['tribunales']
                    await asyncio.to_thread(BatchRunner(bridge,cat,lambda n,v:events.append((n,v)),threading.Event()).run,
                        Batch(('49','113'),(),(),screen=screen,month=month,year='2026'),Path(tmp)/'descargas')
                    assert await target.frame_locator('iframe').locator('[name="TIP_Consulta"][value="2"]').is_checked()
                cat=await asyncio.to_thread(bridge.call,'catalog',{'screen':'seguimiento'});assert cat['pantalla']=='seguimiento'
                cat=await asyncio.to_thread(bridge.call,'catalog',{'screen':'carga'});assert cat['pantalla']=='carga'
                await asyncio.to_thread(BatchRunner(bridge,cat,lambda n,v:events.append((n,v)),threading.Event()).run,
                    Batch(('49','113'),(),(),screen='carga',dates=Dates('06/09/2026','05/10/2026')),Path(tmp)/'descargas')
                assert await target.frame_locator('iframe').locator('[name="FEC_Desde"]').input_value()=='01/09/2026'
                assert len([v for n,v in events if n=='progress'])==21
                assert len([v for n,v in events if n=='done'])==6
                for screen,trib,state,pages in [('seguimiento','49','1',3),('seguimiento','113','1',3),('litigantes','49','2',3),('litigantes','113','2',3),('litigantes','49','1',3),('calendario_informes','49','1',1),('calendario_informes','113','1',1),('calendario_medidas','49','1',1),('calendario_medidas','113','1',1)]:
                    sequence=[(e[0],e[4]) for e in network if len(e)==5 and e[1:4]==(screen,trib,state)]
                    assert sequence==[item for page in range(1,pages+1) for item in [('POST',page),('XLS',page)]]
                for trib in ('49','113'):
                    assert [(e[0],e[4]) for e in network if len(e)==5 and e[1:4]==('carga',trib,'1')]==[('POST',1),('XLS',1)]
                report.update(resultado='OK',pantallas=5,lotes=6,consultas=11,paginas=21,navegacion_entre_pantallas=True,
                    tribunales_no_marcados_excluidos=True,fechas_litigantes=True,filtros_litigantes_conservados_y_restaurados=True,
                    calendarios_mes_anio=True,carga_30_dias_y_23_columnas=True,cargo_visual_no_enviado=True,
                    radio_y_checkbox_restaurados=True,descargas_secuenciales=True)
            finally:await context.close()
    bridge.close();(root/'pruebas_locales'/'verificacion_flujos_extension.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
    print(json.dumps(report,ensure_ascii=False))

asyncio.run(main())
