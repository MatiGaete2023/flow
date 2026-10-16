@echo off
setlocal
cd /d "%~dp0"
where py >nul 2>&1
if errorlevel 1 goto python
py -3 -m venv .venv
if errorlevel 1 goto error
goto instalar
:python
python -m venv .venv
if errorlevel 1 goto error
:instalar
".venv\Scripts\python.exe" -m pip install -r requirements.txt
if errorlevel 1 goto error
echo.
echo Instalacion terminada. Abre "Iniciar Meta 4.bat".
pause
exit /b 0
:error
echo.
echo No se pudo instalar. Se requiere Python 3.10 o superior con tkinter y acceso a Internet.
pause
exit /b 1
