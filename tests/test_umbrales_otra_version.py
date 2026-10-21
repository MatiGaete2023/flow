"""Una configuración guardada por otra versión del Asistente puede traer umbrales que esta no conoce."""

from types import SimpleNamespace

from nurus.personal.app_base import App as BaseApp
from nurus.personal.config import PARAMETER_LABELS, Configuration
from test_integral_ui import run_gui
from test_personal_usability_20260921 import Var

AJENO = 'umbral_de_otra_version'


def perfil_con_umbral_ajeno(directory):
    cfg = Configuration(directory)
    assert AJENO not in PARAMETER_LABELS
    cfg.data['umbrales'][AJENO] = 90
    cfg.save(cfg.data)
    return cfg


def test_umbral_desconocido_no_impide_abrir_y_no_se_muestra(tmp_path):
    perfil_con_umbral_ajeno(tmp_path / 'config')
    run_gui(
        tmp_path,
        """
from nurus.personal.config import PARAMETER_LABELS
assert set(app.param_vars)==set(PARAMETER_LABELS)
assert app.cfg.data['umbrales']['umbral_de_otra_version']==90
app.config_search.set('otra');app.update()
""",
    )


def test_guardar_umbrales_conserva_los_de_otra_version(tmp_path):
    cfg = perfil_con_umbral_ajeno(tmp_path / 'cfg')
    app = SimpleNamespace(
        cfg=cfg,
        param_vars={k: Var(str(v)) for k, v in cfg.data['umbrales'].items() if k in PARAMETER_LABELS},
        disabled={},
        status=Var(''),
    )
    app.param_vars['oido'] = Var('40')
    BaseApp._save_params(app)
    umbrales = Configuration(cfg.directory).data['umbrales']
    assert umbrales[AJENO] == 90 and umbrales['oido'] == 40
