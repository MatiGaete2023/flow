@echo off
setlocal EnableExtensions DisableDelayedExpansion
rem Acceso anterior: el producto vigente es CSMP Assistant.
call "%~dp0Instalar_CSMP.bat"
exit /b %errorlevel%
