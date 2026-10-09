"""WP-41: la barra «Siguiente paso» cuenta lo mismo que cada página y abre la página del paso."""
from test_integral_ui import run_gui


def test_conteos_coinciden_con_cada_pagina(tmp_path):
    run_gui(tmp_path, """
import re
from nurus.personal.outputs import Draft, prepare_drafts
from nurus.personal.resolutions import prepare_projects, automatic_project_selections
from nurus.personal.config import BASE
from reglas_rt_util import analizar, persona
encabezados=['RIT','TRIBUNAL','NOMBRE','RUT','NOMBRE CENTRO','T ESPERA']
work=analizar(root,'ESPERA',encabezados,[persona(1)+[45],persona(2)+['sin dato'],persona(3)+['x'],persona(4,tribunal='OTRO')+[50]])
work.rows[2].excluded=True
app.work=work;app._show_work();app.update()
app.drafts=prepare_drafts(work,'programa_espera',directory=root/'salida')+[Draft('Correo pendiente','Texto',to='')];app.draft_index=None
projects,errors=prepare_projects(work,automatic_project_selections(work),BASE/'plantillas_word');assert not errors
app.projects=projects
app.tabs.select(app.pages['Resultados']);app.update()
texto={k:v.get() for k,v in app.next_step_text.items()}
con_aviso=sum('warning' in app.records.item(i,'tags') for i in app.records.get_children())
assert con_aviso==2 and texto['revisar']=='1 Revisar (2)', texto
assert texto['exportar']=='2 Exportar copia'
assert texto['correos']==f'3 Correos ({len(app.drafts)})' and len(app.drafts)>=2
assert texto['resoluciones']==f'4 Resoluciones ({len(app.projects)})' and len(app.projects)>=1
por_corregir=int(re.search(r'(\\d+) por corregir',app.results_summary.get()).group(1))
assert por_corregir>=1 and texto['resultados']==f'5 Resultados ({por_corregir})'
for clave,pagina in [('revisar','Trabajo'),('correos','Correos'),('resoluciones','Resoluciones'),('resultados','Resultados')]:
    app.next_step_buttons[clave].invoke();app.update()
    assert app.tabs.select()==str(app.pages[pagina]),clave
""")


def test_sin_trabajo_todo_en_cero(tmp_path):
    run_gui(tmp_path, """
app.update()
assert {k:v.get() for k,v in app.next_step_text.items()}=={
    'revisar':'1 Revisar (0)','exportar':'2 Exportar copia','correos':'3 Correos (0)',
    'resoluciones':'4 Resoluciones (0)','resultados':'5 Resultados (0)'}
""")
