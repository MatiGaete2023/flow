"""El cambio de trabajo en GUI ocurre solo después de validación completa."""

from test_integral_ui import run_gui


def test_import_without_remote_launcher_and_rejected_zip_keeps_edits(tmp_path):
    run_gui(
        tmp_path,
        """
from unittest.mock import patch
import contextlib
patcher=patch.object(app,'_ask_cut_date',return_value=date(2026,10,8));patcher.start()
from nurus.capture import write_capture
from test_capture_contract import fixture
m,t=fixture();path=write_capture(root/'capture.zip',m,t)
app._run=lambda label,action,done=None:done(action()) if done else action()
with patch('nurus.personal.app_base.filedialog.askopenfilename',return_value=str(path)):
    app._import_capture()
for _ in range(5):app.update()
assert len(app.work.rows)==4
rid=app.work.rows[0].id
app.records.selection_set(rid);app._detail()
# WP-50: la columna y el panel solo muestran la bitácora capturada.
assert app.records.set(rid,'Última obs. centro')=='10/09/2026'
assert app.work_novelties.get().startswith('Novedades posteriores · solo mostrar')
app.observation_editor.delete('1.0','end');app.observation_editor.insert('1.0','Decisión humana')
with patch('nurus.personal.app_base.filedialog.askopenfilename',return_value=str(path)):
    app._import_capture()
for _ in range(5):app.update()
assert app.work.rows[0].review['OBSERVACION']=='Decisión humana'
from test_capture_contract import mutate
mutate(path,lambda items:items.update({'unsafe':b'x'}))
previous=app.work
try:
    with patch('nurus.personal.app_base.filedialog.askopenfilename',return_value=str(path)):
        app._import_capture()
except ValueError:pass
else:raise AssertionError('Paquete corrupto aceptado')
assert app.work is previous and app.work.rows[0].review['OBSERVACION']=='Decisión humana'
try:app._download_joint()
except ValueError:pass
else:raise AssertionError('Lanzador remoto activo')
app._capture_evidence();app.update()
""",
    )


def test_complementary_import_restores_live_products_after_large_redraw(tmp_path):
    run_gui(
        tmp_path,
        """
from unittest.mock import patch
from nurus.capture import write_capture
from nurus.capture.fixtures import fixture
from nurus.personal.outputs import Draft
from nurus.personal.selection import selected_ids
import time
m,t=fixture(600);path=write_capture(root/'base.zip',m,t)
app._run=lambda label,action,done=None:done(action()) if done else action()
def import_file(path):
    with patch('nurus.personal.app_base.filedialog.askopenfilename',return_value=str(path)),patch.object(app,'_ask_cut_date',return_value=date(2026,10,8)):
        app._import_capture()
    deadline=time.monotonic()+10
    while getattr(app,'rendering',False) and time.monotonic()<deadline:app.update()
    assert not getattr(app,'rendering',False)
    app.update()
import_file(path)
app.records.selection_set('r0');app.work.rows[0].excluded=True
app.mail_target.set('todos')
app._display_prepared_drafts([Draft('Ficticio','Texto original',kind='programa_espera',record_ids=['r0'])],'{count}')
app.body.delete('1.0','end');app.body.insert('1.0','Edición humana todavía abierta')
app._prepared_selection=('r0',)
m['downloader']['commit']='COMPLEMENTARIO'
for row in t['registros']:row['record_id']='new-'+row['record_id']
path=write_capture(root/'complement.zip',m,t)
import_file(path)
assert app.work.rows[0].excluded
assert app.drafts[0].body=='Edición humana todavía abierta'
assert app.drafts[0].record_ids==['new-r0']
assert app.work.session['drafts'][0]['record_ids']==['new-r0']
assert app._prepared_selection==('new-r0',)
assert 'new-r0' in selected_ids(app,'records')
assert all(not iid.startswith('r0|') for iid in app._resolution_all_iids)
restored=Work.load(app.cfg.directory/'sesion')
assert restored.session['drafts'][0]['record_ids']==['new-r0']
assert restored.session['drafts'][0]['body']=='Edición humana todavía abierta'
assert len(list((app.cfg.directory/'sesion'/'anteriores').iterdir()))==1
""",
    )
