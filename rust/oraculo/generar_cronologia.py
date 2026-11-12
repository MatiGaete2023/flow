"""Paridad local de cronología, CT-05/07/11 y reiteración; solo eventos ficticios, sin red."""
import json
import random
import sys
from datetime import date, timedelta
from difflib import SequenceMatcher
from pathlib import Path
from types import SimpleNamespace

RAIZ = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(RAIZ / 'src'))
from nurus.capture.temporal import display_chronology
from nurus.personal.bitacora_avisos import avisos, aplicar_reiteracion
from nurus.personal.config import defaults
from nurus.capture.fixtures import fixture
from nurus.capture import write_capture
from nurus.personal.capture_input import import_capture
from nurus.personal.capture_findings import bitacora_alerts


def evento(id, cuando, **extra):
    return dict(event_id=str(id), event_at=cuando, time_precision='DIA', time_zone='DESCONOCIDA',
                origin='CENTRO', origin_verified='true', coverage='PARCIAL', text='Observación ficticia.',
                event_type='Observación', response='', **extra)


def main():
    config = defaults()
    base = 'Persona PRM: Se solicita actualizar antecedentes de la medida.'
    cases = []
    def agregar(events, *, corte='2026-10-31', solicitada=True, leida=True, propuesta=base, reglas=(), desactivadas=()):
        corte = date.fromisoformat(corte)
        conf = {**config, 'desactivadas': list(desactivadas)}
        display = display_chronology(events, corte, requested=solicitada, read=leida)
        row = SimpleNamespace(id='ficticio', observation=propuesta, rules=list(reglas), actions=['programa_espera'], warnings=[])
        alertas = avisos(display, propuesta, reglas, conf['textos'], corte)
        aplicar_reiteracion([row], {'ficticio': {'display': display}}, conf)
        cases.append(dict(eventos=events, corte=corte.isoformat(), solicitada=solicitada, leida=leida,
                          propuesta=propuesta, reglas=list(reglas), desactivadas=list(desactivadas), display=display,
                          avisos=alertas, reiteracion=dict(observacion=row.observation, reglas=row.rules, acciones=row.actions, avisos=row.warnings)))
    for solicitada in [False, True]:
        for leida in [False, True]:
            agregar([], solicitada=solicitada, leida=leida)
    agregar([evento('antigua','2026-06-30'),evento('futura','2026-11-01'),evento('sin-fecha','')])
    agregar([evento('origen','2026-10-01') | {'origin_verified':'false'}])
    agregar([evento('a','2026-10-01') | {'text':base,'event_type':'Al Tribunal'}])
    agregar([evento('a','2026-10-01') | {'text':base}],desactivadas=['COMUN.REITERA_OBSERVACION'])
    agregar([evento('a','2026-10-01') | {'text':base}],propuesta='X'*41+': '+base)
    agregar([evento('a','2026-10-01') | {'text':'Nombre PRM: '+config['textos']['ESPERA']['E05_SOLO_CORREO']['texto']}],reglas=['ESPERA.E05_SOLO_CORREO'])
    # Empates con hora desconocida; con segundos y zona cierta; diferencias de huso horario.
    for certain in [False,True]:
        entries=[evento('a','2026-10-01T12:00:00+00:00'),evento('b','2026-10-01T11:30:00-01:00'),evento('c','2026-10-01T13:00:00Z') | {'origin':'OTRO'}]
        if certain:
            entries=[e | {'time_precision':'SEGUNDO','time_zone':'UTC'} for e in entries]
        agregar(entries)
    for iguales in [91,92]:
        agregar([evento('limite','2026-10-01') | {'text':'a'*iguales+'b'*(100-iguales)}],propuesta='a'*100)
    rnd=random.Random(1291)
    for _ in range(150):
        corte=rnd.choice([date(2026,10,31),date(2026,3,31),date(2024,6,30)])
        entries=[]
        for j in range(rnd.randrange(8)):
            fecha=corte-timedelta(days=rnd.choice([-1,0,1,29,30,31,120,121,124,300]))
            entries.append(evento(j,fecha.isoformat()) | {'origin':rnd.choice(['CENTRO','DESCONOCIDO','OTRO']),
                'origin_verified':rnd.choice(['true','false']),'text':rnd.choice([base,base[:-3]+'???','Otra observación']),
                'coverage':rnd.choice(['PARCIAL','COMPLETA','FALLIDA']),'event_type':rnd.choice(['Al tribunal','Observación']),
                'response':rnd.choice(['','Respuesta ficticia'])})
        agregar(entries,corte=corte.isoformat())
    ratios=[]
    pairs=[('',''),('tide','diet'),('diet','tide'),('a'*210+'b','a'*210+'c'),('a'*100,'a'*92+'b'*8),('a'*100,'a'*91+'b'*9)]
    for _ in range(250):
        a=''.join(rnd.choices('abcñ. :)',k=rnd.randrange(80)))
        b=rnd.choice([a[::-1],a+'xx',''.join(rnd.choices('abcñ. :)',k=rnd.randrange(80)))])
        pairs.append((a,b))
    for a,b in pairs:ratios.append(dict(a=a,b=b,ratio=SequenceMatcher(None,a,b,autojunk=False).ratio()))
    salida=RAIZ/'rust/crates/flow-trabajo/tests/datos/cronologia.json'
    salida.write_text(json.dumps(dict(casos=cases,similitudes=ratios),ensure_ascii=False,indent=1)+'\n',encoding='utf-8')
    print(f'{len(cases)} cronologías y {len(ratios)} comparaciones ficticias cotejadas con Python.')
    metadata, tablas=fixture(1)
    for tabla in tablas.values():
        for row in tabla:
            for k,v in row.items():
                if isinstance(v,str):row[k]=v.replace('FICT-0-2026','X-1-2099')
    destino=salida.parent/'paquete_cronologia.zip'
    destino.unlink(missing_ok=True)
    write_capture(destino,metadata,tablas)
    inicial=import_capture(destino,config,as_of=date(2026,10,8))
    tablas['bitacoras'][0].update(text=inicial.rows[0].observation,event_at='2026-10-01',event_type='Al tribunal')
    destino.unlink()
    write_capture(destino,metadata,tablas)
    work=import_capture(destino,config,as_of=date(2026,10,8))
    rows=[dict(id=r.id,observacion=r.observation,reglas=r.rules,acciones=r.actions,avisos=r.warnings,
               cronologia=work.capture_context[r.id]['display'],alertas=bitacora_alerts(work,r)) for r in work.rows]
    (salida.parent/'paquete_cronologia.json').write_text(json.dumps(dict(corte=work.as_of,filas=rows),ensure_ascii=False,indent=1)+'\n',encoding='utf-8')


if __name__=='__main__':main()
