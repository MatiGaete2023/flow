param(
    [string]$Python = 'python',
    [string]$Downloader = (Join-Path $PSScriptRoot '../descargador'),
    [string]$Output = (Join-Path $PSScriptRoot '../dist/integral')
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'windows_process_check.ps1')
$csmpRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$downloaderRoot = (Resolve-Path $Downloader).Path
$outputRoot = [IO.Path]::GetFullPath($Output)
New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null
function CheckedPython([string[]]$Arguments) {
    & $Python @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Falló Python: $($Arguments -join ' ')" }
}
Push-Location $csmpRoot
try {
    CheckedPython @('tools/write_build_meta.py')
    CheckedPython @('tools/write_build_meta.py','--descargador',$downloaderRoot)
    $minor = (& $Python -c "import sys; print(str(sys.version_info.major)+str(sys.version_info.minor))").Trim()
    if ($LASTEXITCODE -ne 0) { throw 'No se pudo identificar Python del build' }
    CheckedPython @('tools/flow_dependency_inventory.py','--lock',"requirements/windows-py$minor.lock",'--output',(Join-Path $outputRoot 'DEPENDENCIAS.json'))
    Copy-Item -LiteralPath (Join-Path $csmpRoot "requirements/windows-py$minor.lock") -Destination (Join-Path $outputRoot 'DEPENDENCIAS.lock') -Force
    $sourceCommit = (git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'No se pudo identificar el SHA de la plantilla de aceptación' }
    $acta = Join-Path $outputRoot 'ACTA_INSTITUCIONAL_NO_EJECUTADA.json'
    if (Test-Path $acta) {
        CheckedPython @('tools/flow_acceptance.py','validate',$acta)
        $previousActa = Get-Content $acta -Raw | ConvertFrom-Json
        if ($previousActa.artifact.source_commit -ne $sourceCommit) { throw 'El acta existente corresponde a otro SHA; use una carpeta nueva' }
        if (@($previousActa.records | Where-Object { $_.result -ne 'NO_EJECUTADO' -or $_.evidence_kind -ne 'INSTITUCIONAL' }).Count -ne 0) {
            throw 'El acta existente contiene resultados; conserve ese archivo y use una carpeta nueva para distribuir'
        }
    } else {
        CheckedPython @('tools/flow_acceptance.py','init',$acta,'--commit',$sourceCommit,'--assistant-version','0.5.0.dev4','--downloader-version','2.7.5','--extension-version','2.7.5')
    }
    CheckedPython @('-m','PyInstaller','--clean','--noconfirm','--distpath',$outputRoot,'CSMP_Experimental.spec')
} finally { Pop-Location }
Push-Location $downloaderRoot
try {
    CheckedPython @('-m','PyInstaller','--clean','--noconfirm','--distpath',$outputRoot,'SITFA_Descargador.spec')
} finally { Pop-Location }
Copy-Item -LiteralPath (Join-Path $downloaderRoot 'extension') -Destination $outputRoot -Recurse -Force
Copy-Item -LiteralPath (Join-Path $csmpRoot 'docs/GUIA_ENTREGA_MVP_20261008.md') -Destination (Join-Path $outputRoot 'GUIA_INTEGRAL.md') -Force
foreach ($manual in @('MANUAL_DESCARGADOR_MVP_20261008.md','MANUAL_ASISTENTE_MVP_20261008.md','ACEPTACION_INSTITUCIONAL_PENDIENTE.md','REGRESIONES_PRUEBAS_USUARIO_20261009.md','CORRECCION_REFERENCIAS_ZIP_20261009.md')) {
    Copy-Item -LiteralPath (Join-Path $csmpRoot "docs/$manual") -Destination $outputRoot -Force
}
'@echo off', 'start "" "%~dp0CSMP_Integral\CSMP_Integral.exe"' | Set-Content -LiteralPath (Join-Path $outputRoot 'Abrir_CSMP.cmd') -Encoding ascii
'@echo off', 'start "" "%~dp0SITFA_Descargador\SITFA_Descargador.exe"' | Set-Content -LiteralPath (Join-Path $outputRoot 'Abrir_Descargador.cmd') -Encoding ascii
$checks = @(
    @('SITFA_Descargador/SITFA_Descargador.exe','--verificar-paquete','VERIFICACION_DESCARGADOR.json'),
    @('CSMP_Integral/CSMP_Integral.exe','--verificar-paquete','VERIFICACION_CSMP.json')
)
Push-Location $outputRoot
try {
    foreach ($check in $checks) {
        Invoke-CheckedProcess -FilePath (Join-Path $outputRoot $check[0]) -WorkingDirectory $outputRoot -Arguments $check[1..($check.Length-1)] -Label "verificación $($check[0]) $($check[1])"
        $proof = Get-Content -LiteralPath $check[2] -Raw | ConvertFrom-Json
        if (-not $proof.ok) { throw 'La distribución no confirmó su integración.' }
        if ($check[1] -eq '--verificar-paquete' -and $check[0].StartsWith('CSMP_Integral/') -and -not $proof.capture_pipeline_verified) {
            throw 'CSMP no confirmó los productos v1 desde el ejecutable autónomo.'
        }
    }
} finally { Pop-Location }
Write-Output "Paquete integral verificado: $outputRoot"
