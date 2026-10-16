"""Selector anterior a las apps; rutas empaquetadas y argumentos legados."""

import pytest
from tools import csmp_launcher


@pytest.mark.parametrize('installed', [True, False])
def test_descargador_se_encuentra_en_instalacion_y_paquete_portable(tmp_path, installed):
    base = tmp_path if installed else tmp_path / 'CSMP_Integral'
    base.mkdir(exist_ok=True)
    target = tmp_path / 'SITFA_Descargador' / 'SITFA_Descargador.exe'
    target.parent.mkdir()
    target.write_bytes(b'ejecutable ficticio')
    command, cwd = csmp_launcher.downloader_command(base, frozen=True)
    assert command == [str(target)] and cwd == target.parent


def test_diagnostico_y_acceso_directo_omiten_selector(monkeypatch):
    from nurus.personal import app

    received = []
    monkeypatch.setattr(app, 'main', lambda args: received.append(args))
    monkeypatch.setattr(csmp_launcher, 'choose_mode', lambda: pytest.fail('No debe abrir selector'))
    csmp_launcher.run(['--verificar-paquete', 'diagnostico.json'])
    csmp_launcher.run(['--asistente'])
    assert received == [['--verificar-paquete', 'diagnostico.json'], []]


def test_elegir_descargador_no_crea_asistente(monkeypatch, tmp_path):
    from nurus.personal import app

    script = tmp_path / 'descargador' / 'descargador.py'
    script.parent.mkdir()
    script.write_text('# ficticio')
    monkeypatch.setattr(csmp_launcher, '__file__', str(tmp_path / 'tools/csmp_launcher.py'))
    monkeypatch.setattr(csmp_launcher, 'choose_mode', lambda: 'descargador')
    monkeypatch.setattr(app, 'main', lambda args: pytest.fail('No debe crear Asistente'))
    calls = []
    monkeypatch.setattr(csmp_launcher.subprocess, 'Popen', lambda command, **kw: calls.append((command, kw)))
    csmp_launcher.run([])
    assert calls[0][0][-1] == str(script)
    assert calls[0][1]['cwd'] == script.parent
    assert str(tmp_path / 'src') in calls[0][1]['env']['PYTHONPATH']


def test_falta_descargador_no_busca_ejecutables_ajenos(tmp_path):
    with pytest.raises(FileNotFoundError, match='Reinstala'):
        csmp_launcher.downloader_command(tmp_path, frozen=True)


def test_cerrar_selector_no_abre_apps(monkeypatch):
    from nurus.personal import app

    monkeypatch.setattr(csmp_launcher, 'choose_mode', lambda: None)
    monkeypatch.setattr(app, 'main', lambda args: pytest.fail('Selector cancelado'))
    monkeypatch.setattr(csmp_launcher.subprocess, 'Popen', lambda *a, **k: pytest.fail('Selector cancelado'))
    csmp_launcher.run([])
