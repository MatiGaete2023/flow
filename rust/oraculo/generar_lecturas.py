"""Oráculo local de ventanas HTML ficticias: bitácora, historia y calendario por ingreso. Sin red."""
import json
import sys
from pathlib import Path

RAIZ=Path(__file__).resolve().parents[2]
sys.path[:0]=[str(RAIZ/'src'),str(RAIZ/'tests')]
from bitacora_fixtures import PARAMS,popup
from nurus.bitacora_html import read_snapshot
from nurus.historia_html import read_historia
from nurus.calendario_html import read_calendar


def main():
    casos=[]
    for texto,respuesta in [('Texto completo que la tabla oculta.','---'),('Observación ficticia con tildes, ñ y = literal.','Respuesta del tribunal')]:
        html=popup(texto,respuesta).replace('P-12-2026','P-12-2099').replace('Persona de prueba','Persona Ficticia')
        if respuesta!='---':html=html.replace('<td>---</td>','<td>13/08/2026 11:00</td>',1)
        esperado=read_snapshot(html,PARAMS,source='ficticio.html',captured_at='2026-11-12T12:00:00Z')
        casos.append({'tipo':'bitacora','html':html,'params':PARAMS,'esperado':esperado})
    html=(RAIZ/'descargador/fixtures/historia_ficticia/ventana_historia.html').read_text(encoding='utf-8')
    vacia=html[:html.index('<tbody>')+7]+html[html.index('</tbody>'):]
    for variante in [html,vacia,html.replace('>09/10/2099<','>ilegible<'),html.replace('</body>','<ul class="pagination"></ul></body>')]:
        casos.append({'tipo':'historia','html':variante,'esperado':read_historia(variante,source='ficticio.html',captured_at='2026-11-12T12:00:00Z')})
    args=['100','300','49','400','X-1-2099','01/03/2026','PRM Ficticio','Persona Ficticia','6 Meses','3 Meses','201','1']
    for filas in ['', '<tr><td>01/06/2026 00:00</td><td>28/05/2026 13:16:05</td><td>ENVIADO</td></tr><tr><td>01/09/2026</td><td>---</td><td>PENDIENTE</td></tr>']:
        html='<html><body><form name="InformesPpalForm"><input name="COD_Tribunal" value="49"><input name="CRR_IdCausa" value="100"><input name="ID_Ingreso" value="201"><table><tr><td>Plazo Remisión: 3 Meses</td><td>Plazo Intervención: 6 Meses</td></tr></table><table><tr><th>Fecha Proyección</th><th>Fecha Informe</th><th>Estado</th></tr>'+filas+'</table></form></body></html>'
        casos.append({'tipo':'calendario','html':html,'args':args,'esperado':read_calendar(html,args,source='ficticio.html',captured_at='2026-11-12T12:00:00Z')})
    salida=RAIZ/'rust/crates/flow-captura/tests/datos/lecturas_ficticias.json'
    salida.write_text(json.dumps(casos,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(f'{len(casos)} ventanas ficticias cotejadas con Python.')


if __name__=='__main__':main()
