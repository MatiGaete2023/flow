"""WP-41: barra «Siguiente paso» con los mismos conteos de cada página."""
from nurus.personal.next_step import counts, text
from test_integral_ui import run_gui


def test_texto_sin_trabajo():
    assert counts(None, [], []) == {'revisar': 0, 'exportar': False, 'correos': 0, 'resoluciones': 0, 'resultados': 0}
    assert text(counts(None, [], [])) == ('1 Revisar (0) → 2 Exportar copia → 3 Correos (0) → '
                                          '4 Resoluciones (0) → 5 Resultados (0)')


def test_conteos_coinciden_con_cada_pagina_y_se_actualizan(tmp_path):
    run_gui(tmp_path, """
import re,time
from nurus.personal.outputs import Draft
from nurus.personal.resolutions import prepare_projects, automatic_project_selections
bar=app.next_step
assert bar.values['revisar']==0 and not bar.values['exportar']
app.work=Work(app.cfg.data).analyze(source(root),'ESPERA',as_of=date(2026,10,2));app._show_work();app.update()
assert bar.values['revisar']==int(app.incident_text.get().split()[0])==len(app.work.rows)
app.work.export(root/'copia.xlsx',backend='portable',reduced_fidelity=True)
projects,_=prepare_projects(app.work,automatic_project_selections(app.work,'PC_IE'),app.template_dir)
app.projects=projects
app._display_prepared_drafts([Draft('Uno','Cuerpo',to='a@example.test'),Draft('Dos','',to='')],'{count} preparados')
fin=time.monotonic()+5
while time.monotonic()<fin and not bar.values['exportar']:
    app.update();time.sleep(0.05)
assert bar.values['exportar']
app.refresh_pending(capture=False)
total,_,por_corregir=map(int,re.findall(r'(\\d+) ',app.results_summary.get())[:3])
from nurus.personal.product_checks import current_drafts
assert bar.values['correos']==len(current_drafts(app))==2
assert bar.values['resoluciones']==len(app.projects)
assert bar.values['resultados']==por_corregir>=1
assert total>=bar.values['correos']+bar.values['resoluciones']
assert bar.buttons['correos'].cget('text')=='3 Correos (2)'
assert bar.buttons['exportar'].cget('text')=='2 Exportar copia ✓'
for clave,pagina in [('correos','Correos'),('resoluciones','Resoluciones'),('resultados','Resultados'),('revisar','Trabajo')]:
    bar.buttons[clave].invoke();app.update()
    assert app.tabs.select()==str(app.pages[pagina])
assert bar._after is not None
""")
