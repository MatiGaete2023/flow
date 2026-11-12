"""Listados ficticios y vínculos que extrae Python. Sin red ni lectura de archivos del usuario."""
import json
import sys
from pathlib import Path
from lxml import html,etree

RAIZ=Path(__file__).resolve().parents[2]
sys.path[:0]=[str(RAIZ/'descargador'),str(RAIZ/'src')]
from vinculos import extract,extract_historia


def main():
    fila='''<tr><td>X-1-2099</td><td>11111111-0</td><td>Persona Ficticia</td><td>PRM Centro Ficticio</td><td>
    <a onclick="ShowHistoria('901','X-1-2099','100')">Causa</a>
    <a onclick="ShowObservaciones('100','901','201','12','3','0')">Obs</a>
    <a onclick="ShowInformesProgramados('100','300','901','400','X-1-2099','01/03/2026','PRM Centro Ficticio','Persona Ficticia','6 Meses','3 Meses','201','1')">Inf</a>
    </td></tr>'''
    cabecera='<tr><th>RIT</th><th>RUT</th><th>Nombre</th><th>Derivación</th><th>Acciones</th></tr>'
    variantes=[('completa',fila),('sin_historia',fila.replace('ShowHistoria','OtraAccion')),('sin_bitacora',fila.replace('ShowObservaciones','OtraAccion')),
               ('corte_ajena',fila.replace("'901'","'902'")),('duplicada',fila+fila.replace("'201'","'202'")),
               ('obs_repetida',fila.replace('</td></tr>', '<a onclick="ShowObservaciones(100,901,201,12,3,0)">otra</a></td></tr>')),
               ('rut_con_script',fila.replace('11111111-0</td>', '11111111-0<script>document.write("texto ajeno")</script></td>')),
               ('causa_ambigua',fila+fila.replace("'100'","'101'")),
               ('solo_texto',fila.replace("ShowHistoria('901','X-1-2099','100')",'/* ShowHistoria(901,\'X-1-2099\',100) */'))]
    casos=[]
    for nombre,contenido in variantes:
        tabla=html.fromstring('<table>'+cabecera+contenido+'</table>')
        esperado=extract(tabla,'901');historias=extract_historia(tabla,'901')
        tabla.set('id','tablaEspera')
        texto='<html><body><form name="InformesPpalForm">'+etree.tostring(tabla,encoding='unicode')+'</form></body></html>'
        casos.append({'nombre':nombre,'html':texto,'bitacoras':esperado,'historias':historias})
    salida=RAIZ/'rust/crates/flow-captura/tests/datos/vinculos_ficticios.json'
    salida.write_text(json.dumps(casos,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(f'{len(casos)} listados ficticios cotejados con Python.')


if __name__=='__main__':main()
