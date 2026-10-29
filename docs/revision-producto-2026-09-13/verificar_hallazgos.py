"""Verificaciones de auditoría sobre copias; sin Excel/Outlook ni datos personales."""
from pathlib import Path
import sys, json, importlib.util, argparse
from datetime import date
from types import SimpleNamespace
import time_machine

ROOT = Path(__file__).resolve().parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--asistente', type=Path, required=True)
parser.add_argument('--nurus', type=Path, required=True)
parser.add_argument('--creador', type=Path, required=True)
parser.add_argument('--salida', type=Path, default=ROOT / 'evidencia_auditoria.json')
args = parser.parse_args()
sys.path[:0] = [str(args.asistente.resolve()), str(args.nurus.resolve() / 'src')]
from motor.reglas_espera import generar_observacion_espera
from motor.reglas_cumplimiento import generar_observacion_cumplimiento
from resoluciones.generador_resoluciones import detectar_tipo
from nurus.rus.rules import evaluate_waiting, evaluate_compliance
from nurus.rus.models import SourceRecord, SourceReference
from nurus.rus.catalog import load_catalog

spec = importlib.util.spec_from_file_location('creador_auditado', args.creador.resolve())
creador = importlib.util.module_from_spec(spec)
spec.loader.exec_module(creador)
results = {}
catalog = load_catalog()
def rec(row):
    return SourceRecord(row, SourceReference('sintetico.xlsx', 'auditoria', 'Hoja1', 2))

with time_machine.travel('2026-07-15 10:00:00', tick=False):
    cols = {k:k for k in ('nombre', 'programa', 'tribunal', 'espera')}
    row = dict(nombre='Persona Ejemplo', programa='AFT EJEMPLO', tribunal='LAJA', espera=30)
    obs = generar_observacion_espera(row, 'LAJA', cols)
    edited = obs.replace('proyecto de resolución pidiendo cuenta al programa respecto del ingreso efectivo', 'proyecto para consultar al programa por el ingreso efectivo')
    assert detectar_tipo(obs) == 'PC_IE' and detectar_tipo(edited) is None
    results['V03_dependencia_redaccion'] = {'original':detectar_tipo(obs), 'misma_intencion_redactada_distinto':detectar_tipo(edited)}
    rows=[]
    for days in (0,29,30,59,60):
        row.update(programa='DCE EJEMPLO',espera=days)
        a=generar_observacion_espera(row,'LAJA',cols)
        n,ids,_=evaluate_waiting(rec(row),cols,catalog,date(2026,7,15))
        rows.append({'dias':days,'asistente_menciona_correo':'correo' in a,'nurus_E05':'E-05' in ids})
    assert [r['nurus_E05'] for r in rows]==[False,False,True,True,True]
    results['V04_DCE']=rows
    row.update(programa='AFT EJEMPLO',espera='dato inválido')
    a=generar_observacion_espera(row,'LAJA',cols)
    n,ids,issues=evaluate_waiting(rec(row),cols,catalog,date(2026,7,15))
    assert a and not n and issues
    results['V05_fallback_invalido']={'asistente_genera_base':bool(a),'nurus_genera_base':bool(n),'nurus_incidencias':[i.code for i in issues]}
    row=dict(nombre='Persona Ejemplo',programa='AFT EJEMPLO',tribunal='LAJA',dias_cumpl=50,dias_egresar=-1,egreso_proy=date(2026,8,1))
    cols={k:k for k in row}
    a=generar_observacion_cumplimiento(row,'LAJA',cols)
    n,ids,issues=evaluate_compliance(rec(row),cols,catalog,date(2026,7,15))
    assert 'vencida' in a and 'C-04' not in ids and any(i.code=='C-04' for i in issues)
    results['V06_fecha_contradictoria']={'asistente_afirma_vencida':True,'nurus_advierte':True}

class Var:
    def __init__(self,value):self.value=value
    def get(self):return self.value
fake=SimpleNamespace(_destinatarios_para=lambda _: 'persona@example.org',cc_var=Var(''),cfg={'validar_manual':False},_validate_modalidades_for_template=lambda:None,attachments=[])
para,cc,error=creador.CSMPMailApp._validate_before_create(fake,'LAJA')
assert cc==[] and error is None
fake._destinatarios_para=lambda _: ''
_,_,error2=creador.CSMPMailApp._validate_before_create(fake,'LAJA')
assert error2=='No hay destinatarios en Para.'
results['V07_validacion_correo']={'CC_omitido_sin_error':True,'Para_vacio_bloqueado':True}
a=json.loads((args.asistente/'motor/textos_observaciones.json').read_text())
entries=[(s,k,v['texto']) for s,d in a.items() if not s.startswith('_') for k,v in d.items()]
assert len(entries)==25 and all(catalog[s][k]['texto']==v for s,k,v in entries)
results['V08_catalogos']={'textos_comparados':25,'diferencias':0}
args.salida.write_text(json.dumps(results,ensure_ascii=False,indent=2)+'\n', encoding='utf-8')
print(json.dumps(results,ensure_ascii=False,indent=2))
