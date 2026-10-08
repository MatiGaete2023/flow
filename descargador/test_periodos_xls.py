from datetime import date,datetime
from dataclasses import replace
from pathlib import Path
import json,tempfile
import unittest

import motor as m
from perfiles import Selection
from preferencias import Preferences
from lotes import Batch,Dates
from test_flujos import catalog,extended_data,workbook_bytes
from perfiles import response_profile


class PeriodTests(unittest.TestCase):
    def test_monthly_download_saves_dash_dates_and_stops_on_other_period(self):
        selection=Selection('49','','',screen='calendario_informes',month='10',year='2026')
        pairs,html=extended_data(selection)
        initial_profile,page=response_profile(html,pairs,selection,catalog(selection.screen))
        class Transport:
            def __init__(self,data):self.data=data
            def download(self,*args):return self.data
            def search(self,*args):raise AssertionError('El calendario solo tiene una página.')
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            for month in ('10','11'):
                profile=replace(initial_profile,month=month)
                query=[(k,month if k=='COD_Mes_sel' else v) for k,v in pairs]
                data=workbook_bytes(['RIT','NOMBRE MENOR','FECHA VENCIMIENTO'],
                                    [['X-1-2026','Persona ficticia 1',f'01-{month}-2026']])
                folder=root/month
                results=m.run_sequence(Transport(data),query,page,folder,profile=profile)
                self.assertEqual(len(results),1)
                self.assertEqual((folder/results[0]['archivo']).read_bytes(),data)
                self.assertEqual(json.loads((folder/'verificacion.json').read_text())['estado'],'VALIDADA')
                bad=root/(month+'-otro-periodo')
                with self.assertRaises(m.PocError):
                    m.run_sequence(Transport(data),[(k,'9' if k=='COD_Mes_sel' else v) for k,v in query],
                                   page,bad,profile=replace(profile,month='9'))
                self.assertFalse(list(bad.glob('*.xls')))
                self.assertEqual(json.loads((bad/'verificacion.json').read_text())['estado'],'INCOMPLETA')

    def test_same_person_and_rit_from_previous_month_is_rejected(self):
        for screen,title in [('calendario_informes','F.VENC'),('calendario_medidas','FEC.EGRESO PROYECTADO'),
                             ('calendario_medidas','FECHA VENCIMIENTO')]:
            selection=Selection('49','','',screen=screen,month='9',year='2026')
            pairs,html=extended_data(selection)
            profile,page=response_profile(html,pairs,selection,catalog(screen))
            def data(value):return workbook_bytes(['RIT','NOMBRE MENOR',title],[['X-1-2026','Persona ficticia 1',value]])
            for value in ('10/10/2026','10-10-2026','2026-10-10','',
                          '31-09-2026','texto',date(2025,9,10)):
                with self.subTest(screen=screen,value=value):
                    with self.assertRaises(m.PocError):m.verified_download(data(value),page,set(),[],profile)
            for value in ('10/09/2026','10-09-2026','2026-09-10',
                          ' 10-09-2026 ',date(2026,9,10),datetime(2026,9,10,12,30)):
                self.assertEqual(sum(m.verified_download(data(value),page,set(),[],profile)[0].records.values()),1)

    def test_calendar_error_distinguishes_missing_date_from_other_month(self):
        selection=Selection('49','','',screen='calendario_informes',month='9',year='2026')
        pairs,html=extended_data(selection)
        profile,page=response_profile(html,pairs,selection,catalog('calendario_informes'))
        for value,message in [('',r'fila 2.*vacía o no se reconoce'),
                              ('10-10-2026',r'10/10/2026.*fila 2.*mes 9/2026')]:
            data=workbook_bytes(['RIT','NOMBRE MENOR','FECHA VENCIMIENTO'],
                                [['X-1-2026','Persona ficticia 1',value]])
            with self.subTest(value=value),self.assertRaisesRegex(m.PocError,message):
                m.verified_download(data,page,set(),[],profile)

    def test_carga_favorite_recalculates_recent_thirty_days(self):
        prefs=Preferences(memory=True)
        batch=Batch(('49',),(),(),screen='carga',dates=Dates('01/09/2026','30/09/2026'))
        prefs.favorite('Carga habitual',batch)
        current={**catalog('carga'),'pantalla':'carga'}
        restored,missing=prefs.restore('Carga habitual',current,date(2026,10,5))
        self.assertEqual(restored.dates,Dates('06/09/2026','05/10/2026'))
        self.assertEqual(missing,[])
