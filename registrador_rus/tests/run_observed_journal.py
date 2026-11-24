"""Recorrido cerrado DOM ficticio -> diario -> parser -> recibo.

El navegador no guarda: las respuestas HTML se construyen localmente como
fixtures. No existe transporte, URL de navegación, ni adaptador institucional.
"""
from pathlib import Path
from tempfile import TemporaryDirectory
from hashlib import sha256
from dataclasses import asdict, replace
import os
import sys

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT))
from playwright.sync_api import sync_playwright
from run_observed_prepare import load_fixture, CATALOGS, TEXT
from test_reader_integration import fixture as response_fixture, params
from nurus.bitacora_html import read_snapshot
from registrador_rus.app.contracts import RegistroFlowV1
from registrador_rus.app.journal import SimulationJournal, JournalError
from registrador_rus.app.simulation import FakeClock


def run(page,origin,destination,scenario):
    load_fixture(page,origin,destination)
    op=RegistroFlowV1.from_mapping(page.evaluate('op'))
    def verify(current=op):
        report=page.evaluate('value=>session.verify(value)',asdict(current))
        assert report==dict(schema='ObservedPreparationSimulationV1',operation_id=op.operation_id,
            origin=op.origin,observation_state=op.observation_state,destination=op.destination,
            checked=op.tribunal_checked,cc=op.cc,saved=False,write_enabled=False),report
        return report
    def assert_no_submit():
        assert page.evaluate('events.submit===0&&events.click===0')
        assert page.evaluate("P.querySelector('[name=GLS_Observacion]').value==='Edición ficticia intacta'")
    def require_block(action,contains):
        try:action()
        except Exception as exc:assert contains in str(exc),str(exc)
        else:raise AssertionError('Se aceptó una operación que debía bloquearse')
    page.evaluate('()=>{window.session=RegistradorObservedPreparation.prepareSession(L,P,op,current)}')
    verify()
    clock=FakeClock()
    with TemporaryDirectory() as tmp:
        path=Path(tmp)/'observed-simulation.sqlite'
        j=SimulationJournal(path,clock=clock)
        try:
            assert j.freeze(op)=='PREPARADA'
            verify();j.mark_ready(op)
            if scenario=='NO_CONFIRM':
                require_block(lambda:j.authorize_simulation(op,human_actor='actor-ficticio',confirmed=False),'CONFIRMACION_HUMANA_REQUERIDA')
                assert j.attempt_count()==0 and j.state(op.operation_id)=='LISTA'
                assert_no_submit();return
            verify();j.authorize_simulation(op,human_actor='actor-ficticio',confirmed=True)
            before=read_snapshot(response_fixture(op),params(op),captured_at=clock().isoformat())
            assert j.bind_baseline(op,before,author='actor-ficticio',stage='Etapa ficticia',
                stage_code='1',server_day=clock().date())=='SIN_PREVIA'
            if scenario=='CHANGED':
                page.evaluate("P.querySelector('[name=chkBitacora]').checked=!P.querySelector('[name=chkBitacora]').checked")
                require_block(verify,'VALORES_NO_COINCIDEN')
                assert j.attempt_count()==0 and j.state(op.operation_id)=='LISTA'
                assert_no_submit();return
            if scenario=='REPLACED':
                page.evaluate("P.querySelector('#GLS_ObsCentro_Nueva').replaceWith(P.querySelector('#GLS_ObsCentro_Nueva').cloneNode(true))")
                require_block(verify,'FORMULARIO_CAMBIADO')
                assert j.attempt_count()==0
                assert_no_submit();return
            if scenario=='STALE':
                require_block(lambda:verify(replace(op,approval_version=2)),'APROBACION_CAMBIADA')
                assert j.attempt_count()==0
                assert_no_submit();return
            if scenario=='CONTEXT':
                page.evaluate("P.querySelector('[name=ID_Ingreso]').value='999999'")
                require_block(verify,'CONTEXTO_NO_COINCIDE')
                assert j.attempt_count()==0
                assert_no_submit();return
            # Datos efectivamente leídos de la preparación ficticia, no un submit.
            verify()
            observed=page.evaluate("()=>({text:P.querySelector('#GLS_ObsCentro_Nueva').value,state:P.querySelector('select').value,checked:P.querySelector('[name=chkBitacora]').checked})")
            assert observed==dict(text=op.approved_text,state=op.observation_state,checked=op.tribunal_checked)
            synthetic=replace(op,approved_text=observed['text'],approved_text_sha256=sha256(observed['text'].encode()).hexdigest(),
                observation_state=observed['state'],destination='AL_TRIBUNAL' if observed['checked'] else 'ADMINISTRATIVA')
            assert synthetic==op
            assert j.reserve_simulated_attempt(op)=='ENVIANDO'
            clock.tick()
            if scenario=='REOPEN':
                j.mark_uncertain(op.operation_id)
                j.close()
                j=SimulationJournal(path,clock=clock)
                assert j.recover_unresolved()==(op.operation_id,)
                require_block(lambda:j.reserve_simulated_attempt(op),'INTENTO_PREVIO_SOLO_LECTURA')
            after=read_snapshot(response_fixture(synthetic,entries=('900001',),partial=scenario=='PARTIAL'),params(op),captured_at=clock().isoformat())
            state=j.resolve_simulated(op,after)
            if scenario=='PARTIAL':
                assert state=='INCIERTA'
                assert j.recover_unresolved()==(op.operation_id,)
                clock.tick()
                after=read_snapshot(response_fixture(synthetic,entries=('900001',)),params(op),captured_at=clock().isoformat())
                state=j.resolve_simulated(op,after)
            assert state=='COMPROBADA' and j.attempt_count()==1 and j.recover_unresolved()==()
            assert j.db.execute('SELECT COUNT(*) FROM readback_receipts').fetchone()[0]==1
            assert j.db.execute('SELECT simulated FROM write_attempts').fetchone()[0]==1
            require_block(lambda:j.reserve_simulated_attempt(op),'INTENTO_PREVIO_SOLO_LECTURA')
            assert_no_submit()
        finally:j.close()


if __name__=='__main__':
    with sync_playwright() as p:
        browser=p.chromium.launch(headless=True,**({'executable_path':os.environ['REGISTRADOR_CHROMIUM']} if os.getenv('REGISTRADOR_CHROMIUM') else {}))
        context=browser.new_context();context.route('**/*',lambda r:r.abort())
        page=context.new_page()
        cases=[('ESPERA','ADMINISTRATIVA','OK'),('CUMPLIMIENTO','AL_TRIBUNAL','OK'),
               ('INFORMES_POR_VENCER','AL_TRIBUNAL','PARTIAL'),('INFORMES_VENCIDOS','ADMINISTRATIVA','REOPEN')]
        cases.extend(('ESPERA','ADMINISTRATIVA',s) for s in ('NO_CONFIRM','CHANGED','REPLACED','STALE','CONTEXT'))
        for case in cases:run(page,*case)
        browser.close()
        print(f'PASS {len(cases)} DOM-journal-reader integration checks')
