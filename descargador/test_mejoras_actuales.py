from datetime import date
from pathlib import Path
import threading,time
import unittest
from preferencias import Preferences
from lotes import Batch,Dates
from pausa import PauseGate
from motor import PocError
from flujo_csmp import plan
from test_resultados import fixture
from resultados import export_ics,load_lot
from test_lotes import CATALOG


class CurrentImprovements(unittest.TestCase):
    def test_relative_calendar_rolls_year_and_refuses_unavailable_period(self):
        prefs=Preferences(memory=True)
        batch=Batch(('113',),(),(),screen='calendario_informes',month='12',year='2026')
        prefs.favorite('Siguiente',batch,calendar_offset=1)
        catalog={**CATALOG,'tribunales':{'113':'Laja'},'meses':{'1':'Enero','2':'Febrero'},'anios':{'2027':'2027'},'pantalla':'calendario_informes'}
        restored,missing=prefs.restore('Siguiente',catalog,today=date(2026,12,15))
        self.assertEqual((restored.month,restored.year),('1','2027'));self.assertEqual(missing,[])
        catalog['anios']={}
        with self.assertRaises(PocError):prefs.restore('Siguiente',catalog,today=date(2026,12,15))
    def test_relative_dates_do_not_reuse_saved_absolute_dates(self):
        prefs=Preferences(memory=True);batch=Batch(('113',),('2',),('Espera',),dates=Dates('01/09/2026','30/09/2026'))
        prefs.favorite('Actual',batch)
        restored,_=prefs.restore('Actual',CATALOG,today=date(2026,10,6))
        self.assertEqual((restored.dates.start,restored.dates.end),('07/09/2026','06/10/2026'))
    def test_pause_blocks_next_unit_and_cancel_always_releases_worker(self):
        gate=PauseGate();gate.pause();entered=threading.Event();finished=threading.Event();result=[]
        def worker():entered.set();result.append(gate.is_set());finished.set()
        thread=threading.Thread(target=worker);thread.start();self.assertTrue(entered.wait(1));self.assertFalse(finished.wait(.05))
        gate.set();self.assertTrue(finished.wait(1));thread.join();self.assertEqual(result,[True])
        gate.clear();self.assertFalse(gate.is_set());gate.pause();gate.resume();self.assertFalse(gate.is_set())
    def test_joint_has_only_current_sources(self):
        phases=plan(('113',),('2',),'Cumplimiento',date(2026,12,31))
        self.assertEqual([b.screen for _,b in phases],['seguimiento','calendario_informes','calendario_informes'])
        self.assertEqual([(b.month,b.year) for _,b in phases[1:]],[('12','2026'),('1','2027')])
