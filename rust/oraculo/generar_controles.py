"""Oráculo de controles de paquete, calendario por ingreso y Búsqueda Lit; solo datos ficticios."""
import json
import random
import sys
from datetime import date,timedelta
from pathlib import Path

RAIZ=Path(__file__).resolve().parents[2]
sys.path[:0]=[str(RAIZ/'src')]
from nurus.personal.ordenes_busqueda import avisos_orden
from nurus.personal.calendario_ingreso import avisos_cotejo,primera_pendiente,proxima_pendiente
from nurus.capture.fixtures import fixture
from nurus.capture.contract import TABLES
from nurus.capture import write_capture
from nurus.personal.capture_input import import_capture
from nurus.personal.config import defaults


def main():
    destino=RAIZ/'rust/crates/flow-trabajo/tests/datos'
    corte=date(2026,10,8)
    rnd=random.Random(156)
    ordenes=[]
    for dias in [59,60,61,100]:
        for estado in ['Activa','Inactiva']:
            for envi in ['', 'Realizada','Pendiente']:
                values={'ESTADO O. B.':estado,'RESULTADO':'---','FECHA TRAMITE':(corte-timedelta(days=dias)).isoformat(),
                        'FEC. NACIMIENTO':'2000-10-08','ESTADO ENVIO':envi,'TIPO LITIGANTE':'Niña','CURADOR':'',
                        'PROX. AUDS.':'07/10/2026; 09/10/2026; 08/10/2026'}
                ordenes.append(dict(valores=values,corte=corte.isoformat(),avisos=avisos_orden(values,corte)))
    for _ in range(70):
        values={'ESTADO O B':'ACTIVA','RESULTADO':'','FECHA TRAMITE':'2026-01-01','FEC NACIMIENTO':rnd.choice(['2008-10-08','2008-10-09','ilegible']),
                'TIPO LITIGANTE':rnd.choice(['Niño','Adolescente','Adulto']),'CURADOR':rnd.choice(['---','Curador ficticio']),
                'PROX AUDS':rnd.choice(['09/10/2026','ilegible','01/01/2027'])}
        values={k:v for k,v in values.items() if rnd.random()>.25}
        ordenes.append(dict(valores=values,corte=corte.isoformat(),avisos=avisos_orden(values,corte)))
    calendarios=[]
    for _ in range(100):
        items=sorted({(corte+timedelta(days=rnd.choice([-1,0,1,61,90])),rnd.choice(['PENDIENTE','ENVIADO'])) for _ in range(rnd.randrange(5))})
        v=rnd.choice([None,corte,corte+timedelta(days=1)])
        h=rnd.choice([None,corte+timedelta(days=1),corte+timedelta(days=61)])
        calendarios.append(dict(items=[dict(fecha=f.isoformat(),estado=s) for f,s in items],corte=corte.isoformat(),vencimiento=v.isoformat() if v else None,hoja2=h.isoformat() if h else None,
            primera=primera_pendiente(items).isoformat() if primera_pendiente(items) else None,
            proxima=proxima_pendiente(items,corte).isoformat() if proxima_pendiente(items,corte) else None,
            avisos=avisos_cotejo(items,corte,vencimiento_informes=v,fecha_hoja2=h)))
    (destino/'controles.json').write_text(json.dumps(dict(ordenes=ordenes,calendarios=calendarios),ensure_ascii=False,indent=1)+'\n',encoding='utf-8')
    metadata,tablas=fixture(5)
    # Una causa con dos ingresos y vistas simultáneas de estado; una orden de búsqueda sin identidad remota.
    for i,r in enumerate(tablas['registros']):
        raw=json.loads(r['normalized_json']);raw['RIT']='X-1-2099' if i<4 else 'X-2-2099'
        raw.update({'DIAS DE CUMPLIMIENTO':100,'DIAS PARA EGRESAR':100,'FECHA VENCIMIENTO':'2026-10-09'})
        r.update(cause_id='causa-ficticia-0' if i<4 else '',ingreso_id='ingreso-ficticio-0' if i<3 else 'ingreso-ficticio-1',record_type=['CUMPLIMIENTO','EGRESADOS','INFORMES','INFORMES','OTRO'][i],rit_original=raw['RIT'])
        if i==4:
            raw.update({'ESTADO O B':'Activa','RESULTADO':'','FECHA TRAMITE':'2026-07-01','ESTADO ENVIO':'Pendiente','FEC NACIMIENTO':'2000-01-01','TIPO LITIGANTE':'Niña','CURADOR':'','PROX AUDS':'09/10/2026'})
            r.update(query_id='q-ob',identity_status='INCIERTA',ingreso_id='')
        r['normalized_json']=r['raw_json']=json.dumps(raw,ensure_ascii=False)
    tablas['consultas'].append({**tablas['consultas'][0],'query_id':'q-ob','screen':'litigantes','tab':''})
    tablas['bitacoras']=[];metadata['table_states']['bitacoras']='VACIA_COMPROBADA'
    base=tablas['registros'][0]
    calendario={k:base.get(k,'') for k in TABLES['calendarios']}
    calendario.update(calendar_id='cal-1',event_type='INFORME_PROYECTADO',event_at='2026-10-10',association='VERIFICADA',status='PENDIENTE',normalized_json='{}')
    hoja2={**calendario,'calendar_id':'cal-2','event_type':'INFORME','event_at':'2026-10-12','normalized_json':json.dumps({'FECHA VENCIMIENTO':'2026-10-12'})}
    tablas['calendarios']=[calendario,hoja2];metadata['table_states']['calendarios']='COMPLETA'
    paquete=destino/'paquete_controles.zip';paquete.unlink(missing_ok=True);write_capture(paquete,metadata,tablas)
    work=import_capture(paquete,defaults(),as_of=corte)
    filas=[dict(id=r.id,observacion=r.observation,reglas=r.rules,acciones=r.actions,avisos=r.warnings,proximo=work.cross_dates.get(r.id).isoformat() if work.cross_dates.get(r.id) else None) for r in work.rows]
    (destino/'paquete_controles.json').write_text(json.dumps(dict(corte=corte.isoformat(),filas=filas),ensure_ascii=False,indent=1)+'\n',encoding='utf-8')
    print(f'{len(ordenes)} órdenes, {len(calendarios)} calendarios y un paquete ficticio cotejados con Python.')


if __name__=='__main__':main()
