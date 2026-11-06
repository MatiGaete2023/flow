@echo off
setlocal
rem Arrastra un ZIP sobre este archivo, o: zip2exe.bat repo.zip [opciones]
if "%~1"=="" (
  echo Uso: zip2exe.bat repo.zip [--entry main.py] [--windowed] [--onedir] [--name App]
  pause
  exit /b 1
)
where py >nul 2>nul && (set "PY=py -3") || (set "PY=python")
%PY% "%~dp0zip2exe.py" %*
set "RC=%ERRORLEVEL%"
pause
exit /b %RC%
