param([string]$Python = 'python')
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Push-Location $root
try {
    & $Python -m PyInstaller --noconfirm --clean --onefile --windowed --name AnonimizadorCSMP --paths src --paths tools --paths portable --hidden-import xlrd --exclude-module pandas --exclude-module pytest --exclude-module pip --exclude-module setuptools --distpath dist/anonimizador --workpath build/anonimizador --specpath portable portable/launcher.py
    if ($LASTEXITCODE -ne 0) { throw 'Fallo de compilacion' }
    $report = Join-Path $root ('build/autoprueba-' + [guid]::NewGuid().ToString('N') + '.json')
    $process = Start-Process -FilePath (Join-Path $root 'dist/anonimizador/AnonimizadorCSMP.exe') -ArgumentList @('--self-test', ('"' + $report + '"')) -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw 'Fallo de autoprueba del EXE' }
    Copy-Item -LiteralPath $report -Destination 'dist/anonimizador/AUTOPRUEBA.json'
    Get-FileHash 'dist/anonimizador/AnonimizadorCSMP.exe' -Algorithm SHA256 | Format-List
} finally { Pop-Location }
