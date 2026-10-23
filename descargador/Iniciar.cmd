@echo off
setlocal
cd /d "%~dp0"
if exist "dist\SITFA_Descargador\SITFA_Descargador.exe" (
    start "" "dist\SITFA_Descargador\SITFA_Descargador.exe"
    exit /b 0
)
if not exist ".venv\Scripts\pythonw.exe" (
    echo Preparando el entorno local. No se requieren permisos de administrador.
    rem Python 3.12 primero: es la version con la que CI prueba el lock con hashes.
    rem Luego 3.13 y 3.14: son las otras versiones con lock. py -3 queda al final.
    py -3.12 -m venv .venv >nul 2>nul
    if errorlevel 1 py -3.13 -m venv .venv >nul 2>nul
    if errorlevel 1 py -3.14 -m venv .venv >nul 2>nul
    if errorlevel 1 py -3 -m venv .venv >nul 2>nul
    if errorlevel 1 python -m venv .venv
    if errorlevel 1 goto error
)
".venv\Scripts\python.exe" "%~dp0preparar_entorno.py"
rem Codigo 3: esta version de Python no tiene lock; se borra el entorno para no reutilizarlo.
if errorlevel 3 goto nolock
if errorlevel 1 goto error
start "" ".venv\Scripts\pythonw.exe" "%~dp0descargador.py"
exit /b 0
:nolock
rmdir /s /q ".venv" >nul 2>nul
echo Esta version de Python no es compatible. Instale Python 3.12 (recomendado), 3.13 o 3.14 y vuelva a abrir.
pause
exit /b 1
:error
echo No se pudo preparar el entorno. Compruebe Python y la conexion para instalar paquetes.
echo Puede consultar las instrucciones del archivo README.md.
pause
exit /b 1
