from datetime import timedelta
from hashlib import sha256
import json
import os
from pathlib import Path
import subprocess
import sys

import pytest

from nurus.personal.registration import FormEvidence, submit
from nurus.personal.rus_capture import inspect_capture, main
from test_registration import intent, setup


def capture(tmp_path):
    documents = [{'frame_id': 0, 'document': {
        'schema': 'rus-document-v1', 'url': 'https://familia.pjud.cl/SITFAWEB/fixture',
        'forms': [{'index': 0, 'name': 'Ficticio', 'id': '', 'action': 'https://familia.pjud.cl/fixture',
                   'method': 'post', 'controls': [{'tag': 'textarea', 'type': 'textarea', 'name': 'texto',
                                                 'id': 'obs', 'max_length': '3000', 'visible': True,
                                                 'disabled': False, 'value': 'Texto 😀', 'redacted': False}]}],
        'tables': [], 'capabilities': {'writes': False, 'complete_entries': False, 'server_time_verified': False}
    }}]
    data = {'schema': 'rus-capture-v1', 'stage': 'formulario', 'captured_at': '2026-10-06T13:00:00Z',
            'documents': documents}
    write(tmp_path / 'capture.json', data)
    return tmp_path / 'capture.json', data


def write(path, data):
    data['documents_sha256'] = sha256(json.dumps(data['documents'], ensure_ascii=False,
                                                separators=(',', ':')).encode()).hexdigest()
    path.write_text(json.dumps(data, ensure_ascii=False), encoding='utf-8')


def test_inspection_inventories_fields_without_claiming_a_real_registration(tmp_path):
    path, _ = capture(tmp_path)
    report = inspect_capture(path)
    assert report['registration_enabled'] is False
    assert report['forms'][0]['text_fields'][0]['max_length'] == '3000'
    assert 'Texto' not in json.dumps(report)  # El informe de estructura no duplica el texto personal.
    destination = tmp_path / 'report.json'
    main([str(path), '--salida', str(destination)])
    assert json.loads(destination.read_text(encoding='utf-8')) == report
    with pytest.raises(SystemExit):
        main([str(path), '--salida', str(destination)])
    assert json.loads(destination.read_text(encoding='utf-8')) == report


def test_changed_capture_is_rejected(tmp_path):
    path, data = capture(tmp_path)
    data['documents'][0]['document']['forms'][0]['name'] = 'Cambiado'
    path.write_text(json.dumps(data), encoding='utf-8')
    with pytest.raises(ValueError, match='huella'):
        inspect_capture(path)


@pytest.mark.parametrize('url', ['https://otro.example/form', 'https://familia.pjud.cl.ejemplo/form',
                               'http://familia.pjud.cl/form', 'https://familia.pjud.cl/form?token=secreto'])
def test_noninstitutional_or_undepurated_url_is_rejected(tmp_path, url):
    path, data = capture(tmp_path)
    data['documents'][0]['document']['url'] = url
    write(path, data)
    with pytest.raises(ValueError, match='origen'):
        inspect_capture(path)


@pytest.mark.parametrize('limit', [None, 3000])
def test_observed_limit_replaces_fixed_assumption(limit):
    data = intent(text='😀' * 1100)
    now = setup_clock()
    FormEvidence(data, limit, now.isoformat(), 'f' * 64).validate(data, now)


def setup_clock():
    from test_registration import Clock
    return Clock()()


@pytest.mark.parametrize('limit', [-1, True, '2000'])
def test_invalid_form_limit_blocks(limit):
    data = intent()
    now = setup_clock()
    with pytest.raises(ValueError, match='límite'):
        FormEvidence(data, limit, now.isoformat(), 'f' * 64).validate(data, now)


def test_unknown_or_stale_form_evidence_prevents_any_save(tmp_path):
    clock, data, journal, operation, adapter = setup(tmp_path)
    adapter.prepare_form = lambda data: data
    with pytest.raises(ValueError, match='límite observado'):
        submit(journal, operation, adapter)
    assert adapter.saves == 0 and journal.get(operation)['state'] == 'PREPARADA'
    adapter.prepare_form = lambda data: FormEvidence(data, None,
        (clock() - timedelta(seconds=121)).isoformat(), 'f' * 64)
    with pytest.raises(ValueError, match='lectura reciente'):
        submit(journal, operation, adapter)
    assert adapter.saves == 0 and journal.get(operation)['state'] == 'PREPARADA'


def test_preparation_timeout_invalidates_baseline_and_does_not_save(tmp_path):
    clock, data, journal, operation, adapter = setup(tmp_path)
    def prepare(data):
        clock.now += timedelta(seconds=121)
        return FormEvidence(data, 3000, clock().isoformat(), 'f' * 64)
    adapter.prepare_form = prepare
    with pytest.raises(ValueError, match='antigua'):
        submit(journal, operation, adapter)
    assert adapter.saves == 0 and journal.get(operation)['state'] == 'PREPARADA'


def test_form_limit_source_is_durable_before_save(tmp_path):
    clock, data, journal, operation, adapter = setup(tmp_path)
    original = adapter.save
    def save(data):
        record = journal.get(operation)
        assert record['state'] == 'ENVIANDO'
        assert record['form_max_text_units'] == 2000 and record['form_sha256'] == 'd' * 64
        original(data)
    adapter.save = save
    assert submit(journal, operation, adapter)['verified'] is True


@pytest.mark.skipif(sys.platform != 'win32' and not os.environ.get('DISPLAY'), reason='Interfaz Windows o pantalla de prueba requerida')
def test_separate_inspector_opens_capture_and_saves_current_structure(tmp_path):
    path, _ = capture(tmp_path)
    destination = tmp_path / 'gui-report.json'
    script = '''
import sys
from pathlib import Path
from tkinter import ttk
from nurus.personal import rus_capture_app as app
original = app.tk.Tk
def root():
    window = original()
    def click():
        frame = window.winfo_children()[0]
        button = next(widget for widget in frame.winfo_children() if isinstance(widget, ttk.Button))
        button.invoke()
        window.destroy()
    window.after(100, click)
    return window
app.tk.Tk = root
app.filedialog.askopenfilename = lambda **kw: sys.argv[1]
app.filedialog.asksaveasfilename = lambda **kw: sys.argv[2]
app.main()
assert Path(sys.argv[2]).is_file()
'''
    env = dict(os.environ, PYTHONPATH=str(Path(__file__).resolve().parents[1] / 'src'))
    result = subprocess.run([sys.executable, '-c', script, str(path), str(destination)],
                            env=env, capture_output=True, text=True, timeout=15)
    assert result.returncode == 0, result.stderr
    assert json.loads(destination.read_text(encoding='utf-8')) == inspect_capture(path)
