"""Prueba del recorrido completo dentro del EXE, con datos ficticios."""
import base64,hashlib,json,threading
from pathlib import Path
from urllib.parse import urlencode
from lotes import Batch
from lectura_bitacoras import DiaryRunner
from csmp_shared import parser

def produce(folder):
    headers=parser().__globals__['HEADERS']
    params={'tipo_popUp':'12','CRR_IdCausa':'100','COD_Tribunal':'49','TIP_Consulta':'1','ID_Ingreso':'200','COD_Etapa':'3','FLG_MejorNinez':'0'}
    fields={**params,'tipo_popUp':'','RIT_Causa':'X-1-2026','GLS_Nombre':'Persona ficticia','GLS_Centro':'Centro ficticio'}
    hidden={'HOBS_Centro_':'Texto completo','HOBS_Tribunal_':'---','HCOD_Estado_':'6','HFUN_Tribunal_':'','HFLG_Envio_':'1'}
    popup='<html><form name="InformesPpalForm" action="/SITFAWEB/InformesDAction.do">'+''.join(f'<input name="{k}" value="{v}">' for k,v in fields.items())
    popup+='<table id="TablaInforme"><tr>'+''.join(f'<td>{v}</td>' for v in headers)+'</tr><tr>'+''.join(f'<td>{v}</td>' for v in ('Al Tribunal','Cumplimiento','08/10/2026 10:00','Usuario ficticio','Texto','---','---','---'))+'<td><img id="44"></td></tr>'
    popup+=''.join(f'<input type="hidden" name="{k}44" value="{v}">' for k,v in hidden.items())+'</table></form></html>'
    catalog={'tribunales':{'49':'Tribunal ficticio'},'modalidades':{'1':'Residencia'},'pestanas':['Cumplimiento'],'informes':{},'medidas':{}}
    class Bridge:
        connected=True
        def call(self,command,payload=None,timeout=75):
            if command=='capabilities':
                from contrato_integral import capabilities
                return capabilities()
            if command=='prepare':return {'pairs':[[k,v] for k,v in {'COD_Lengueta':'tdCumplimiento','COD_Tribunal_sel':'49','TIP_Consulta':'1','FLG_Consulta':'0','irAccion':'Buscar medida','NUM_PaginaCumplimiento':'','NUM_TotalCumplimiento':''}.items()]}
            if command=='search':
                values={**dict(payload['pairs']),'NUM_PaginaCumplimiento':'1','NUM_TotalCumplimiento':'1'}
                body='<html><meta charset="utf-8"><form name="InformesPpalForm" action="/SITFAWEB/InformesDAction.do" method="post">'+''.join(f'<input name="{k}" value="{v}">' for k,v in values.items())
                body+='''<table id="tablaCumplimiento"><tr><td>RIT</td><td>RUT</td><td>Nombre</td><td>Derivación</td><td>Obs.</td></tr><tr><td>X-1-2026</td><td>11111111-1</td><td>Persona ficticia</td><td>Centro ficticio</td><td><a onclick="ShowHistoria('49','X-1-2026','100')">Historia</a><a onclick="ShowObservaciones('100','49','200','12','3','0')">Obs</a></td></tr></table></form><script>function ShowExcel(){window.open('https://familia.pjud.cl/sitfa/reportes/ficticio_reslistcump.xls');}</script></html>'''
            elif command=='diary_open':body=popup
            else:return {'ok':True}
            data=body.encode();return {'status':200,'type':'text/html','body':base64.b64encode(data).decode(),
                'params':params,'digest':hashlib.sha256(data).hexdigest(),'startedAt':'2026-10-08T13:00:00Z'}
    result=DiaryRunner(Bridge(),catalog,lambda *a:None,threading.Event()).run(Batch(('49',),('1',),('Cumplimiento',)),folder)
    if result['estado']!='COMPLETA' or result['leidas']!=1:
        details=json.loads(Path(result['manifest']).read_text(encoding='utf-8'))
        raise ValueError('Falló el recorrido congelado ficticio: '+json.dumps(details['consultas'],ensure_ascii=False))
    return result

def check(folder):
    produce(folder)
    return True
