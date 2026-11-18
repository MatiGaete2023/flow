import ast
from pathlib import Path

from nurus.personal import commands


class Status:
    def __init__(self):
        self.value = ''

    def set(self, value):
        self.value = value


class App:
    def __init__(self):
        self.status = Status()


def test_manual_fuera_de_windows_indica_ruta_sin_iniciar_procesos(tmp_path, monkeypatch):
    manual = tmp_path / 'Manual_Asistente_CSMP.pdf'
    manual.write_bytes(b'%PDF-ficticio')
    monkeypatch.setattr(commands, 'find_doc', lambda name: manual)
    monkeypatch.setattr('sys.platform', 'linux')
    app = App()
    commands.open_doc(app, manual.name)
    assert str(manual) in app.status.value


def test_commands_no_importa_subprocess():
    tree = ast.parse(Path(commands.__file__).read_text(encoding='utf-8'))
    nombres = [a.name for n in ast.walk(tree) if isinstance(n, ast.Import) for a in n.names]
    nombres += [n.module or '' for n in ast.walk(tree) if isinstance(n, ast.ImportFrom)]
    assert 'subprocess' not in nombres
