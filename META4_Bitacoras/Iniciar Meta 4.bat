@echo off
setlocal
cd /d "%~dp0"
if not exist ".venv\Scripts\python.exe" goto falta
".venv\Scripts\python.exe" Meta4.py
if errorlevel 1 pause
exit /b
:falta
echo Ejecuta primero "Instalar dependencias.bat".
pause
exit /b 1
