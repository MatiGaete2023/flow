"""Nuevas pantallas y selección múltiple; datos ficticios, sin acceder a SITFA."""
import base64
from collections import Counter
from dataclasses import replace
import io
import json
from pathlib import Path
import threading
import unittest
from urllib.parse import urlencode
from openpyxl import Workbook

import motor as m
from perfiles import Selection,selection_profile,response_profile,TABS
from lotes import Batch,BatchRunner,Dates,file_base
from test_lotes import CATALOG as OLD_CATALOG
from test_lotes import dispose_interface
import test_poc as old

def catalog(screen):
    return {**OLD_CATALOG,'pantalla':screen,
        'tribunales':{**OLD_CATALOG['tribunales'],'777':'Tribunal ficticio no elegido'},
        'estados':{'1':'Activa','2':'Terminada','3':'Sin efecto'},
        'meses':{'9':'Septiembre','10':'Octubre'},'anios':{'2026':'2026'}}

def extended_data(selection,number=1,total=3,empty=False):
    cat=catalog(selection.screen);profile=selection_profile(selection,cat)
    pairs=list({**profile.target,'irAccion':profile.action}.items())
    if profile.page_field:pairs.extend([(profile.page_field,str(number)),(profile.total_field,str(total))])
    fields=''.join(f'<input name="{k}" value="{v}">' for k,v in profile.target.items())
    if selection.screen=='litigantes':
        fields=fields.replace('name="TIP_Consulta" value="2"','name="TIP_Consulta" value=""')
        fields+=f'<input name="NUM_PaginaInforme" value="{number}"><input name="NUM_Total" value="{0 if empty else total}">'
        listing=old.table(number).replace('id="tablaCumplimiento"','') if not empty else '<table><tr><th>RIT</th><th>Nombre</th></tr></table>'
        suffix='BusquedaLit';script='if(Cantidad>0)'
    else:
        fields=fields.replace('<input name="TIP_Consulta" value="1">',''.join(f'<input type="radio" name="TIP_Consulta" value="{i}"'+(' checked' if i==1 else '')+'>' for i in (1,2,3)))
        fields+='<input name="Excel" type="button"><input name="Excel" type="button">'
        listing=''.join(f'<table id="{t}"><tbody>'+('' if empty else '<tr><td>Registro ficticio</td></tr>')+'</tbody></table>' for t in ('TablaCalendarioSemana','TablaCalendarioDia'))
        suffix='InfoPorVencer' if selection.screen=='calendario_informes' else 'MedPorVencer'
        script='var excel="disponible";if(excel!="null")'
    source=f'<html><meta charset="utf-8"><form name="InformesPpalForm" method="post" action="/SITFAWEB/InformesDAction.do">{fields}{listing}</form><script>'
    if selection.screen.startswith('calendario_') and not empty:
        source+="function RellenaCalendario(){Ritxdias[dia]='X-1-2026';Nombresxdias[dia]='Persona ficticia 1';}"
    source+='function ShowExcel(){'+script+'{window.open("https://familia.pjud.cl/sitfa/reportes/demo_'+suffix+'.xls");}}</script></html>'
    return pairs,source

def workbook_bytes(header,rows):
    book=Workbook();sheet=book.active;sheet.append(header)
    for row in rows:sheet.append(row)
    out=io.BytesIO();book.save(out);book.close();return out.getvalue()

class NewFlows(unittest.TestCase):
    setUp=old.PocTests.setUp;tearDown=old.PocTests.tearDown
    def test_litigant_state_and_pages_are_part_of_contract(self):
        s=Selection('49','','',screen='litigantes',state='2');pairs,source=extended_data(s)
        profile,first=response_profile(source,pairs,s,catalog(s.screen))
        self.assertEqual(first.total,3);self.assertEqual(profile.total_field,'NUM_Total')
        for changed in ({'COD_EstBusqueda':'1'},{'COD_Lengueta':'tdEspera'},{'irAccion':'Grabar'}):
            with self.subTest(changed=changed):
                with self.assertRaises(m.PocError):m.query_pairs(urlencode({**dict(pairs),**changed}),profile)
        with self.assertRaises(m.PocError):m.search_page(source,pairs,expected_page=2,profile=profile)
    def test_monthly_calendar_templates_and_periods_do_not_cross(self):
        for screen in ('calendario_informes','calendario_medidas'):
            s=Selection('49','','',screen=screen,month='9',year='2026');pairs,source=extended_data(s)
            profile,first=response_profile(source,pairs,s,catalog(screen))
            self.assertEqual((first.current,first.total),(1,1))
            for extra in ({'TIP_Consulta':'2'},{'COD_Mes_sel':'10'},{'NUM_PaginaInforme':'2'},{'irAccion':'Grabar'}):
                with self.assertRaises(m.PocError):m.query_pairs(urlencode({**dict(pairs),**extra}),profile)
            with self.assertRaises(m.PocError):response_profile(source.replace('InfoPorVencer','MedPorVencer') if screen=='calendario_informes' else source.replace('MedPorVencer','InfoPorVencer'),pairs,s,catalog(screen))
    def test_calendar_literal_records_are_complete_and_not_executed(self):
        s=Selection('49','','',screen='calendario_informes',month='9',year='2026');pairs,source=extended_data(s)
        for changed in (source.replace("Nombresxdias[dia]='Persona ficticia 1';",''),source.replace("Nombresxdias[dia]='Persona ficticia 1';",'Nombresxdias[dia]=funcion();')):
            with self.assertRaises(m.PocError):response_profile(changed,pairs,s,catalog(s.screen))
    def test_new_screens_accept_empty_results_but_not_login(self):
        for screen in ('litigantes','calendario_informes','calendario_medidas'):
            s=Selection('49','','',screen=screen,month='9',year='2026');pairs,source=extended_data(s,empty=True)
            self.assertIsNone(response_profile(source,pairs,s,catalog(screen))[1])
            with self.assertRaises(m.PocError):response_profile('<input type="password">',pairs,s,catalog(screen))
    def test_radio_groups_do_not_allow_duplicate_private_fields(self):
        root=m.root_html('<form><input name="TIP_Consulta" type="radio" value="1"><input name="TIP_Consulta" type="radio" value="2" checked><input name="Excel" type="button"><input name="Excel" type="button"></form>')
        self.assertEqual(m.fields_from_form(root)['TIP_Consulta'],'2')
        for source in ('<form><input name="RUT_Consulta"><input name="RUT_Consulta"></form>', '<form><input name="TIP_Consulta" type="radio"><input name="TIP_Consulta" type="hidden"></form>'):
            with self.assertRaises(m.PocError):m.fields_from_form(m.root_html(source))
    def test_selected_subset_excludes_unmarked_tribunal_in_every_flow(self):
        for screen in ('seguimiento','litigantes','calendario_informes','calendario_medidas'):
            batch=Batch(('49','113'),('1',) if screen=='seguimiento' else (),('Espera','Cumplimiento') if screen=='seguimiento' else (),screen=screen,month='9',year='2026')
            plan=batch.plan(catalog(screen));self.assertEqual({s.tribunal for s in plan},{'49','113'})
            self.assertNotIn('777',{s.tribunal for s in plan})
    def test_new_screen_rejects_stale_catalog_unknown_state_and_month(self):
        with self.assertRaises(m.PocError):Batch(('49',),(),(),screen='litigantes').plan(catalog('seguimiento'))
        with self.assertRaises(m.PocError):Batch(('49',),(),(),screen='litigantes',state='9').plan(catalog('litigantes'))
        with self.assertRaises(m.PocError):Batch(('49',),(),(),screen='calendario_informes',month='12',year='2026').plan(catalog('calendario_informes'))
    def test_names_preserve_followup_and_add_new_screen_labels(self):
        self.assertEqual(file_base(Selection('49','Espera','1'),OLD_CATALOG),'ESP Residencia Jdo Familia Tome')
        self.assertEqual(file_base(Selection('49','','',screen='calendario_informes'),catalog('calendario_informes')),'Informes Jdo Familia Tome')
        self.assertEqual(file_base(Selection('49','','',screen='calendario_medidas'),catalog('calendario_medidas')),'Medidas por vencer Jdo Familia Tome')
        self.assertEqual(file_base(Selection('49','','',screen='litigantes',state='2'),catalog('litigantes')),'Litigantes Terminada Jdo Familia Tome')
    def test_calendar_name_minor_not_center_and_split_litigant_case(self):
        data=workbook_bytes(['RIT','NOMBRE MENOR','NOMBRE CENTRO'],[['X-1-2026','Persona ficticia','Centro ficticio']])
        self.assertEqual(m.read_excel(data).records,Counter({('X-1-2026','PERSONA FICTICIA'):1}))
        for roll,year in [('17','2026'),(17.0,2026.0)]:
            data=workbook_bytes(['Tipo Causa','Rol Causa','Era Causa','Nombre'],[['P',roll,year,'Persona ficticia']])
            self.assertEqual(m.read_excel(data).records,Counter({('P-17-2026','PERSONA FICTICIA'):1}))
    def test_ambiguous_or_invalid_split_case_excel_rejected(self):
        for header,row in [(['RIT','NOMBRE','NOMBRE MENOR'],['X-1-2026','A','B']),(['RIT','RIT','NOMBRE'],['X-1-2026','X-2-2026','A']),(['Tipo Causa','Rol Causa','Era Causa','Nombre'],['P','17.5','2026','A'])]:
            with self.assertRaises(m.PocError):m.read_excel(workbook_bytes(header,[row]))
    def test_stale_calendar_excel_is_still_rejected(self):
        data=workbook_bytes(['RIT','NOMBRE MENOR'],[['X-1-2026','Persona ficticia']])
        expected=m.SearchPage(1,1,'',Counter({('X-1-2026','PERSONA FICTICIA'):1}),{})
        _,digest=m.verified_download(data,expected,set(),[])
        with self.assertRaises(m.PocError):m.verified_download(data,expected,{digest},[])
        with self.assertRaises(m.PocError):m.verified_download(workbook_bytes(['RIT','NOMBRE MENOR'],[['X-2-2026','Persona ficticia']]),expected,set(),[])

class FakeNewBridge:
    def __init__(self,cat):self.catalog=cat;self.selection=None;self.page=1;self.events=[]
    def call(self,command,payload=None,timeout=75):
        payload=payload or {};self.events.append(command)
        if command=='prepare':
            self.selection=Selection(**payload['selection']);pairs,_=extended_data(self.selection)
            if self.selection.screen=='litigantes':
                pairs+=[('FLG_Consulta','1' if payload.get('dates') else '0')]
                if payload.get('dates'):pairs+=[('FEC_Inicio',payload['dates']['start']),('FEC_Fin',payload['dates']['end']),('CHK_Consulta','on')]
            return {'pairs':[list(p) for p in pairs]}
        if command=='search':
            profile=selection_profile(self.selection,self.catalog);self.page=int(dict(payload['pairs']).get(profile.page_field,'1'))
            _,source=extended_data(self.selection,self.page)
            data=source.encode()
        elif command=='download':
            data=old.excel(self.page)
            if self.selection.screen.startswith('calendario_'):
                title='FECHA VENCIMIENTO' if self.selection.screen=='calendario_informes' else 'FEC.EGRESO PROYECTADO'
                data=workbook_bytes(['RIT','NOMBRE MENOR',title],[[f'X-{self.page}-2026',f'Persona ficticia {self.page}',f'10/{int(self.selection.month):02}/{self.selection.year}']])
            else:data=data.replace(b'<th>RIT</th>',b'<th>Tipo Causa</th><th>Rol Causa</th><th>Era Causa</th>').replace(f'<td>X-{self.page}-2026</td>'.encode(),f'<td>X</td><td>{self.page}</td><td>2026</td>'.encode())
        else:return {'ok':True}
        return {'status':200,'type':'application/vnd.ms-excel' if command=='download' else 'text/html','body':base64.b64encode(data).decode()}

class NewBatchTests(unittest.TestCase):
    setUp=old.PocTests.setUp;tearDown=old.PocTests.tearDown
    def test_selected_tribunals_download_in_same_folder_for_each_new_flow(self):
        for screen,pages in [('litigantes',6),('calendario_informes',2),('calendario_medidas',2)]:
            cat=catalog(screen);bridge=FakeNewBridge(cat);events=[]
            result=BatchRunner(bridge,cat,lambda n,v:events.append((n,v)),threading.Event()).run(Batch(('49','113'),(),(),screen=screen,month='9',year='2026'),self.temp)
            folder=Path(next(v for n,v in events if n=='folder'))
            self.assertEqual(len(result),2);self.assertEqual(len(list(folder.glob('*.xls'))),pages)
            self.assertEqual(bridge.events[-1],'unlock')
            self.assertEqual(json.loads((folder/'resumen.json').read_text())['estado'],'VALIDADA')
    def test_cancellation_stops_next_page_and_next_tribunal(self):
        cat=catalog('litigantes');bridge=FakeNewBridge(cat);cancel=threading.Event();events=[]
        def emit(n,v):
            events.append((n,v))
            if n=='progress':cancel.set()
        with self.assertRaises(m.PocError):BatchRunner(bridge,cat,emit,cancel).run(Batch(('49','113'),(),(),screen='litigantes'),self.temp)
        self.assertEqual(bridge.events.count('prepare'),1);self.assertEqual(bridge.events.count('download'),1)
        self.assertEqual(bridge.events[-1],'unlock')

class NewInterfaceTests(unittest.TestCase):
    def test_checkboxes_save_subset_and_empty_disables_download(self):
        import tkinter as tk
        from descargador import Application,TribunalPicker
        root=tk.Tk();root.withdraw();app=Application(root,start_worker=False)
        try:
            app.process_event('connected','Ficticio');app.process_event('catalog',catalog('seguimiento'))
            popup=TribunalPicker(root,app.catalog['tribunales'],app.selected_tribunals,app.select_tribunals)
            popup.choices['777'].set(False);popup.apply()
            self.assertEqual(set(app.batch().tribunals),{'49','113'})
            popup=TribunalPicker(root,app.catalog['tribunales'],app.selected_tribunals,app.select_tribunals)
            popup.set_all(False);popup.destroy();self.assertEqual(set(app.batch().tribunals),{'49','113'})
            app.select_tribunals(());self.assertEqual(str(app.download_button['state']),'disabled')
            with self.assertRaises(m.PocError):app.batch().plan(app.catalog)
        finally:
            popup.choices.clear();dispose_interface(app,root)
    def test_screen_controls_and_selection_survive_catalog_reload(self):
        import tkinter as tk
        from descargador import Application
        root=tk.Tk();root.withdraw();app=Application(root,start_worker=False)
        try:
            app.process_event('connected','Ficticio');app.process_event('catalog',catalog('seguimiento'));app.select_tribunals(('49','113'))
            app.process_event('catalog',catalog('litigantes'));app.order_state.set('Terminada');app.use_dates.set(True);app.refresh()
            self.assertEqual(app.batch().state,'2');self.assertEqual(str(app.mod_combo['state']),'disabled')
            self.assertEqual(str(app.state_combo['state']),'readonly');self.assertIsNotNone(app.batch().dates)
            app.process_event('catalog',catalog('calendario_informes'));app.refresh()
            self.assertEqual(set(app.batch().tribunals),{'49','113'})
            expected=str(m.now().month) if str(m.now().month) in app.catalog['meses'] else '9'
            self.assertEqual(app.batch().month,expected)
            self.assertIsNone(app.batch().dates);self.assertEqual(str(app.month_combo['state']),'readonly')
            self.assertEqual(str(app.date_check['state']),'disabled')
            app.process_event('catalog',catalog('seguimiento'));self.assertEqual(set(app.batch().tribunals),{'49','113'})
        finally:dispose_interface(app,root)

if __name__=='__main__':unittest.main()
