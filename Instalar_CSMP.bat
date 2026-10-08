@echo off
setlocal EnableExtensions DisableDelayedExpansion
cd /d "%~dp0"

echo ========================================
echo CSMP Assistant - instalacion local aislada
echo ========================================

if not exist ".venv-csmp\Scripts\python.exe" goto :create_venv

".venv-csmp\Scripts\python.exe" -c "import sys; v=sys.version_info[:2]; raise SystemExit(0 if (3, 12) <= v <= (3, 14) else 1)" >nul 2>&1
if errorlevel 1 goto :wrong_venv
echo Entorno CSMP Assistant compatible existente detectado. No se requiere buscar otro Python.
goto :install

:create_venv
call :find_python
if not defined NURUS_PYTHON goto :no_python
echo Python compatible 3.12-3.14 detectado: %NURUS_PYTHON%
echo Creando entorno virtual .venv-csmp...
%NURUS_PYTHON% -m venv .venv-csmp
if errorlevel 1 goto :error

:install

echo Instalando CSMP Assistant y soporte Excel / Outlook...
set "NURUS_PIP_SOURCE="
if exist "paquetes\" (
  echo Usando paquetes institucionales locales desde .\paquetes
  set "NURUS_PIP_SOURCE=--no-index --find-links paquetes"
)
".venv-csmp\Scripts\python.exe" -m pip install %NURUS_PIP_SOURCE% ".[excel-legacy,excel-native,outlook]"
if errorlevel 1 goto :error

".venv-csmp\Scripts\python.exe" -m pip check
if errorlevel 1 goto :error

".venv-csmp\Scripts\python.exe" -c "import pandas, openpyxl, xlrd, docx, win32com.client, nurus; print('CSMP Assistant instalado correctamente en el entorno aislado.')"
if errorlevel 1 goto :error

echo.
echo Instalacion terminada. Usa Abrir_CSMP.bat para iniciar.
pause
exit /b 0

:find_python
set "NURUS_PYTHON="
if defined NURUS_PYTHON_EXE call :try_python "%NURUS_PYTHON_EXE%"
if not defined NURUS_PYTHON call :try_python py -3.14
if not defined NURUS_PYTHON call :try_python py -3.13
if not defined NURUS_PYTHON call :try_python py -3.12
if not defined NURUS_PYTHON if exist "%LOCALAPPDATA%\Programs\Python\Python314\python.exe" call :try_python "%LOCALAPPDATA%\Programs\Python\Python314\python.exe"
if not defined NURUS_PYTHON if exist "%LOCALAPPDATA%\Programs\Python\Python313\python.exe" call :try_python "%LOCALAPPDATA%\Programs\Python\Python313\python.exe"
if not defined NURUS_PYTHON if exist "%LOCALAPPDATA%\Programs\Python\Python312\python.exe" call :try_python "%LOCALAPPDATA%\Programs\Python\Python312\python.exe"
if not defined NURUS_PYTHON if exist "%ProgramFiles%\Python314\python.exe" call :try_python "%ProgramFiles%\Python314\python.exe"
if not defined NURUS_PYTHON if exist "%ProgramFiles%\Python313\python.exe" call :try_python "%ProgramFiles%\Python313\python.exe"
if not defined NURUS_PYTHON if exist "%ProgramFiles%\Python312\python.exe" call :try_python "%ProgramFiles%\Python312\python.exe"
if not defined NURUS_PYTHON call :try_python python
if not defined NURUS_PYTHON call :try_python python3.14
if not defined NURUS_PYTHON call :try_python python3.13
if not defined NURUS_PYTHON call :try_python python3.12
exit /b 0

:try_python
%* -c "import sys; v=sys.version_info[:2]; raise SystemExit(0 if (3, 12) <= v <= (3, 14) else 1)" >nul 2>&1
if not errorlevel 1 set "NURUS_PYTHON=%*"
exit /b 0

:wrong_venv
echo.
echo ERROR: existe .venv-csmp, pero no usa una version soportada de Python 3.12 a 3.14.
echo Renombra o elimina esa carpeta solo si no contiene trabajo pendiente y vuelve a ejecutar el instalador.
pause
exit /b 1

:no_python
echo.
echo ERROR: no se encontro un ejecutable compatible de Python 3.12, 3.13 o 3.14.
echo Se probo NURUS_PYTHON_EXE, el launcher py, rutas estandar y los comandos python/python3.x.
echo Si ya conoces la ruta, ejecuta antes: set "NURUS_PYTHON_EXE=C:\ruta\python.exe"
echo Instala una version compatible para tu usuario o solicita soporte institucional antes de continuar.
pause
exit /b 1

:error
echo.
echo ERROR: la instalacion no termino correctamente.
echo Revisa el mensaje anterior. No se modifican politicas de Windows ni controles institucionales.
pause
exit /b 1
