# Comprobación reusable sobre el instalador del build actual o una entrega elegida.
. (Join-Path $PSScriptRoot 'windows_process_check.ps1')

function Wait-ApplicationRemoval {
    param([Parameter(Mandatory=$true)][string]$Directory,
          [ValidateRange(1,180)][int]$TimeoutSeconds = 60)
    $elapsed = [Diagnostics.Stopwatch]::StartNew()
    while (Test-Path -LiteralPath $Directory) {
        if ($elapsed.Elapsed.TotalSeconds -ge $TimeoutSeconds) {
            throw 'La carpeta instalada sigue presente tras desinstalar.'
        }
        Start-Sleep -Milliseconds 200
    }
    Write-Host 'APROBADO: eliminación real de la carpeta instalada'
}

function Invoke-InstallerVerification {
    param([Parameter(Mandatory=$true)][string]$Installer,
          [Parameter(Mandatory=$true)][ValidatePattern('^[0-9a-f]{40}$')][string]$ExpectedSourceCommit,
          [Parameter(Mandatory=$true)][string]$Destination,
          [Parameter(Mandatory=$true)][string]$EvidenceDirectory)
    $ErrorActionPreference = 'Stop'
    if (Test-Path -LiteralPath $Destination) { throw 'La carpeta de prueba debe ser nueva.' }
    $installerPath = (Resolve-Path -LiteralPath $Installer).Path
    New-Item -ItemType Directory -Force $EvidenceDirectory | Out-Null
    $evidence = (Resolve-Path -LiteralPath $EvidenceDirectory).Path
    $arguments = @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',('/DIR="'+$Destination+'"'))
    Invoke-CheckedProcess -FilePath $installerPath -Arguments ($arguments + ('/LOG="'+(Join-Path $evidence 'installer-install.log')+'"')) -Label 'instalación del build actual'
    $exe = Join-Path $Destination 'CSMP_Integral.exe'
    if (-not (Test-Path -LiteralPath $exe)) { throw 'La instalación no creó el ejecutable.' }
    if (-not (Test-Path -LiteralPath (Join-Path $Destination 'SITFA_Descargador/SITFA_Descargador.exe'))) { throw 'La instalación no incluyó el Descargador.' }
    if (-not (Test-Path -LiteralPath (Join-Path $Destination 'extension/manifest.json'))) { throw 'La instalación no incluyó la extensión.' }
    $check = Join-Path $evidence 'installed-package-check.json'
    Invoke-CheckedProcess -FilePath $exe -Arguments @('--verificar-paquete',('"'+$check+'"')) -WorkingDirectory $evidence -Label 'productos desde EXE instalado'
    $proof = Get-Content -LiteralPath $check -Raw | ConvertFrom-Json
    if (-not $proof.ok -or -not $proof.frozen -or -not $proof.capture_pipeline_verified -or $proof.source_commit -ne $ExpectedSourceCommit) {
        throw 'El ejecutable instalado no acredita productos y SHA esperado.'
    }
    $exeHash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
    $data = Join-Path $env:LOCALAPPDATA 'CSMP_Personal_Prototipo_Integral'
    New-Item -ItemType Directory -Force $data | Out-Null
    $sentinel = Join-Path $data ('installer-test-'+[guid]::NewGuid().ToString()+'.txt')
    Set-Content -LiteralPath $sentinel 'dato ficticio que se conserva'
    try {
        Invoke-CheckedProcess -FilePath $installerPath -Arguments ($arguments + ('/LOG="'+(Join-Path $evidence 'installer-reinstall.log')+'"')) -Label 'reinstalación del build actual'
        if ((Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant() -ne $exeHash) { throw 'Reinstalar cambió el ejecutable.' }
        Invoke-CheckedProcess -FilePath (Join-Path $Destination 'unins000.exe') -Arguments @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',('/LOG="'+(Join-Path $evidence 'installer-uninstall.log')+'"')) -Label 'lanzador de desinstalación'
        Wait-ApplicationRemoval -Directory $Destination
        if (-not (Test-Path -LiteralPath $sentinel) -or (Get-Content -LiteralPath $sentinel -Raw).Trim() -ne 'dato ficticio que se conserva') {
            throw 'La desinstalación alteró los datos externos.'
        }
        @{ok=$true; data_kind='SINTETICA'; binary_source_commit=$ExpectedSourceCommit;
          installed_exe_sha256=$exeHash; installation_verified=$true; installed_pipeline_verified=$true;
          reinstall_verified=$true; installed_directory_removed=$true; external_data_preserved=$true;
          institutional_reads=0; institutional_writes=0} | ConvertTo-Json -Depth 5 |
            Set-Content -LiteralPath (Join-Path $evidence 'delivery-verification.json') -Encoding utf8
        Write-Host 'APROBADO: instalación, SHA, productos, eliminación y datos externos'
    } finally {
        # Únicamente el archivo ficticio creado por esta verificación.
        Remove-Item -LiteralPath $sentinel -ErrorAction SilentlyContinue
    }
}
