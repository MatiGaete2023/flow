"""Regression for the real server initializer; no JS or real requests run."""
import base64
from datetime import datetime
from pathlib import Path
import threading
from unittest.mock import patch

import pytest
import motor as m
from filtros_respuesta import selected_report
from lotes import Batch,BatchRunner
from perfiles import Selection,response_profile,selection_profile
from test_lotes import CATALOG,FakeBridge,document,dispose_interface
from test_flujos import catalog


INITIALIZER=(Path(__file__).parent/'fixtures/selector_vencimiento_sitfa.js').read_text(encoding='utf-8')


def dynamic_response(selection,number=1,total=3,server_value=None,initializer=INITIALIZER):
    source=document(selection,number,total).decode()
    actual=selection.report if server_value is None else server_value
    source=source.replace(f'<input name="TIP_Informe" value="{selection.report}">',
                          '<select name="TIP_Informe" id="TIP_Informe"><option value="1">Por vencer a 25 días</option></select>')
    source=source.replace('</html>', '<script>'+initializer.replace("var valorSeleccionado = '0'",f"var valorSeleccionado = '{actual}'")+'</script></html>')
    return source


@pytest.mark.parametrize('tab,value',[('Cumplimiento','0'),('Cumplimiento','4'),('Cumplimiento','5'),
                                      ('Informes','1'),('Informes','2'),('Informes','3')])
def test_response_uses_server_dynamic_filter(tab,value):
    selection=Selection('49',tab,'2',value)
    with patch.dict(CATALOG['medidas'],{'0':'Todas las medidas'}):
        initial=selection_profile(selection,CATALOG)
        pairs=[*initial.target.items(),('irAccion','Buscar medida')]
        profile,page=response_profile(dynamic_response(selection),pairs,selection,CATALOG)
    assert profile.report_type==value
    assert page.records[('X-1-2026','PERSONA FICTICIA 1')]==1


def test_wrong_filter_still_blocks_response():
    selection=Selection('49','Cumplimiento','2','4')
    profile=selection_profile(selection,CATALOG)
    pairs=[*profile.target.items(),('irAccion','Buscar medida')]
    for value in ('0','5'):
        with pytest.raises(m.PocError,match='seleccion'):
            response_profile(dynamic_response(selection,server_value=value),pairs,selection,CATALOG)


@pytest.mark.parametrize('changed',[
    INITIALIZER.replace("var valorSeleccionado = '0'",'var valorSeleccionado = getValue()'),
    INITIALIZER.replace('select.innerHTML', 'another.innerHTML'),
    INITIALIZER+'\n'+INITIALIZER,
    INITIALIZER.replace('select.appendChild(newOption);','select.appendChild(newOption); sideEffect();'),
])
def test_unknown_or_ambiguous_initializer_is_rejected(changed):
    with pytest.raises(m.PocError):
        selected_report(m.root_html('<script>'+changed+'</script>'),'tdCumplimiento')


def test_filter_45_days_downloads_all_pages(tmp_path):
    class DynamicBridge(FakeBridge):
        def __init__(self):super().__init__();self.requests=[]
        def call(self,command,payload=None,timeout=75):
            result=super().call(command,payload,timeout)
            if command in ('search','download'):self.requests.append((command,self.number))
            if command=='search':
                source=dynamic_response(self.selection,self.number,self.total)
                result['body']=base64.b64encode(source.encode()).decode()
            return result
    events=[];bridge=DynamicBridge()
    result=BatchRunner(bridge,CATALOG,lambda n,v:events.append((n,v)),threading.Event()).run(
        Batch(('49',),('2',),('Cumplimiento',),'4'),tmp_path)
    folder=Path(next(value for name,value in events if name=='folder'))
    assert result[0]['estado']=='VALIDADA'
    assert result[0]['seleccion']['report']=='4'
    assert len(list(folder.glob('*.xls')))==3
    assert [('search',1),('download',1),('search',2),('download',2),('search',3),('download',3)]==bridge.requests


def test_calendar_defaults_current_period_and_preserves_explicit_choice():
    import tkinter as tk
    from descargador import Application
    root=tk.Tk();root.withdraw();app=Application(root,start_worker=False)
    try:
        options={**catalog('calendario_medidas'),'anios':{'2027':'2027','2026':'2026'},
                 'seleccion':{'anio':'2027','mes':'9'}}
        with patch('motor.now',return_value=datetime(2026,10,6)):
            app.process_event('catalog',options)
            assert app.month.get()=='Octubre'
            assert app.year.get()=='2026'
            app.year.set('2027')
            app.process_event('catalog',options)
            assert app.year.get()=='2027'
    finally:dispose_interface(app,root)
