"""Continuación visible y exportación automática con archivos locales sintéticos."""

import os
from pathlib import Path
import subprocess
import sys

from test_integral_ui import run_gui


def test_importar_exporta_y_reimportar_conserva_copia_y_ediciones(tmp_path):
    run_gui(
        tmp_path,
        """
from unittest.mock import patch
from nurus.capture import write_capture
from test_capture_contract import fixture
m,t=fixture();path=write_capture(root/'paquete.zip',m,t)
app._run=lambda label,action,done=None:done(action()) if done else action()
with patch.object(app,'_ask_cut_date',return_value=date(2026,10,8)):
    app._import_capture(path)
for _ in range(5):app.update()
assert app.work.output and Path(app.work.output).is_file()
first_output=app.work.output
assert 'Excel generado' in app.status.get()
rid=app.work.rows[0].id;app.records.selection_set(rid);app._detail()
app.observation_editor.delete('1.0','end');app.observation_editor.insert('1.0','Revisión humana ficticia')
with patch.object(app,'_ask_cut_date',return_value=date(2026,10,8)):
    app._import_capture(path)
for _ in range(5):app.update()
assert app.work.output==first_output
assert app.work.rows[0].review['OBSERVACION']=='Revisión humana ficticia'
""",
    )


def test_fallo_exportacion_conserva_importacion_y_permite_reintentar(tmp_path):
    run_gui(
        tmp_path,
        """
from unittest.mock import patch
from nurus.capture import write_capture
from test_capture_contract import fixture
m,t=fixture();path=write_capture(root/'paquete.zip',m,t)
app._run=lambda label,action,done=None:done(action()) if done else action()
with patch.object(app,'_ask_cut_date',return_value=date(2026,10,8)),patch.object(Work,'export',side_effect=OSError('Fallo de disco ficticio')):
    try:app._import_capture(path)
    except OSError:pass
    else:raise AssertionError('No informó error de exportación')
assert app.work and len(app.work.rows)==4 and not app.work.output
app.next_step.refresh(force=True)
assert app.next_step.action_button.cget('text')=='Reintentar copia Excel'
app.next_step.next_action()
assert Path(app.work.output).is_file()
""",
    )


def test_stepper_y_siguiente_accion_navegan_sin_efectos_office(tmp_path):
    run_gui(
        tmp_path,
        """
from unittest.mock import patch
from nurus.personal.outputs import Draft
assert len(app.next_step.flow_labels)==4
app.work=Work(app.cfg.data).analyze(source(root),'ESPERA',as_of=date(2026,10,2))
app.work.export(root/'copia.xlsx',backend='portable',reduced_fidelity=True);app._show_work()
for _ in range(5):app.update()
assert app.import_button.visual_role=='secondary'
assert app.work_primary_button.visual_role=='primary'
assert app.mail_action_buttons['preparar_todos'].visual_role=='primary'
app.next_step.next_action();app.update()
assert app.tabs.select()==str(app.pages['Correos'])
with patch.object(app,'_send_all',side_effect=AssertionError('No guardar')),patch.object(app,'_generate_words',side_effect=AssertionError('No generar')):
    app.mail_target.set('todos')
    app._display_prepared_drafts([Draft('Ficticio','Cuerpo',to='a@example.test')],'{count} preparados')
    app.geometry('1024x650');app.update()
    assert not app.result_notice_label.winfo_ismapped()
    assert app.body.winfo_height()>=120
    app.next_step.next_action();app.update()
assert app.tabs.select()==str(app.pages['Resoluciones'])
assert app.mail_action_buttons['guardar_todos'].visual_role=='primary'
app.next_step.toggle_details();app.update()
assert app.next_step.details.winfo_manager()
assert set(app.next_step.buttons)=={'revisar','exportar','correos','resoluciones','resultados'}
for label in app.next_step.flow_labels.values():
    assert label.winfo_rootx()+label.winfo_width()<=app.winfo_rootx()+app.winfo_width()
app.status.set('Producto ficticio terminado');app.busy=True;app.status.set('Nueva fase en curso')
app.update()
assert app.result_notice.get()=='Producto ficticio terminado'
assert app.result_notice_label.winfo_ismapped()
app.result_notice.set('');app.update()
assert not app.result_notice_label.winfo_ismapped()
app.busy=False
""",
    )


def test_descargador_comparte_preferencia_y_expone_ruta_final(tmp_path):
    root = Path(__file__).resolve().parents[1]
    script = """
from pathlib import Path
import sys,tkinter as tk
from descargador import Application
from ventana_resultados import ResultsWindow
from test_lotes import dispose_interface
root=tk.Tk();root.withdraw();folder=Path(sys.argv[1])
app=Application(root,start_worker=False,preferences_path=folder/'prefs.json')
try:
    assert not hasattr(app,'signature_days')
    window=ResultsWindow(app);app.result_windows.append(window)
    assert window.package_consolidated is app.package_consolidated
    app.busy=True;app.refresh();window.poll()
    assert str(app.package_choice.cget('state'))=='disabled'
    assert str(window.package_choice.cget('state'))=='disabled'
    app.busy=False;app.refresh();window.poll()
    app.package_consolidated.set(False);app.save_preferences()
    from preferencias import Preferences
    assert Preferences(folder/'prefs.json').data['paquete_consolidado'] is False
    assert not app.advanced_actions.winfo_manager()
    app.toggle_advanced();assert app.advanced_actions.winfo_manager()
    app.toggle_advanced();assert not app.advanced_actions.winfo_manager()
    target=folder/'Captura.zip';target.write_bytes(b'zip ficticio')
    app.process_event('package_done',{'path':str(target),'reuse':'Fuentes verificadas','partial':True})
    app.process_event('idle',None)
    assert 'parcial' in app.result_notice.get().lower()
    assert str(app.copy_path_button.cget('state'))=='normal'
    app.copy_package_path();assert root.clipboard_get()==str(target.resolve())
    assert 'Captura.zip' in app.result_notice.get()
    from unittest.mock import patch
    opened=[];app.last_folder=folder/'otra-descarga';app.last_folder.mkdir()
    with patch('ventana_resultados.open_local',side_effect=lambda path:opened.append(path)):
        app.open_package_folder()
    assert opened==[target.parent]
    window.close()
    root.deiconify();root.geometry('1000x650');root.update()
    for control in (app.pipeline_button,app.live_pause,app.live_cancel,app.copy_path_button):
        assert control.winfo_rooty()+control.winfo_height()<=root.winfo_rooty()+root.winfo_height()
finally:dispose_interface(app,root)
"""
    result = subprocess.run(
        [sys.executable, '-c', script, str(tmp_path)],
        cwd=root / 'descargador',
        env={**os.environ, 'PYTHONPATH': str(root / 'src')},
        capture_output=True,
        text=True,
        timeout=45,
    )
    assert result.returncode == 0, result.stdout + result.stderr
