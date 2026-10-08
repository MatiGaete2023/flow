"""Windows: selección de tribunal, preparación asíncrona y editor de borrador."""
import os
import subprocess
import sys
import time

import pytest


@pytest.mark.skipif(os.name != 'nt', reason='Windows GUI contract')
def test_mulchen_draft_is_visible_after_automatic_and_manual_preparation(tmp_path):
    env = dict(os.environ, PYTHONPATH=os.pathsep.join(sys.path))
    script = 'from pathlib import Path; import sys; from test_mulchen_mail_gui import probe; probe(Path(sys.argv[1]))'
    result = subprocess.run([sys.executable, '-c', script, str(tmp_path)],
                            env=env, capture_output=True, text=True, timeout=45)
    assert result.returncode == 0, result.stdout + '\n' + result.stderr


def probe(tmp_path):
    from unittest.mock import patch
    from nurus.personal.app import App
    from nurus.personal.config import Configuration
    from test_residential_mail import residential_work
    cfg = Configuration(tmp_path/'profile')
    # Configuración antigua con clave descriptiva en lugar del ID estable.
    cfg.data['correos']['tribunales']['MULCHËN'] = cfg.data['correos']['tribunales'].pop('MULCHEN')
    app = App(cfg)
    try:
        app.work = residential_work(tmp_path)
        app._show_work()
        app.mail_target.set('tribunales')
        app.mail_kind.set('cumplimiento')
        for key,var in app.modality_vars.items():var.set(key == 'RES')
        app.mail_courts.selection_set(app.mail_court_keys.index('MULCHËN'))
        app.tabs.select(app.pages['Correos'])
        errors = []
        with patch('tkinter.messagebox.showerror', side_effect=lambda *args: errors.append(args)):
            for manual in (False, True):
                app.manual_mail.set(manual)
                app.records.selection_set(app.work.rows[0].id)
                app.update()
                app._prepare_mail()
                deadline = time.monotonic() + 10
                while app.busy and time.monotonic() < deadline:
                    app.update()
                    time.sleep(0.01)
                assert not app.busy
                app.update_idletasks()
                assert not errors, errors
                assert len(app.drafts) == 1
                assert len(app.drafts[0].record_ids) == (1 if manual else 7)
                assert app.draft_index == 0
                assert app.to.get() == '; '.join(cfg.data['correos']['tribunales']['MULCHËN']['para'])
                assert 'Mulchén' in app.subject.get()
                assert app.mail_list.cards[0].winfo_ismapped()
    finally:
        app.destroy()
