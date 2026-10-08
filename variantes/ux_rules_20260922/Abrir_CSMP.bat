@echo off
setlocal EnableExtensions DisableDelayedExpansion
cd /d "%~dp0"
if not exist ".venv-csmp\Scripts\python.exe" (
  echo Ejecuta primero Instalar_CSMP.bat en esta carpeta.
  pause
  exit /b 1
)
".venv-csmp\Scripts\python.exe" -m nurus.personal.app
if errorlevel 1 (
  echo No se pudo abrir CSMP Assistant. Conserva el mensaje anterior.
  pause
  exit /b 1
)
