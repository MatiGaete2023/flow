"""Postcondición nativa: retorno del lanzador no prueba eliminación de archivos."""
from pathlib import Path
import shutil
import subprocess
import sys
import pytest

pytestmark=pytest.mark.skipif(sys.platform!='win32',reason='PowerShell Windows real.')


def quote(value):return "'"+str(value).replace("'","''")+"'"


def run(script):
    shell=shutil.which('pwsh') or shutil.which('powershell')
    return subprocess.run([shell,'-NoProfile','-File',str(script)],capture_output=True,
                          text=True,encoding='utf-8',errors='replace',timeout=20)


def test_launcher_return_with_remaining_folder_is_failure(tmp_path):
    folder=tmp_path/'app';folder.mkdir();(folder/'app.txt').write_text('ficticio')
    script=tmp_path/'probe.ps1'
    script.write_text("$ErrorActionPreference='Stop'\n. "+quote(Path(__file__).parents[1]/'tools/verify_installer.ps1')+
                      '\nWait-ApplicationRemoval -Directory '+quote(folder)+' -TimeoutSeconds 1\n',encoding='utf-8')
    result=run(script)
    assert result.returncode!=0 and folder.exists()
    assert 'APROBADO:' not in result.stdout


def test_delegated_removal_must_finish_before_success(tmp_path):
    folder=tmp_path/'app';folder.mkdir();(folder/'app.txt').write_text('ficticio')
    child=tmp_path/'remove.ps1';child.write_text('Start-Sleep -Milliseconds 700\nRemove-Item -Recurse -LiteralPath '+quote(folder),encoding='utf-8')
    shell=shutil.which('pwsh') or shutil.which('powershell')
    script=tmp_path/'probe.ps1'
    script.write_text("$ErrorActionPreference='Stop'\n. "+quote(Path(__file__).parents[1]/'tools/verify_installer.ps1')+
        '\nStart-Process -FilePath '+quote(shell)+' -ArgumentList @('+quote('-NoProfile')+','+quote('-File')+','+quote('"'+str(child)+'"')+') | Out-Null\n'+
        'Wait-ApplicationRemoval -Directory '+quote(folder)+' -TimeoutSeconds 10\n',encoding='utf-8')
    result=run(script)
    assert result.returncode==0,result.stdout+result.stderr
    assert not folder.exists() and 'APROBADO:' in result.stdout
