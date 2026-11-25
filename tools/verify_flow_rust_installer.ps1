param(
    [Parameter(Mandatory = $true)][string]$BundleDir,
    [string]$InnoInstaller = '',
    [switch]$SkipSelfTest,
    [string]$EvidenceDirectory = ''
)

$ErrorActionPreference = 'Stop'

function Assert-FileEquals {
    param([string]$Path, [string]$Expected)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Falta archivo preservado esperado: $Path"
    }
    $actual = (Get-Content -LiteralPath $Path -Raw -Encoding UTF8).TrimEnd("`r", "`n")
    if ($actual -ne $Expected) {
        throw "Contenido alterado en $Path (esperado '$Expected', actual '$actual')"
    }
}

$bundleFull = (Resolve-Path -LiteralPath $BundleDir).Path
$installScript = Join-Path $bundleFull 'Instalar_Flow.ps1'
if (-not (Test-Path -LiteralPath $installScript -PathType Leaf)) {
    $installScript = (Resolve-Path (Join-Path $PSScriptRoot '..\installer\Instalar_Flow.ps1')).Path
}
$exeBundle = Join-Path $bundleFull 'flow_rust.exe'
$shaBundle = Join-Path $bundleFull 'SHA256.txt'
if (-not (Test-Path -LiteralPath $exeBundle -PathType Leaf)) {
    throw "No se encontro flow_rust.exe en el paquete: $bundleFull"
}
if (-not (Test-Path -LiteralPath $shaBundle -PathType Leaf)) {
    throw "No se encontro SHA256.txt en el paquete: $bundleFull"
}

$rootTmp = Join-Path ([IO.Path]::GetTempPath()) ('flow-rust-inst-check-' + [guid]::NewGuid().ToString('N'))
$tamperBundle = Join-Path $rootTmp 'tamper_bundle'
$tamperDest = Join-Path $rootTmp 'tamper_dest'
$dest = Join-Path $rootTmp 'programa\FlowRust'
$dataBase = Join-Path $rootTmp 'localappdata\FlowRust'
$startMenu = Join-Path $rootTmp 'menu'
$regKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\FlowRust_Prueba_" + ([guid]::NewGuid().ToString('N').Substring(0, 8))

try {
    New-Item -ItemType Directory -Force -Path $tamperBundle, $rootTmp | Out-Null

    # 1. Huella SHA-256 alterada debe rechazar la instalacion antes de copiar el programa.
    Copy-Item -LiteralPath $exeBundle -Destination (Join-Path $tamperBundle 'flow_rust.exe') -Force
    "$('0' * 64)  flow_rust.exe" | Set-Content -LiteralPath (Join-Path $tamperBundle 'SHA256.txt') -Encoding UTF8
    $tamperExit = 0
    try {
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $installScript `
            -SourceDir $tamperBundle -Destination $tamperDest -DataBaseDir (Join-Path $rootTmp 'tamper_data') `
            -StartMenuDir $startMenu -SkipRegistry -RequireSha256 -Silent 2>$null
        $tamperExit = $LASTEXITCODE
    } catch {
        $tamperExit = 1
    }
    if ($tamperExit -eq 0 -or (Test-Path -LiteralPath (Join-Path $tamperDest 'flow_rust.exe'))) {
        throw 'El instalador acepto un SHA256.txt manipulado.'
    }
    Write-Host 'APROBADO: rechazo de huella SHA-256 manipulada'

    # 2. Instalacion limpia por usuario.
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $installScript `
        -SourceDir $bundleFull -Destination $dest -DataBaseDir $dataBase `
        -StartMenuDir $startMenu -UninstallRegistryKey $regKey -RequireSha256 -Silent
    if ($LASTEXITCODE -ne 0) { throw "Fallo la instalacion limpia (exit $LASTEXITCODE)" }

    $installedExe = Join-Path $dest 'flow_rust.exe'
    $updatesDir = Join-Path $dataBase 'actualizaciones'
    $shortcutPath = Join-Path $startMenu 'Flow.lnk'
    foreach ($req in @($installedExe, (Join-Path $dest 'SHA256.txt'), (Join-Path $dest 'Desinstalar_Flow.ps1'), $updatesDir, $shortcutPath)) {
        if (-not (Test-Path -LiteralPath $req)) {
            throw "Falta artefacto tras instalar: $req"
        }
    }

    $origHash = (Get-FileHash -LiteralPath $exeBundle -Algorithm SHA256).Hash.ToLowerInvariant()
    $instHash = (Get-FileHash -LiteralPath $installedExe -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($origHash -ne $instHash) {
        throw 'La huella del ejecutable instalado difiere del origen.'
    }

    $wsh = New-Object -ComObject WScript.Shell
    try {
        $lnk = $wsh.CreateShortcut($shortcutPath)
        if ($lnk.TargetPath -ne $installedExe) {
            throw "TargetPath inesperado en Flow.lnk: $($lnk.TargetPath)"
        }
    } finally {
        [void][Runtime.InteropServices.Marshal]::ReleaseComObject($wsh)
    }

    $regProps = Get-ItemProperty -LiteralPath $regKey
    if ($regProps.DisplayName -ne 'Flow Rust' -or $regProps.InstallLocation -ne $dest) {
        throw 'Registro de desinstalacion HKCU incompleto.'
    }
    Write-Host 'APROBADO: instalacion por usuario, huella, acceso directo y registro HKCU'

    # 3. Ejecutar --self-test desde el ejecutable instalado.
    if (-not $SkipSelfTest) {
        $diagJson = if ($EvidenceDirectory) {
            New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
            Join-Path (Resolve-Path -LiteralPath $EvidenceDirectory).Path 'installed-self-test.json'
        } else {
            Join-Path $rootTmp 'installed-self-test.json'
        }
        $proc = Start-Process -FilePath $installedExe -ArgumentList @('--self-test', $diagJson) -Wait -PassThru -NoNewWindow
        if ($proc.ExitCode -ne 0) {
            throw "El ejecutable instalado fallo --self-test (exit $($proc.ExitCode))"
        }
        $proof = Get-Content -LiteralPath $diagJson -Raw -Encoding UTF8 | ConvertFrom-Json
        if (-not $proof.ok -or $proof.comprobaciones.Count -lt 1) {
            throw 'El informe --self-test del ejecutable instalado no reporto ok=true.'
        }
        Write-Host "APROBADO: --self-test desde ejecutable instalado ($($proof.comprobaciones.Count) comprobaciones)"
    }

    # 4. Reinstalacion y desinstalacion deben conservar actualizaciones/ (E-I2) y extensiones.
    $pendingExe = Join-Path $updatesDir 'flow_rust.exe'
    $pendingSha = Join-Path $updatesDir 'SHA256.txt'
    $appliedDir = Join-Path $updatesDir 'aplicadas'
    $appliedSentinel = Join-Path $appliedDir 'v0.txt'
    $extDir = Join-Path $dataBase 'extension-integral'
    $extManifest = Join-Path $extDir 'manifest.json'
    New-Item -ItemType Directory -Force -Path $appliedDir, $extDir | Out-Null
    'MZpendiente-sintetico' | Set-Content -LiteralPath $pendingExe -Encoding UTF8
    'abc  flow_rust.exe' | Set-Content -LiteralPath $pendingSha -Encoding UTF8
    'version-previa-aplicada' | Set-Content -LiteralPath $appliedSentinel -Encoding UTF8
    '{"name":"Sitfa Ficticia"}' | Set-Content -LiteralPath $extManifest -Encoding UTF8
    'MZanterior' | Set-Content -LiteralPath (Join-Path $dest 'flow_rust.exe.anterior') -Encoding UTF8

    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $installScript `
        -SourceDir $bundleFull -Destination $dest -DataBaseDir $dataBase `
        -StartMenuDir $startMenu -UninstallRegistryKey $regKey -RequireSha256 -Silent
    if ($LASTEXITCODE -ne 0) { throw "Fallo la reinstalacion (exit $LASTEXITCODE)" }

    Assert-FileEquals $pendingExe 'MZpendiente-sintetico'
    Assert-FileEquals $pendingSha 'abc  flow_rust.exe'
    Assert-FileEquals $appliedSentinel 'version-previa-aplicada'
    Assert-FileEquals $extManifest '{"name":"Sitfa Ficticia"}'
    Write-Host 'APROBADO: reinstalacion preserva actualizaciones (E-I2) y extension'

    # 5. Desinstalacion retira el programa, Flow.lnk y HKCU sin tocar actualizaciones ni extension.
    $uninst = Join-Path $dest 'Desinstalar_Flow.ps1'
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $uninst `
        -Destination $dest -StartMenuDir $startMenu -UninstallRegistryKey $regKey -Silent
    if ($LASTEXITCODE -ne 0) { throw "Fallo la desinstalacion (exit $LASTEXITCODE)" }

    if (Test-Path -LiteralPath $dest) { throw "La carpeta del programa sigue existiendo tras desinstalar: $dest" }
    if (Test-Path -LiteralPath $shortcutPath) { throw "El acceso directo Flow.lnk sigue existiendo tras desinstalar." }
    if (Test-Path -LiteralPath $regKey) { throw "La clave HKCU Uninstall sigue existiendo tras desinstalar." }

    Assert-FileEquals $pendingExe 'MZpendiente-sintetico'
    Assert-FileEquals $pendingSha 'abc  flow_rust.exe'
    Assert-FileEquals $appliedSentinel 'version-previa-aplicada'
    Assert-FileEquals $extManifest '{"name":"Sitfa Ficticia"}'
    Write-Host 'APROBADO: desinstalacion limpia conservando actualizaciones (E-I2) y extension'

    # 6. Si se entrega instalador Inno Setup (.exe), verificar instalacion silenciosa, --self-test y unins000.exe.
    if ($InnoInstaller) {
        $innoExe = (Resolve-Path -LiteralPath $InnoInstaller).Path
        $innoDest = Join-Path $rootTmp 'inno_dest\FlowRust'
        $realUpdates = Join-Path $env:LOCALAPPDATA 'FlowRust\actualizaciones'
        New-Item -ItemType Directory -Force -Path $realUpdates | Out-Null
        $innoSentinel = Join-Path $realUpdates ("prueba-ei1-" + [guid]::NewGuid().ToString('N') + '.txt')
        'centinela-ei1' | Set-Content -LiteralPath $innoSentinel -Encoding UTF8
        try {
            $procInst = Start-Process -FilePath $innoExe -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/DIR=`"$innoDest`"") -Wait -PassThru
            if ($procInst.ExitCode -ne 0) { throw "Fallo instalador Inno Setup (exit $($procInst.ExitCode))" }
            $innoProgram = Join-Path $innoDest 'flow_rust.exe'
            if (-not (Test-Path -LiteralPath $innoProgram -PathType Leaf)) {
                throw 'El instalador Inno Setup no copio flow_rust.exe'
            }
            if (-not $SkipSelfTest) {
                $innoDiag = Join-Path $rootTmp 'inno-self-test.json'
                $procDiag = Start-Process -FilePath $innoProgram -ArgumentList @('--self-test', $innoDiag) -Wait -PassThru -NoNewWindow
                if ($procDiag.ExitCode -ne 0) { throw 'Fallo --self-test desde instalacion Inno Setup' }
            }
            $uninsExe = Join-Path $innoDest 'unins000.exe'
            $procUninst = Start-Process -FilePath $uninsExe -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART') -Wait -PassThru
            if ($procUninst.ExitCode -ne 0) { throw "Fallo unins000.exe (exit $($procUninst.ExitCode))" }
            $sw = [Diagnostics.Stopwatch]::StartNew()
            while ((Test-Path -LiteralPath $innoDest) -and $sw.Elapsed.TotalSeconds -lt 30) {
                Start-Sleep -Milliseconds 200
            }
            if (Test-Path -LiteralPath $innoDest) {
                throw 'unins000.exe no elimino la carpeta del programa.'
            }
            Assert-FileEquals $innoSentinel 'centinela-ei1'
            Write-Host 'APROBADO: instalador Inno Setup (.exe), --self-test y preservacion de actualizaciones'
        } finally {
            Remove-Item -LiteralPath $innoSentinel -ErrorAction SilentlyContinue
        }
    }
} finally {
    if (Test-Path -LiteralPath $regKey) {
        Remove-Item -LiteralPath $regKey -Recurse -Force -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath $rootTmp -Recurse -Force -ErrorAction SilentlyContinue
}
