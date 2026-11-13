"""Oráculo sintético del mes móvil y última observación; no modifica la herramienta Python."""
import json
import random
import sys
from datetime import date, datetime, timedelta
from pathlib import Path
RAIZ=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(RAIZ/'META4_Bitacoras/scripts'))
import meta4_bitacoras as m
r=random.Random(22)
casos=[]
for n in range(450):
    corte=date(2026,1,1)+timedelta(days=r.randrange(365))
    entradas=[]
    for i in range(r.randrange(12)):
        momento=datetime.combine(corte,datetime.min.time())+timedelta(days=r.randrange(-65,5),hours=r.randrange(24))
        entradas.append(dict(fecha=momento.isoformat(),etapa=r.choice(['Espera','Cumplimiento','Informes','LISTA DE ESPERA','EN CUMPLIMIENTO','otra']),texto='Texto ficticio'))
    if n%7==0 and entradas:
        entradas.append(dict(entradas[-1],etapa='Cumplimiento'))
    fecha,etapa,control=m.ultima(entradas,corte)
    casos.append(dict(corte=str(corte),desde=str(m.periodo(corte)[0]),entradas=entradas,fecha=str(fecha) if fecha else None,etapa=etapa or None,control=control))
p=RAIZ/'rust/crates/flow-meta4/tests/datos/calculo_python.json'
p.write_text(json.dumps(casos,ensure_ascii=False,indent=1)+'\n',encoding='utf-8')
print(f'{len(casos)} cálculos ficticios de Meta 4 generados con Python.')
