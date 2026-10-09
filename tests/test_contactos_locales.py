"""WP-12: contactos fuera del paquete y sesiones sin copia de la configuración."""
from datetime import date
import json

import pytest
from openpyxl import Workbook

from nurus.personal import config as configuracion
from nurus.personal.config import Configuration, base_contacts, defaults, save_contacts
from nurus.personal.work import Work


@pytest.fixture(autouse=True)
def contactos_aislados(tmp_path, monkeypatch):
    monkeypatch.setenv('LOCALAPPDATA', str(tmp_path))


def test_sin_archivo_local_se_usa_el_ejemplo_y_se_avisa(tmp_path):
    contactos, local = base_contacts(tmp_path)
    assert not local and len(contactos) == 3
    assert all(direccion.endswith('@example.invalid') for direccion in contactos.values())
    assert not (configuracion.BASE / 'contactos_base.json').exists()
    cfg = Configuration(tmp_path / 'perfil')
    assert cfg.contacts_missing and cfg.data['contactos'] == contactos


def test_archivo_local_con_programas_y_tribunales(tmp_path):
    local = tmp_path / 'CSMP_Personal'
    local.mkdir()
    (local / 'contactos.json').write_text(json.dumps([
        {'Nombre': 'PRM FICTICIO', 'Mail': 'prm@example.test'},
        {'Nombre': 'Tribunal: LAJA', 'Mail': 'uno@example.test; dos@example.test'},
    ]), encoding='utf-8')
    datos = defaults(local)
    assert datos['contactos'] == {'PRM FICTICIO': 'prm@example.test'}
    assert datos['correos']['tribunales']['LAJA']['para'] == ['uno@example.test', 'dos@example.test']
    cfg = Configuration(tmp_path)
    assert not cfg.contacts_missing


def test_save_contacts_escribe_formato_importable(tmp_path):
    save_contacts({'B': 'b@example.test', 'A': 'a@example.test'}, tmp_path)
    assert base_contacts(tmp_path) == ({'A': 'a@example.test', 'B': 'b@example.test'}, True)


def test_formato_historico_de_objetos_separados_por_coma():
    texto = '{"Nombre": "X", "Mail": "x@example.test", "Director": "Ficticio"},\n{"Nombre": "Y", "Mail": "no valido"},'
    assert configuracion.parse_contacts(texto) == {'X': 'x@example.test'}


def _trabajo(tmp_path):
    libro = Workbook()
    hoja = libro.active
    hoja.title = 'ESPERA'
    hoja.append(['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'NOMBRE CENTRO', 'T ESPERA'])
    hoja.append(['X-1-2026', 'LAJA', 'Persona Ficticia', '11111111-1', 'PRM UNO', 45])
    ruta = tmp_path / 'fuente.xlsx'
    libro.save(ruta)
    trabajo = Work(defaults())
    trabajo.analyze(ruta, 'ESPERA', as_of=date(2026, 10, 9))
    return trabajo


def test_sesion_nueva_sin_config_y_con_huella(tmp_path):
    trabajo = _trabajo(tmp_path)
    trabajo.save(tmp_path / 'sesion')
    datos = json.loads((tmp_path / 'sesion' / 'trabajo.json').read_text(encoding='utf-8'))
    assert 'config' not in datos and len(datos['config_sha256']) == 64
    actual = defaults()
    actual['firma'] = 'Firma vigente'
    restaurado = Work.load(tmp_path / 'sesion', actual)
    assert restaurado.config is actual and restaurado.rows[0].observation == trabajo.rows[0].observation
    assert Work.load(tmp_path / 'sesion').config['contactos'] == defaults()['contactos']


def test_sesion_antigua_con_config_todavia_carga(tmp_path):
    trabajo = _trabajo(tmp_path)
    trabajo.save(tmp_path / 'sesion')
    archivo = tmp_path / 'sesion' / 'trabajo.json'
    datos = json.loads(archivo.read_text(encoding='utf-8'))
    antigua = defaults()
    antigua['firma'] = 'Firma de la sesión antigua'
    datos['config'] = antigua
    datos.pop('config_sha256')
    archivo.write_text(json.dumps(datos, ensure_ascii=False), encoding='utf-8')
    restaurado = Work.load(tmp_path / 'sesion', defaults())
    assert restaurado.config['firma'] == 'Firma de la sesión antigua'
    assert len(restaurado.rows) == 1


def test_boton_importar_contactos_json_guarda_copia_local(tmp_path):
    from test_integral_ui import run_gui
    run_gui(tmp_path, """
import json
from unittest.mock import patch
origen=root/'contactos_base.json'
origen.write_text('{"Nombre": "PRM FICTICIO", "Mail": "prm@example.test", "Director": "Ficticio"},\\n'
                  '{"Nombre": "Tribunal: TOME", "Mail": "tome@example.test"},',encoding='utf-8')
assert app.cfg.contacts_missing and 'Importar contactos' in app.status.get()
app.update()
assert app._contacts_dialog.winfo_exists() and app._contacts_dialog.title()=='Importar contactos'
from customtkinter import CTkButton
dialog=app._contacts_dialog
buttons=[child for child in dialog.winfo_children() if isinstance(child,CTkButton)]
assert len(buttons)==2
assert all(button.winfo_rooty()+button.winfo_height() <= dialog.winfo_rooty()+dialog.winfo_height() for button in buttons)
with patch('nurus.personal.app_base.filedialog.askopenfilename',return_value=str(origen)):
    app._import_contacts()
assert app.cfg.data['contactos']=={'PRM FICTICIO':'prm@example.test'}
assert app.cfg.data['correos']['tribunales']['TOME']['para']==['tome@example.test']
guardado=json.loads((app.cfg.contacts_directory/'contactos.json').read_text(encoding='utf-8'))
assert guardado==[{'Nombre':'PRM FICTICIO','Mail':'prm@example.test'},{'Nombre':'Tribunal: TOME','Mail':'tome@example.test'}]
assert not app.cfg.contacts_missing and app.contacts_notice.get()==''
""")


def test_perfil_historico_usa_directorio_contactos_csmp_personal(tmp_path):
    destino = configuracion.user_directory()
    save_contacts({'PRM FICTICIO': 'prm@example.invalid'}, destino)
    cfg = Configuration(tmp_path / 'CSMP_Personal_Prototipo_Integral')
    assert cfg.directory != cfg.contacts_directory
    assert cfg.contacts_directory == tmp_path / 'CSMP_Personal'
    assert cfg.data['contactos'] == {'PRM FICTICIO': 'prm@example.invalid'}
    assert not cfg.contacts_missing


def test_recuperacion_sin_trabajo_activo_usa_configuracion_vigente(tmp_path):
    from test_download_transfer import App, ready
    from nurus.personal import download_link
    from current_fixture import source
    path = source(tmp_path)
    anterior = App(tmp_path)
    solicitud = ready(anterior, path)
    download_link.incorporate(anterior, solicitud)
    reiniciado = App(tmp_path)
    reiniciado.cfg.data['firma'] = 'Firma vigente del perfil'
    reiniciado.cfg.data['contactos'] = {'PRM FICTICIO': 'vigente@example.invalid'}
    download_link.incorporate(reiniciado, solicitud)
    assert reiniciado.analyses == 0
    assert reiniciado.work.config is reiniciado.cfg.data
    assert reiniciado.work.config['firma'] == 'Firma vigente del perfil'
    assert reiniciado.work.config['contactos'] == {'PRM FICTICIO': 'vigente@example.invalid'}


def test_defaults_y_golden_no_cargan_directorio_local_del_usuario(tmp_path):
    local = configuracion.user_directory()
    save_contacts({'PRM LOCAL FICTICIO': 'usuario-local@example.invalid'}, local)
    base = defaults()
    assert len(base['contactos']) == 3
    assert 'PRM LOCAL FICTICIO' not in base['contactos']
    cfg = Configuration(tmp_path / 'perfil')
    assert cfg.data['contactos'] == {'PRM LOCAL FICTICIO': 'usuario-local@example.invalid'}


def test_perfil_anterior_conserva_contactos_al_pedir_importacion(tmp_path):
    profile = tmp_path / 'perfil_anterior'
    cfg = Configuration(profile)
    datos = cfg.data
    datos['contactos'] = {'PRM CONSERVADO FICTICIO': 'conservado@example.invalid'}
    datos['firma'] = 'Edición local ficticia'
    cfg.save(datos)
    recuperada = Configuration(profile)
    assert recuperada.contacts_missing
    assert recuperada.data['contactos'] == {'PRM CONSERVADO FICTICIO': 'conservado@example.invalid'}
    assert recuperada.data['firma'] == 'Edición local ficticia'
