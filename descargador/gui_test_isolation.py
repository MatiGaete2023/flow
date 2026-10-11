"""Una prueba GUI por intérprete Windows, con sus aserciones intactas."""

from functools import wraps
import inspect
import os
from pathlib import Path
import subprocess
import sys


def isolated_gui(function):
    node = str(Path(inspect.getfile(function)).resolve()) + '::' + function.__qualname__.replace('.', '::')

    @wraps(function)
    def run(*args, **kwargs):
        if sys.platform != 'win32' or os.environ.get('DESC_GUI_TEST_CASE') == node:
            return function(*args, **kwargs)
        result = subprocess.run(
            [sys.executable, '-m', 'pytest', '-q', '-o', 'addopts=', node],
            cwd=Path(inspect.getfile(function)).resolve().parent,
            env={**os.environ, 'DESC_GUI_TEST_CASE': node, 'PYTHONIOENCODING': 'utf-8'},
            capture_output=True,
            text=True,
            encoding='utf-8',
            timeout=60,
        )
        if result.returncode:
            raise AssertionError('Prueba GUI Windows aislada falló:\n' + result.stdout + result.stderr)

    return run
