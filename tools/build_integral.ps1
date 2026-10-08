param(
    [string]$Python = 'python',
    [string]$Downloader = (Join-Path $PSScriptRoot '../descargador'),
    [string]$Output = (Join-Path $PSScriptRoot '../dist/integral')
)
$ErrorActionPreference = 'Stop'
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
    CheckedPython @('-m','PyInstaller','--clean','--noconfirm','--distpath',$outputRoot,'CSMP_Experimental.spec')
} finally { Pop-Location }
Push-Location $downloaderRoot
try {
    CheckedPython @('-m','PyInstaller','--clean','--noconfirm','--distpath',$outputRoot,'SITFA_Descargador.spec')
} finally { Pop-Location }
Copy-Item -LiteralPath (Join-Path $downloaderRoot 'extension') -Destination $outputRoot -Recurse -Force
Copy-Item -LiteralPath (Join-Path $csmpRoot 'docs/INTEGRACION_FINAL_20261008.md') -Destination (Join-Path $outputRoot 'GUIA_INTEGRAL.md') -Force
'@echo off', 'start "" "%~dp0CSMP_Integral\CSMP_Integral.exe"' | Set-Content -LiteralPath (Join-Path $outputRoot 'Abrir_CSMP.cmd') -Encoding ascii
'@echo off', 'start "" "%~dp0SITFA_Descargador\SITFA_Descargador.exe"' | Set-Content -LiteralPath (Join-Path $outputRoot 'Abrir_Descargador.cmd') -Encoding ascii
$checks = @(
    @('SITFA_Descargador/SITFA_Descargador.exe','--verificar-paquete','VERIFICACION_DESCARGADOR.json'),
    @('CSMP_Integral/CSMP_Integral.exe','--verificar-paquete','VERIFICACION_CSMP.json'),
    @('CSMP_Integral/CSMP_Integral.exe','--verificar-integracion','VERIFICACION_INTEGRACION.json','--descargador-integral','SITFA_Descargador/SITFA_Descargador.exe')
)
Push-Location $outputRoot
try {
    foreach ($check in $checks) {
        $process = Start-Process -FilePath (Join-Path $outputRoot $check[0]) -WorkingDirectory $outputRoot -ArgumentList $check[1..($check.Length-1)] -PassThru -Wait -WindowStyle Hidden
        if ($process.ExitCode -ne 0) { throw "Falló la verificación $($check[1])" }
        $proof = Get-Content -LiteralPath $check[2] -Raw | ConvertFrom-Json
        if (-not $proof.ok) { throw 'La distribución no confirmó su integración.' }
    }
} finally { Pop-Location }
Write-Output "Paquete integral verificado: $outputRoot"
