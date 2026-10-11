"""G-3: editores reales no mutan el trabajo mientras el worker usa su instantánea."""

import pytest

from test_integral_ui import run_gui


WORKER = '''
from threading import Event
import time
release=Event()
def finish_worker():
    release.set()
    until=time.monotonic()+10
    while app.busy and time.monotonic()<until:
        app.update()
        time.sleep(.01)
    assert not app.busy
'''


def test_manual_editor_cannot_change_inputs_during_worker(tmp_path):
    run_gui(
        tmp_path,
        WORKER
        + '''
import tkinter as tk
from tkinter import ttk
from nurus.personal.manual import open_editor
open_editor(app, 'mail')
win=app._manual_windows['mail']
def descendants(widget):
    for child in widget.winfo_children():
        yield child
        yield from descendants(child)
entry=next(w for w in descendants(win) if isinstance(w, ttk.Entry))
text=next(w for w in descendants(win) if isinstance(w, tk.Text))
entry.insert(0, 'a@example.test')
text.insert('1.0', 'Contenido ficticio')
before=(entry.get(), text.get('1.0', 'end-1c'))
app._run('Worker ficticio', lambda: release.wait(10))
try:
    assert app.busy and app.cancel_button.cget('state')=='normal'
    entry.insert(0, 'CAMBIO')
    text.insert('1.0', 'CAMBIO')
    assert (entry.get(), text.get('1.0', 'end-1c'))==before
    assert entry.instate(['disabled'])
    assert text.cget('state')=='disabled'
finally:
    finish_worker()
assert not entry.instate(['disabled']) and text.cget('state')=='normal'
entry.insert(0, 'EDITABLE')
assert entry.get().startswith('EDITABLE')
''',
    )


def test_preferences_cannot_save_configuration_during_worker(tmp_path):
    run_gui(
        tmp_path,
        WORKER
        + '''
import tkinter as tk
from tkinter import ttk
from copy import deepcopy
from nurus.personal.work_tools import preferences
preferences(app)
win=next(w for w in app.winfo_children() if isinstance(w, tk.Toplevel) and w.title()=='Preferencias de vista')
check=next(w for w in win.winfo_children() if isinstance(w, ttk.Checkbutton))
check.invoke()
button=next(w for w in win.winfo_children() if isinstance(w, ttk.Button))
before=deepcopy(app.cfg.data)
app._run('Worker ficticio', lambda: release.wait(10))
try:
    button.invoke()
    assert win.winfo_exists(), 'Guardar vista se ejecutó durante el worker'
    assert app.cfg.data==before
    assert check.instate(['disabled']) and button.instate(['disabled'])
finally:
    finish_worker()
button.invoke()
assert not win.winfo_exists()
''',
    )


@pytest.mark.parametrize('custom', [False, True])
def test_nested_windows_restore_readonly_selected_and_disabled_states(tmp_path, custom):
    run_gui(
        tmp_path,
        f'custom={custom!r}\n'
        + '''
import tkinter as tk
from tkinter import ttk
import customtkinter as ctk
from nurus.personal.interaction import lock, unlock
outer=(ctk.CTkToplevel if custom else tk.Toplevel)(app)
inner=tk.Toplevel(outer)
classic=tk.Entry(inner, state='readonly')
text=tk.Text(inner)
combo=ttk.Combobox(inner, state='readonly')
checked=ttk.Checkbutton(inner)
checked.state(['selected'])
disabled=ttk.Button(inner)
disabled.state(['disabled'])
modern=ctk.CTkEntry(inner)
modern_disabled=ctk.CTkButton(inner, state='disabled')
for widget in (classic, text, combo, checked, disabled, modern, modern_disabled):
    widget.pack()
app.update()
lock(app)
assert classic.cget('state')=='disabled'
assert text.cget('state')=='disabled'
assert combo.instate(['disabled', 'readonly'])
assert checked.instate(['disabled', 'selected'])
assert disabled.instate(['disabled'])
assert modern.cget('state')=='disabled'
unlock(app)
assert classic.cget('state')=='readonly'
assert text.cget('state')=='normal'
assert combo.instate(['!disabled', 'readonly'])
assert checked.instate(['!disabled', 'selected'])
assert disabled.instate(['disabled'])
assert modern.cget('state')=='normal'
assert modern_disabled.cget('state')=='disabled'
''',
    )


def test_closed_secondary_window_does_not_prevent_unlock(tmp_path):
    run_gui(
        tmp_path,
        '''
import tkinter as tk
from tkinter import ttk
from nurus.personal.interaction import lock, unlock
first=tk.Toplevel(app)
second=tk.Toplevel(app)
entry=ttk.Entry(second)
lock(app)
assert entry.instate(['disabled'])
first.destroy()
unlock(app)
assert not entry.instate(['disabled'])
assert app._locked_inputs==[]
''',
    )


def test_failed_secondary_lock_restores_inputs_already_disabled(tmp_path):
    run_gui(
        tmp_path,
        '''
import tkinter as tk
from tkinter import ttk
from unittest.mock import patch
from nurus.personal.interaction import lock
win=tk.Toplevel(app)
entry=ttk.Entry(win)
text=tk.Text(win)
original=text.configure
def fail_disable(*args, **kwargs):
    if kwargs.get('state')=='disabled':
        raise OSError('fallo ficticio')
    return original(*args, **kwargs)
with patch.object(text, 'configure', side_effect=fail_disable):
    try:
        lock(app)
    except OSError as error:
        assert str(error)=='fallo ficticio'
    else:
        raise AssertionError('No se intentó bloquear el texto de la ventana secundaria')
assert not entry.instate(['disabled'])
assert text.cget('state')=='normal'
assert app._locked_inputs==[]
''',
    )
