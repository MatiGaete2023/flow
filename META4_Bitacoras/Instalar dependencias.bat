@echo off
setlocal
cd /d "%~dp0"
where py >nul 2>&1
if errorlevel 1 goto python
py -3 -m venv .venv
if errorlevel 1 goto error
goto comprobar
:python
python -m venv .venv
if errorlevel 1 goto error
:comprobar
".venv\Scripts\python.exe" -c "import sys, tkinter; sys.exit(0 if sys.version_info >= (3, 10) else 1)"
if errorlevel 1 goto version
".venv\Scripts\python.exe" -m pip install -r requirements.txt
if errorlevel 1 goto error
echo.
echo Instalacion terminada. Abre "Iniciar Meta 4.bat".
pause
exit /b 0
:version
echo.
echo Se requiere Python 3.10 o superior con tkinter. Instala Python desde python.org y vuelve a ejecutar.
rmdir /s /q .venv
pause
exit /b 1
:error
echo.
echo No se pudo instalar. Se requiere Python 3.10 o superior con tkinter y acceso a Internet.
pause
exit /b 1
