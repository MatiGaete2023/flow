@echo off
rem Abre el Anonimizador CSMP (requiere Python 3.12 o superior con: pip install -r requirements.txt)
cd /d "%~dp0"
py -3 -m anonimizador_csmp %*
