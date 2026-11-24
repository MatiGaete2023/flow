"""Medición reproducible exclusivamente ficticia; sin esperas ni red institucional."""
from collections import Counter
from pathlib import Path
from statistics import median
from tempfile import TemporaryDirectory
from time import perf_counter
import json
import tracemalloc

from registrador_rus.app.journal import SimulationJournal
from registrador_rus.app.scheduler import ScheduleConfig
from registrador_rus.app.simulation import FakeClock, FakeRus, SimulationController, demo_operations


def run(count=400):
    clock=FakeClock(); start=clock(); ops=demo_operations(count); fake=FakeRus(clock,ops)
    with TemporaryDirectory() as tmp:
        path=Path(tmp)/'demo.sqlite'; journal=SimulationJournal(path,clock=clock)
        phase={key:[] for key in ('prepare','authorize','execute_readback')}; waits=0; states=Counter()
        tracemalloc.start(); started=perf_counter()
        try:
            journal.configure_schedule(ScheduleConfig(True,100,mode='fixed',fixed_seconds=2))
            controller=SimulationController(journal,fake)
            for i,op in enumerate(ops):
                if i==count//2:
                    journal.control_schedule('PAUSE'); journal.close()
                    journal=SimulationJournal(path,clock=clock); controller=SimulationController(journal,fake)
                    assert journal.schedule_eligibility().reason=='PAUSED'
                    journal.control_schedule('RESUME')
                eligibility=journal.schedule_eligibility()
                if not eligibility.allowed:
                    assert eligibility.reason in ('ESPERAR_CUPO','ESPERAR_INTERVALO')
                    clock.tick((eligibility.eligible_at-clock()).total_seconds()); waits+=1
                for key,action in [('prepare',lambda:controller.prepare(op)),('authorize',lambda:controller.authorize(op,confirmed=True)),('execute_readback',lambda:controller.execute(op))]:
                    t=perf_counter(); result=action(); phase[key].append(perf_counter()-t)
                states[result]+=1
                assert result=='COMPROBADA'
            assert journal.attempt_count()==fake.calls==count
            assert not journal.recover_unresolved()
            assert journal.db.execute('SELECT COUNT(*) FROM readback_receipts').fetchone()[0]==count
            assert journal.db.execute('PRAGMA integrity_check').fetchone()[0]=='ok'
            elapsed=perf_counter()-started; _,peak=tracemalloc.get_traced_memory()
            return dict(simulation_only=True, count=count, states=dict(states), attempts=fake.calls, waits_virtual=waits,
                        local_seconds=round(elapsed,3), virtual_seconds=(clock()-start).total_seconds(),
                        peak_python_bytes=peak, median_phase_ms={k:round(median(v)*1000,3) for k,v in phase.items()})
        finally:
            tracemalloc.stop(); journal.close()


if __name__=='__main__':
    print(json.dumps(run(),ensure_ascii=False,indent=2))
