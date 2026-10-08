"""Casos funcionales ficticios: alcance, fechas, nombres, cancelacion y puente."""
import base64
import hashlib
import io
import gc
from http.client import HTTPConnection
import json
from pathlib import Path
import threading
import unittest
from PIL import Image
from urllib.parse import urlencode

import motor as m
from lotes import Batch,BatchRunner,ChromeTransport,Dates,file_base
from perfiles import Selection,TABS,selection_profile
from puente import Bridge
import test_poc as fixture

CATALOG={'tribunales':{'49':'Juzgado de Familia Tomé','113':'Jgdo. L. y G. de Laja'},
 'modalidades':{'1':'Residencia','2':'Ambulatorio','3':'FAE','4':'DCE'},
 'informes':{'1':'Por vencer','2':'Vencidos','3':'Recibidos'},
 'medidas':{'4':'Medidas por vencer','5':'Medidas vencidas'},'pestanas':list(TABS)}


def dispose_interface(app,root):
    # Las variables Tcl deben liberarse en el hilo que creó la ventana.
    app.bridge.close();root.destroy();app.__dict__.clear();gc.collect()


def document(selection,number=1,total=3):
    profile=selection_profile(selection,CATALOG)
    template={'Espera':'reslistesp','Cumplimiento':'dcelistcump','Informes':'ambinforme'}[selection.tab]
    fields={**profile.target,profile.page_field:str(number),profile.total_field:str(total)}
    source='<html><form name="InformesPpalForm" method="post" action="/SITFAWEB/InformesDAction.do">'
    source+=''.join(f'<input name="{k}" value="{v}">' for k,v in fields.items())
    source+=fixture.table(number).replace('tablaCumplimiento',profile.table_id)
    source+=f'</form><script>function ShowExcel(){{window.open("https://familia.pjud.cl/sitfa/reportes/demo_{template}.xls");}}</script></html>'
    return source.encode()


class FakeBridge:
    def __init__(self,total=3,empty_first=False):self.selection=None;self.events=[];self.total=total;self.number=1;self.empty_first=empty_first;self.queries=0
    def call(self,command,payload=None,timeout=75):
        payload=payload or {};self.events.append((command,None if command not in ('search','download') else self.number))
        if command=='capabilities':
            from contrato_integral import capabilities
            return capabilities()
        if command=='prepare':
            self.queries+=1;self.selection=Selection(**payload['selection']);self.number=1
            profile=selection_profile(self.selection,CATALOG)
            dates=payload.get('dates');fields={**profile.target,'irAccion':'Buscar medida',profile.page_field:'1',profile.total_field:str(self.total),
             'FLG_Consulta':'1' if dates else '0','FEC_Inicio':dates['start'] if dates else '', 'FEC_Fin':dates['end'] if dates else ''}
            if dates:fields['CHK_Consulta']='on'
            return {'pairs':[list(pair) for pair in fields.items()]}
        if command in ('search','evidence_open'):
            profile=selection_profile(self.selection,CATALOG);self.number=int(dict(payload['pairs'])[profile.page_field]);data=document(self.selection,self.number,self.total)
            if self.empty_first and self.queries==1:
                data=data.replace(fixture.table(self.number).replace('tablaCumplimiento',profile.table_id).encode(),f'<table id="{profile.table_id}"></table>'.encode())
                data=data.replace(f'name="{profile.total_field}" value="{self.total}"'.encode(),f'name="{profile.total_field}" value="0"'.encode())
            result={'status':200,'type':'text/html','body':base64.b64encode(data).decode()}
            if command=='evidence_open':result['digest']=hashlib.sha256(data).hexdigest()
            return result
        if command=='evidence_capture':
            out=io.BytesIO();Image.new('RGB',(900,600),'white').save(out,'PNG')
            return {'png':base64.b64encode(out.getvalue()).decode(),'digest':payload['digest']}
        if command=='download':return {'status':200,'type':'application/vnd.ms-excel','body':base64.b64encode(fixture.excel(self.number)).decode()}
        return {'ok':True}


class BatchTests(unittest.TestCase):
    setUp=fixture.PocTests.setUp;tearDown=fixture.PocTests.tearDown
    def run_batch(self,batch,bridge=None,cancel=None,callback=None):
        bridge=bridge or FakeBridge();events=[]
        def emit(name,value):
            events.append((name,value))
            if callback:callback(name,value)
        runner=BatchRunner(bridge,CATALOG,emit,cancel or threading.Event())
        result=runner.run(batch,self.temp)
        folder=Path(next(v for n,v in events if n=='folder'))
        return result,folder,bridge,events
    def test_all_tribunals_all_modalities_both_tabs(self):
        batch=Batch(tuple(CATALOG['tribunales']),tuple(CATALOG['modalidades']),('Espera','Cumplimiento'))
        self.assertEqual(len(batch.plan(CATALOG)),16)
        self.assertEqual(len(set(batch.plan(CATALOG))),16)
    def test_residencia_cumplimiento_all_tribunals(self):
        self.assertEqual(len(Batch(tuple(CATALOG['tribunales']),('1',),('Cumplimiento',)).plan(CATALOG)),2)
    def test_exact_names_requested(self):
        self.assertEqual(file_base(Selection('49','Espera','1'),CATALOG),'ESP Residencia Jdo Familia Tome')
        self.assertEqual(file_base(Selection('49','Cumplimiento','4'),CATALOG),'CUMP DCE Jdo Familia Tome')
        self.assertEqual(file_base(Selection('49','Informes','2','1'),CATALOG),'Informes Jdo Familia Tome')
    def test_medidas_vencidas_only_applies_to_cumplimiento(self):
        plan=Batch(('49',),('1',),('Espera','Cumplimiento'),'5').plan(CATALOG)
        self.assertIsNone(plan[0].report);self.assertEqual(plan[1].report,'5')
        self.assertEqual(selection_profile(plan[1],CATALOG).target['TIP_Informe'],'5')
    def test_one_folder_multiple_queries_and_numbered_pages(self):
        result,folder,bridge,events=self.run_batch(Batch(('49',),('1','4'),('Espera','Cumplimiento')))
        self.assertEqual(len(result),4);self.assertEqual(len(list(folder.glob('*.xls'))),12)
        self.assertTrue((folder/'ESP Residencia Jdo Familia Tome 1.xls').exists())
        self.assertTrue((folder/'CUMP DCE Jdo Familia Tome 3.xls').exists())
        self.assertEqual(json.loads((folder/'resumen.json').read_text(encoding='utf-8'))['estado'],'VALIDADA')
        self.assertEqual(bridge.events[-1][0],'unlock')
    def test_single_page_without_suffix(self):
        _,folder,_,_=self.run_batch(Batch(('49',),('1',),('Espera',)),FakeBridge(total=1))
        self.assertTrue((folder/'ESP Residencia Jdo Familia Tome.xls').exists())
    def test_restore_warning_preserves_validated_batch(self):
        class RestoreBridge(FakeBridge):
            def call(self,command,payload=None,timeout=75):
                if command=='unlock':return {'ok':True,'restored':False}
                return super().call(command,payload,timeout)
        result,folder,_,events=self.run_batch(Batch(('49',),('1',),('Espera',)),RestoreBridge(total=1))
        self.assertEqual(result[0]['estado'],'VALIDADA')
        self.assertEqual(json.loads((folder/'resumen.json').read_text(encoding='utf-8'))['estado'],'VALIDADA')
        self.assertEqual(events[-1][0],'warning')
        self.assertTrue((folder/'ESP Residencia Jdo Familia Tome.xls').exists())
    def test_unlock_failure_does_not_hide_download_error(self):
        class FailureBridge(FakeBridge):
            def call(self,command,payload=None,timeout=75):
                if command=='download':raise m.PocError('Descarga ficticia fallida')
                if command=='unlock':raise m.PocError('Restauración ficticia fallida')
                return super().call(command,payload,timeout)
        events=[]
        runner=BatchRunner(FailureBridge(),CATALOG,lambda n,v:events.append((n,v)),threading.Event())
        with self.assertRaisesRegex(m.PocError,'Descarga ficticia fallida'):
            runner.run(Batch(('49',),('1',),('Espera',)),self.temp)
        self.assertEqual(events[-1][0],'warning')
    def test_initial_search_is_not_repeated(self):
        for total in (1,3):
            with self.subTest(total=total):
                _,_,bridge,_=self.run_batch(Batch(('49',),('1',),('Espera',)),FakeBridge(total=total))
                self.assertEqual([name for name,_ in bridge.events if name in ('search','download')],
                    [name for _ in range(total) for name in ('search','download')])
    def test_informes_same_base_across_modalities_do_not_overwrite(self):
        _,folder,_,_=self.run_batch(Batch(('49',),('1','2'),('Informes',),'2'))
        self.assertEqual(sorted(p.name for p in folder.glob('*.xls')),[f'Informes Jdo Familia Tome {i}.xls' for i in range(1,7)])
    def test_empty_query_continues_next_query(self):
        result,folder,_,_=self.run_batch(Batch(('49',),('1','2'),('Espera',)),FakeBridge(empty_first=True))
        self.assertEqual(result[0]['estado'],'SIN_RESULTADOS');self.assertEqual(result[1]['paginas'],3)
        self.assertEqual(len(list(folder.glob('*.xls'))),3)
    def test_cancel_stops_entire_batch_before_next_page(self):
        cancel=threading.Event();bridge=FakeBridge()
        events=[]
        def emit(name,value):
            events.append((name,value))
            if name=='progress':cancel.set()
        with self.assertRaises(m.PocError):BatchRunner(bridge,CATALOG,emit,cancel).run(Batch(('49',),('1','2'),('Espera',)),self.temp)
        folder=Path(next(v for n,v in events if n=='folder'))
        self.assertEqual(len(list(folder.glob('*.xls'))),1);self.assertEqual(bridge.queries,1)
        self.assertEqual(json.loads((folder/'resumen.json').read_text(encoding='utf-8'))['estado'],'INCOMPLETA')
    def test_dates_reject_invalid_or_reversed_dates(self):
        for a,b in [('31/09/2026','30/10/2026'),('02/10/2026','01/10/2026'),('1/10/2026','30/10/2026')]:
            with self.subTest(a=a):
                with self.assertRaises(m.PocError):Dates(a,b)
        self.assertEqual(Dates('29/02/2024','01/03/2024').start,'29/02/2024')
    def test_dates_preserved_in_every_page(self):
        _,folder,_,_=self.run_batch(Batch(('49',),('3',),('Espera',),dates=Dates('01/09/2026','30/09/2026')))
        self.assertEqual(json.loads((folder/'resumen.json').read_text(encoding='utf-8'))['filtro_fecha']['desde'],'01/09/2026')
    def test_wrong_date_flag_stops_before_search(self):
        class Wrong(FakeBridge):
            def call(self,command,payload=None,timeout=75):
                result=super().call(command,payload,timeout)
                if command=='prepare':result['pairs']=[(k,'0' if k=='FLG_Consulta' else v) for k,v in result['pairs']]
                return result
        bridge=Wrong()
        with self.assertRaises(m.PocError):self.run_batch(Batch(('49',),('3',),('Espera',),dates=Dates('01/09/2026','30/09/2026')),bridge)
        self.assertFalse(any(n=='search' for n,p in bridge.events))


class BridgeTests(unittest.TestCase):
    def setUp(self):self.bridge=Bridge();self.origin='chrome-extension://'+'a'*32
    def tearDown(self):self.bridge.close()
    def request(self,path,body=None,code=None,origin=None,omit_origin=False,extension=None,method=None):
        c=HTTPConnection('127.0.0.1',self.bridge.port,timeout=10)
        headers={'Origin':origin or self.origin,'X-Sitfa-Code':self.bridge.code if code is None else code}
        if omit_origin:headers.pop('Origin')
        if extension is not None:headers['X-Sitfa-Extension']=extension
        if body is not None:headers['Content-Type']='application/json'
        c.request(method or ('GET' if body is None else 'POST'),path,body=None if body is None else json.dumps(body),headers=headers)
        response=c.getresponse();status=response.status;raw=response.read();data=json.loads(raw) if raw else {};c.close();return status,data
    def test_extension_without_origin_connects_and_gets_a_task(self):
        self.assertEqual(self.request('/connect',{},omit_origin=True,extension='a'*32)[0],200)
        results=[]
        worker=threading.Thread(target=lambda:results.append(self.bridge.call('catalog',timeout=3)))
        worker.start();status,task=self.request('/next',omit_origin=True,extension='a'*32)
        self.assertEqual(status,200)
        self.assertEqual(self.request('/complete',{'id':task['id'],'ok':True,'result':CATALOG},omit_origin=True,extension='a'*32)[0],200)
        worker.join(5);self.assertEqual(results,[CATALOG])
    def test_missing_origin_still_requires_extension_identity(self):
        self.assertEqual(self.request('/connect',{},omit_origin=True), (403,{'error':'EXTENSION_ORIGIN'}))
    def test_declared_extension_does_not_replace_secret(self):
        self.assertEqual(self.request('/connect',{},omit_origin=True,extension='a'*32,code='0'*32), (403,{'error':'CONNECTION_CODE'}))
        self.assertFalse(self.bridge.connected)
    def test_web_origin_cannot_spoof_declared_extension(self):
        self.assertEqual(self.request('/connect',{},origin='https://familia.pjud.cl',extension='a'*32)[0],403)
        self.assertEqual(self.request('/connect',{},origin='null',extension='a'*32)[0],403)
    def test_present_origin_must_match_declared_extension(self):
        self.assertEqual(self.request('/connect',{},extension='b'*32)[0],403)
    def test_extension_identity_pinned_with_and_without_origin(self):
        self.request('/connect',{},omit_origin=True,extension='a'*32)
        self.assertEqual(self.request('/connect',{},extension='a'*32)[0],200)
        self.assertEqual(self.request('/connect',{},omit_origin=True,extension='b'*32), (403,{'error':'EXTENSION_CHANGED'}))
    def test_invalid_declared_extension_refused(self):
        self.assertEqual(self.request('/connect',{},omit_origin=True,extension='invalid')[0],403)
    def test_preflight_accepts_only_extension_origins(self):
        self.assertEqual(self.request('/connect',method='OPTIONS')[0],204)
        self.assertEqual(self.request('/connect',origin='https://familia.pjud.cl',method='OPTIONS')[0],403)
    def test_followup_error_distinguished_from_connection_error(self):
        self.request('/connect',{});errors=[]
        def work():
            try:self.bridge.call('catalog',timeout=3)
            except m.PocError as exc:errors.append(str(exc))
        worker=threading.Thread(target=work);worker.start();_,task=self.request('/next')
        self.request('/complete',{'id':task['id'],'ok':False,'error':'FOLLOWUP_REQUIRED'})
        worker.join(5);self.assertEqual(len(errors),1)
        self.assertIn('el enlace con chrome funciona',errors[0].lower())
        self.assertTrue(self.bridge.connected)
    def test_refuses_website_and_wrong_code(self):
        self.assertEqual(self.request('/connect',{},origin='https://familia.pjud.cl')[0],403)
        self.assertEqual(self.request('/connect',{},code='wrong')[0],403)
    def test_pins_origin_after_connection(self):
        self.assertEqual(self.request('/connect',{})[0],200)
        self.assertEqual(self.request('/connect',{},origin='chrome-extension://'+'b'*32)[0],403)
    def test_round_trip_without_credentials(self):
        self.request('/connect',{});results=[]
        worker=threading.Thread(target=lambda:results.append(self.bridge.call('catalog',timeout=3)))
        worker.start();status,task=self.request('/next')
        self.assertEqual(task['command'],'catalog');self.assertEqual(status,200)
        self.assertEqual(self.request('/complete',{'id':task['id'],'ok':True,'result':CATALOG})[0],200)
        worker.join(5);self.assertEqual(results,[CATALOG])
    def test_rejects_mutation_task(self):
        with self.assertRaises(m.PocError):self.bridge.call('Grabar')
    def test_invalid_completion_is_rejected_without_losing_connection(self):
        self.request('/connect',{})
        for body in ({'id':[],'ok':True},{'id':{},'ok':True},{'id':'ficticio','ok':'true'}):
            with self.subTest(body=body):
                self.assertEqual(self.request('/complete',body)[0],400)
        self.assertTrue(self.bridge.connected)
    def test_expired_tasks_do_not_delay_active_task(self):
        self.request('/connect',{})
        with self.assertRaises(m.PocError):self.bridge.call('catalog',timeout=0.01)
        results=[]
        worker=threading.Thread(target=lambda:results.append(self.bridge.call('catalog',timeout=3)))
        worker.start()
        try:
            status,task=self.request('/next')
            self.assertEqual(status,200);self.assertIn('id',task)
            self.assertEqual(self.request('/complete',{'id':task['id'],'ok':True,'result':CATALOG})[0],200)
        finally:worker.join(5)
        self.assertEqual(results,[CATALOG])
    def test_duplicate_completion_cannot_replace_result(self):
        self.request('/connect',{})
        event=threading.Event()
        entry={'event':event,'result':None}
        with self.bridge.lock:self.bridge.pending['ficticio']=entry
        first={'id':'ficticio','ok':True,'result':CATALOG}
        self.assertEqual(self.request('/complete',first)[0],200)
        self.assertEqual(self.request('/complete',{'id':'ficticio','ok':False})[0],409)
        self.assertEqual(entry['result'],first)


class TransportTests(unittest.TestCase):
    def test_invalid_response_has_controlled_message(self):
        for result in (None,[],{'body':None},{'body':'YQ==','status':200,'type':None},
                       {'body':'!!','status':200,'type':'text/html'},
                       {'body':'YQ==','status':403,'type':'text/html'}):
            with self.subTest(result=result):
                with self.assertRaises(m.PocError):ChromeTransport.body(result)


class InterfaceTests(unittest.TestCase):
    def test_bulk_selection_and_compact_view(self):
        import tkinter as tk
        from descargador import Application
        root=tk.Tk();app=Application(root,start_worker=False)
        try:
            self.assertEqual(str(app.download_button['state']),'disabled')
            app.process_event('connected','Chrome ficticio');app.process_event('catalog',CATALOG)
            self.assertEqual(len(app.batch().plan(CATALOG)),16)
            app.select_tribunals(('49',));app.modality.set('Residencia');app.tab.set('Cumplimiento');app.refresh()
            self.assertEqual(len(app.batch().plan(CATALOG)),1)
            app.use_dates.set(True);app.from_date.set('01/10/2026');app.to_date.set('31/10/2026');app.refresh()
            self.assertEqual(app.batch().dates.end,'31/10/2026')
            root.geometry('880x600');root.update();app.canvas.yview_moveto(1);root.update()
            self.assertLess(app.tree.winfo_rooty()+app.tree.winfo_height(),root.winfo_rooty()+root.winfo_height())
            app.process_event('warning','Revisa el formulario de SITFA.')
            self.assertEqual(app.status.get(),'Revisa el formulario de SITFA.')
        finally:dispose_interface(app,root)


if __name__=='__main__':unittest.main()
