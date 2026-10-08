"""Regresiones del flujo vigente tras retirar la interfaz NuRus duplicada."""
import ast
import os
import subprocess
import sys
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import Mock

from nurus.personal.app import App
from nurus.personal.app_base import App as BaseApp
from nurus.personal.config import Configuration
from test_personal_usability_20260921 import Var

ROOT=Path(__file__).parents[1]


def test_base_entry_opens_same_assistant(monkeypatch):
    from nurus.personal import app_base
    launch=Mock()
    monkeypatch.setattr('nurus.personal.app.main',launch)
    app_base.main()
    launch.assert_called_once_with()


def test_no_duplicate_operational_methods_or_old_interface():
    def methods(path):
        tree=ast.parse(path.read_text(encoding='utf-8'))
        return {n.name for c in tree.body if isinstance(c,ast.ClassDef) for n in c.body if isinstance(n,ast.FunctionDef)}
    folder=ROOT/'src/nurus/personal'
    assert methods(folder/'app.py') & methods(folder/'app_base.py') == {'__init__'}
    assert not (ROOT/'src/nurus/product_ui.py').exists()
    tree=ast.parse((ROOT/'src/nurus/app.py').read_text())
    assert not any(isinstance(n,ast.ClassDef) for n in tree.body)


def test_mode_change_restores_automatic_sheet_without_discarding_work():
    work=object();app=SimpleNamespace(sheet=Var('Espera'),work=work)
    BaseApp._mode_changed(app)
    assert app.sheet.get()=='' and app.work is work


def test_unchanged_refresh_keeps_manual_projects_and_mail():
    work=SimpleNamespace(refresh=lambda:False)
    app=SimpleNamespace(_require_work=lambda:work,_clear_drafts=Mock(),_show_work=Mock(),_save_session=Mock(),_update_context=Mock(),status=Var(''))
    app._run=lambda label,action,done:done(action())
    BaseApp._refresh(app)
    app._clear_drafts.assert_not_called();app._show_work.assert_not_called()
    assert app.status.get()=='La copia no ha cambiado.'


def test_moved_changed_copy_clears_stale_products(monkeypatch):
    work=SimpleNamespace(output='old.xlsx',refresh=Mock(return_value=True))
    app=SimpleNamespace(_require_work=lambda:work,_clear_drafts=Mock(),_show_work=Mock(),_save_session=Mock(),_update_context=Mock(),status=Var(''))
    app._run=lambda label,action,done:done(action())
    monkeypatch.setattr('nurus.personal.app_base.filedialog.askopenfilename',lambda **kw:'moved.xlsx')
    BaseApp._locate(app)
    assert work.output=='moved.xlsx'
    app._clear_drafts.assert_called_once();app._show_work.assert_called_once()


def test_failed_relocation_preserves_previous_path_and_products(monkeypatch):
    import pytest
    work=SimpleNamespace(output='old.xlsx',refresh=Mock(side_effect=ValueError('Identidad ajena')))
    app=SimpleNamespace(_require_work=lambda:work,_clear_drafts=Mock(),_show_work=Mock(),_save_session=Mock(),status=Var(''))
    app._run=lambda label,action,done:done(action())
    monkeypatch.setattr('nurus.personal.app_base.filedialog.askopenfilename',lambda **kw:'other.xlsx')
    with pytest.raises(ValueError,match='Identidad ajena'):BaseApp._locate(app)
    assert work.output=='old.xlsx'
    app._clear_drafts.assert_not_called()


def test_reexport_does_not_reset_manual_resolution_choices(tmp_path):
    work=SimpleNamespace(path='input.xlsx',export=Mock(return_value='copy.xlsx'))
    app=SimpleNamespace(work=work,folder=Var(str(tmp_path)),_capture_observation=Mock(),_show_work=Mock(),_save_session=Mock(),_update_context=Mock(),status=Var(''))
    app._run=lambda label,action,done:done(action())
    App._export_current(app)
    app._show_work.assert_called_once_with(reset_projects=False)


def test_new_work_clears_visible_previous_mail_and_project():
    app=SimpleNamespace(drafts=[object()],draft_index=0,projects=[object()],project_index=0,last_word='old.docx',_prepared_selection=('a',),
        mail_list=Mock(),project_list=Mock(),project_editor=Mock(),body=Mock(),
        to=Var('old@example.cl'),cc=Var('old@example.cl'),subject=Var('Caso anterior'),attach=Var('old.xlsx'))
    app._clear_mail_editor=lambda:BaseApp._clear_mail_editor(app)
    app._clear_projects=lambda:BaseApp._clear_projects(app)
    BaseApp._clear_drafts(app)
    assert not app.drafts and not app.projects and app.draft_index is None and app.project_index is None
    assert all(not v.get() for v in (app.to,app.cc,app.subject,app.attach))
    assert not app.last_word and app._prepared_selection is None
    app.body.delete.assert_called_once();app.project_editor.delete.assert_called_once()


def test_empty_mail_result_does_not_show_previous_case():
    app=SimpleNamespace(mail_list=Mock(),draft_index=0,drafts=[object()],_clear_mail_editor=Mock(),status=Var(''))
    App._display_prepared_drafts(app,[],'{count} borradores')
    app._clear_mail_editor.assert_called_once()
    assert not app.drafts and app.draft_index is None


def test_saving_thresholds_keeps_rules_not_exposed_by_ui(tmp_path):
    cfg=Configuration(tmp_path/'cfg')
    cfg.data['desactivadas']=['COMUN.PROX_AUDIENCIA','COMUN.OIDO']
    app=SimpleNamespace(cfg=cfg,param_vars={k:Var(str(v)) for k,v in cfg.data['umbrales'].items()},
        disabled={'COMUN.OIDO':Var(True),'COMUN.CURADOR':Var(False)},status=Var(''))
    BaseApp._save_params(app)
    assert set(Configuration(cfg.directory).data['desactivadas'])=={'COMUN.PROX_AUDIENCIA','COMUN.CURADOR'}


def test_active_assistant_import_does_not_load_historical_workflow_stack():
    """El arranque vigente no debe materializar SQLite ni el evaluador NuRus retirado."""
    code = (
        "import sys; import nurus.personal.app; "
        "assert 'nurus.services.workflow' not in sys.modules; "
        "assert 'nurus.storage.database' not in sys.modules; "
        "assert 'nurus.rus.service' not in sys.modules"
    )
    env = dict(os.environ)
    env['PYTHONPATH'] = str(ROOT/'src')
    subprocess.run([sys.executable, '-c', code], cwd=ROOT, env=env, check=True)
