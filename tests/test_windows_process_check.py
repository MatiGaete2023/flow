"""Verifica salida real y terminación del árbol de un proceso de prueba colgado."""
from pathlib import Path
import shutil
import subprocess
import sys

import pytest

pytestmark=pytest.mark.skipif(sys.platform!='win32',reason='Control de procesos Windows.')


def quote(value):return "'"+str(value).replace("'","''")+"'"


def run(tmp_path,body,tail=''):
    shell=shutil.which('pwsh') or shutil.which('powershell')
    assert shell,'Las pruebas Windows requieren PowerShell.'
    child=tmp_path/'child.py';child.write_text(body,encoding='utf-8')
    helper=Path(__file__).parents[1]/'tools/windows_process_check.ps1'
    script=tmp_path/'run.ps1'
    script.write_text("$ErrorActionPreference = 'Stop'\n. "+quote(helper)+"\n"+
        'Invoke-CheckedProcess -FilePath '+quote(sys.executable)+' -Arguments @('+quote(child)+
        ") -Label 'fixture-process' -TimeoutSeconds 3\n"+tail,encoding='utf-8')
    return subprocess.run([shell,'-NoProfile','-File',str(script)],capture_output=True,
                          text=True,encoding='utf-8',errors='replace',timeout=25)


def test_successful_process_is_verified(tmp_path):
    result=run(tmp_path,'raise SystemExit(0)')
    assert result.returncode==0,result.stderr
    assert 'APROBADO: fixture-process' in result.stdout


def test_nonzero_exit_is_not_accepted(tmp_path):
    result=run(tmp_path,'raise SystemExit(3)')
    assert result.returncode!=0
    assert 'APROBADO:' not in result.stdout


def test_timed_out_process_is_killed(tmp_path):
    pid=tmp_path/'pid.txt'
    code="import os,time\nfrom pathlib import Path\nPath("+repr(str(pid))+ ").write_text(str(os.getpid()))\ntime.sleep(60)"
    result=run(tmp_path,code)
    assert result.returncode!=0 and 'APROBADO:' not in result.stdout
    assert pid.exists(),'El proceso de prueba no llegó a arrancar.'
    # Comprobar que no quedó el proceso durmiendo después de vencer el plazo.
    shell=shutil.which('pwsh') or shutil.which('powershell')
    check=subprocess.run([shell,'-NoProfile','-Command',
        f"if (Get-Process -Id {int(pid.read_text())} -ErrorAction SilentlyContinue) {{ exit 1 }}"],
        capture_output=True,timeout=15)
    assert check.returncode==0,'La verificación dejó el proceso ficticio vivo.'
