@echo off
setlocal EnableExtensions DisableDelayedExpansion
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Instalar_Flow.ps1" -SourceDir "%~dp0" -RequireSha256
set "FLOW_INST_ERR=%errorlevel%"
if not "%FLOW_INST_ERR%"=="0" (
  echo.
  echo Error al instalar Flow Rust ^(codigo %FLOW_INST_ERR%^).
  pause
  exit /b %FLOW_INST_ERR%
)
echo.
echo Instalacion completada. Puedes abrir "Flow" desde el menu Inicio.
pause
