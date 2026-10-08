"""El cambio de trabajo en GUI ocurre solo después de validación completa."""
from test_integral_ui import run_gui


def test_import_without_remote_launcher_and_rejected_zip_keeps_edits(tmp_path):
    run_gui(tmp_path,"""
from unittest.mock import patch
import contextlib
patcher=patch('nurus.personal.app_base.simpledialog.askstring',return_value='2026-10-08');patcher.start()
from nurus.capture import write_capture
from test_capture_contract import fixture
m,t=fixture();path=write_capture(root/'capture.zip',m,t)
app._run=lambda label,action,done=None:done(action()) if done else action()
with patch('nurus.personal.app_base.filedialog.askopenfilename',return_value=str(path)):
    app._import_capture()
for _ in range(5):app.update()
assert len(app.work.rows)==4
rid=app.work.rows[0].id
app.records.selection_set(rid);app._detail();app.observation_editor.delete('1.0','end');app.observation_editor.insert('1.0','Decisión humana')
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
""")
