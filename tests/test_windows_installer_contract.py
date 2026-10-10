from pathlib import Path
import os
import subprocess
import sys

import pytest


def test_windows_installer_detects_supported_python_without_requiring_default_launcher():
    script = (Path(__file__).parents[1] / "Instalar_CSMP.bat").read_text(encoding="utf-8")
    assert "call :try_python py -3.14" in script
    assert "call :try_python py -3.13" in script
    assert "call :try_python py -3.12" in script
    assert "call :try_python python" in script
    assert "%LOCALAPPDATA%\\Programs\\Python\\Python314\\python.exe" in script
    assert "%LOCALAPPDATA%\\Programs\\Python\\Python313\\python.exe" in script
    assert "%LOCALAPPDATA%\\Programs\\Python\\Python312\\python.exe" in script
    assert "(3, 12) <= v <= (3, 14)" in script
    assert "excel-native" in script


def test_detected_python_is_expanded_after_find_subroutine_returns():
    script = (Path(__file__).parents[1] / "Instalar_CSMP.bat").read_text(encoding="utf-8")
    assert "if not exist \".venv-csmp\\Scripts\\python.exe\" goto :create_venv" in script
    assert ":create_venv\ncall :find_python" in script
    assert "echo Python compatible 3.12-3.14 detectado: %NURUS_PYTHON%" in script
    assert "%NURUS_PYTHON% -m venv .venv-csmp" in script
    assert "else (\n  call :find_python" not in script
    assert "if not defined NURUS_PYTHON call :try_python py -3.14" in script


def test_installer_uses_regular_package_and_preserves_exclamation_paths():
    root = Path(__file__).parents[1]
    script = (root / "Instalar_CSMP.bat").read_text(encoding="utf-8")
    assert ' -e "' not in script
    assert "DisableDelayedExpansion" in script
    assert "DisableDelayedExpansion" in (root / "Abrir_CSMP.bat").read_text(encoding="utf-8")


@pytest.mark.skipif(sys.platform != "win32", reason="Ejecuta cmd.exe real en el CI Windows")
def test_windows_cmd_executes_detected_python_after_subroutine(tmp_path):
    script = (Path(__file__).parents[1] / "Instalar_CSMP.bat").read_text(encoding="utf-8")
    subroutine = script[script.index("\n:find_python\n"):script.index("\n:wrong_venv\n")]
    probe = tmp_path / "detectar python!.bat"
    probe.write_text(
        '@echo off\nsetlocal EnableExtensions DisableDelayedExpansion\n'
        'call :find_python\nif not defined NURUS_PYTHON exit /b 5\n'
        '%NURUS_PYTHON% -m sysconfig\nexit /b %errorlevel%\n' + subroutine,
        encoding="utf-8",
    )
    result = subprocess.run(
        ["cmd.exe", "/d", "/c", str(probe)], capture_output=True, text=True,
        env={**os.environ, "NURUS_PYTHON_EXE": sys.executable}, timeout=30,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert "Python version" in result.stdout
