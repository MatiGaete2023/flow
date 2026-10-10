@echo off
setlocal
cd /d "%~dp0"
if exist "dist\SITFA_Descargador\SITFA_Descargador.exe" (
    start "" "dist\SITFA_Descargador\SITFA_Descargador.exe"
    exit /b 0
)
if not exist ".venv\Scripts\pythonw.exe" (
    echo Preparando el entorno local. No se requieren permisos de administrador.
    where py >nul 2>nul
    if errorlevel 1 (
        python -m venv .venv
    ) else (
        py -3 -m venv .venv
    )
    if errorlevel 1 goto error
)
".venv\Scripts\python.exe" "%~dp0preparar_entorno.py"
if errorlevel 1 goto error
start "" ".venv\Scripts\pythonw.exe" "%~dp0descargador.py"
exit /b 0
:error
echo No se pudo preparar el entorno. Compruebe Python y la conexion para instalar paquetes.
echo Puede consultar las instrucciones del archivo README.md.
pause
exit /b 1
