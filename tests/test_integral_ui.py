"""Each GUI scenario owns its process and Tcl interpreter, like a CSMP launch."""
import os
from pathlib import Path
import subprocess
import sys

ROOT=Path(__file__).parents[1]


def run_gui(tmp_path, scenario):
    environment=dict(os.environ,PYTHONPATH=os.pathsep.join([str(ROOT/'src'),str(ROOT/'tests'),*sys.path]))
    script='''
from datetime import date
from pathlib import Path
import sys
from nurus.personal.config import Configuration
from nurus.personal.app import App
from nurus.personal.work import Work
from current_fixture import source
root=Path(sys.argv[1])
app=App(Configuration(root/'config'))
app.geometry('1024x650')
try:
'''+''.join('    '+line+'\n' for line in scenario.splitlines())+'''
finally:
    app.destroy()
'''
    result=subprocess.run([sys.executable,'-c',script,str(tmp_path)],env=environment,
                          capture_output=True,text=True,timeout=45)
    assert result.returncode==0,result.stdout+'\n'+result.stderr


def test_current_results_and_manual_editors_are_reachable(tmp_path):
    run_gui(tmp_path,"""
app.tabs.select(app.pages['Resultados']);app.update()
assert set(app.pages)=={'Trabajo','Correos','Resoluciones','Resultados','Configuración'}
assert app.results_download_button.winfo_exists()
from nurus.personal.manual import open_editor
open_editor(app,'mail');open_editor(app,'word');app.update()
""")


def test_hidden_selection_and_view_preferences_survive_refresh(tmp_path):
    run_gui(tmp_path,"""
app.work=Work(app.cfg.data).analyze(source(root),'ESPERA',as_of=date(2026,10,2));app._show_work()
id=app.work.rows[0].id
app.records.selection_set(id);app.work_search.set('no coincidencia');app._apply_work_filter();app.update()
from nurus.personal.selection import selected_ids
assert id in selected_ids(app,'records')
app._show_work();app.update();from nurus.personal.selection import selected_ids
assert id in selected_ids(app,'records')
app.records.configure(displaycolumns=('Nombre','RIT'));app.work_search.set('');app._apply_work_filter()
assert tuple(app.records['displaycolumns'])==('Nombre','RIT')
""")


def test_themes_result_identity_and_command_palette(tmp_path):
    run_gui(tmp_path,"""
from nurus.personal.outputs import Draft
from nurus.personal.commands import show_palette
for theme in ('Claro','Oscuro','Sistema'):
    app.theme_choice.set(theme);app.apply_appearance();app.update()
assert app.cfg.data['vista']['tema']=='Sistema'
app.mail_target.set('todos')
app._display_prepared_drafts([Draft('Igual','Primero',to='a@example.test'),Draft('Igual','Segundo',to='b@example.test')],'{count} preparados')
app.tabs.select(app.pages['Resultados']);app.update()
second='draft:'+app.drafts[1].product_id
app.results_tree.selection_set(second);app.open_result();app.update()
assert app.body.get('1.0','end-1c')=='Segundo'
show_palette(app);app.update();assert app.command_palette.winfo_exists()
""")


def test_large_table_yields_and_retains_hidden_selection(tmp_path):
    run_gui(tmp_path,"""
from copy import deepcopy
import time
app.work=Work(app.cfg.data).analyze(source(root),'ESPERA',as_of=date(2026,10,2))
base=app.work.rows[0]
app.work.rows=[]
for n in range(1100):
    row=deepcopy(base);row.id='row-'+str(n);row.values[app.work.mapping['rit']]='X-'+str(n);row.actions=[];app.work.rows.append(row)
app._show_work();assert app.rendering
heartbeat=[];app.after(0,lambda:heartbeat.append(True))
limit=time.monotonic()+10
while app.rendering and time.monotonic()<limit:app.update()
assert not app.rendering and heartbeat
assert len(app.records.get_children())==1100
app.records.selection_set('row-1099');app.work_search.set('no-match');app._apply_work_filter()
from nurus.personal.selection import selected_ids
assert 'row-1099' in selected_ids(app,'records')
app.work_search.set('');app._apply_work_filter();assert 'row-1099' in app.records.selection()
""")
