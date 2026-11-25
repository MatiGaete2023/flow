param(
    [string]$SourceDir = $PSScriptRoot,
    [string]$Destination = (Join-Path $env:LOCALAPPDATA 'Programs\FlowRust'),
    [string]$DataBaseDir = (Join-Path $env:LOCALAPPDATA 'FlowRust'),
    [string]$StartMenuDir = (Join-Path ([Environment]::GetFolderPath('Programs')) ''),
    [string]$DesktopDir = ([Environment]::GetFolderPath('Desktop')),
    [switch]$CreateDesktopShortcut,
    [string]$UninstallRegistryKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\FlowRust',
    [switch]$SkipRegistry,
    [string]$Version = '1.0.0',
    [switch]$RequireSha256,
    [switch]$Silent
)

$ErrorActionPreference = 'Stop'

function Test-MzHeader {
    param([Parameter(Mandatory = $true)][string]$Path)
    $bytes = New-Object byte[] 2
    $fs = [IO.File]::OpenRead($Path)
    try {
        $n = $fs.Read($bytes, 0, 2)
    } finally {
        $fs.Dispose()
    }
    return ($n -eq 2 -and $bytes[0] -eq 0x4D -and $bytes[1] -eq 0x5A)
}

function Get-Sha256Hex {
    param([Parameter(Mandatory = $true)][string]$Path)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $fs = [IO.File]::OpenRead($Path)
        try {
            $hashBytes = $sha.ComputeHash($fs)
        } finally {
            $fs.Dispose()
        }
    } finally {
        $sha.Dispose()
    }
    return ([BitConverter]::ToString($hashBytes) -replace '-', '').ToLowerInvariant()
}

function Read-ExpectedSha256 {
    param([Parameter(Mandatory = $true)][string]$ShaPath)
    $lines = Get-Content -LiteralPath $ShaPath -Encoding UTF8
    $fallback = $null
    foreach ($line in $lines) {
        if ($line -match '^\s*([0-9a-fA-F]{64})\b(.*)$') {
            $hash = $Matches[1].ToLowerInvariant()
            $rest = $Matches[2]
            if ($rest -match 'flow_rust\.exe') {
                return $hash
            }
            if (-not $fallback) {
                $fallback = $hash
            }
        }
    }
    if ($fallback) { return $fallback }
    throw "SHA256.txt no contiene una huella SHA-256 valida: $ShaPath"
}

function New-ShortcutFile {
    param(
        [Parameter(Mandatory = $true)][string]$ShortcutPath,
        [Parameter(Mandatory = $true)][string]$TargetPath,
        [Parameter(Mandatory = $true)][string]$WorkingDirectory
    )
    $parent = Split-Path -Parent $ShortcutPath
    if ($parent) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    $wsh = New-Object -ComObject WScript.Shell
    try {
        $lnk = $wsh.CreateShortcut($ShortcutPath)
        $lnk.TargetPath = $TargetPath
        $lnk.WorkingDirectory = $WorkingDirectory
        $lnk.IconLocation = "$TargetPath,0"
        $lnk.Description = 'Flow Rust - Asistente CSMP y Descargador'
        $lnk.Save()
    } finally {
        [void][Runtime.InteropServices.Marshal]::ReleaseComObject($wsh)
    }
}

$sourceFull = [IO.Path]::GetFullPath($SourceDir)
$destFull = [IO.Path]::GetFullPath($Destination)
$dataFull = [IO.Path]::GetFullPath($DataBaseDir)
$exeSource = Join-Path $sourceFull 'flow_rust.exe'
$shaSource = Join-Path $sourceFull 'SHA256.txt'
$uninstallScriptSource = Join-Path $PSScriptRoot 'Desinstalar_Flow.ps1'

if (-not (Test-Path -LiteralPath $exeSource -PathType Leaf)) {
    throw "No se encontro flow_rust.exe junto al instalador: $exeSource"
}
if (-not (Test-MzHeader -Path $exeSource)) {
    throw "El archivo flow_rust.exe no es un ejecutable Windows valido."
}

$actualHash = Get-Sha256Hex -Path $exeSource
$hasShaFile = Test-Path -LiteralPath $shaSource -PathType Leaf
if ($RequireSha256 -and -not $hasShaFile) {
    throw "Falta SHA256.txt junto a flow_rust.exe en: $sourceFull"
}
if ($hasShaFile) {
    $expectedHash = Read-ExpectedSha256 -ShaPath $shaSource
    if ($expectedHash -ne $actualHash) {
        throw "La huella SHA-256 de flow_rust.exe no coincide con SHA256.txt (esperada: $expectedHash, actual: $actualHash)."
    }
}

# Verificar que Flow no este ejecutandose desde la carpeta destino antes de reemplazar.
$destPrefix = $destFull.TrimEnd('\', '/') + '\'
$running = Get-Process -Name 'flow_rust' -ErrorAction SilentlyContinue | Where-Object {
    try {
        $_.Path -and $_.Path.StartsWith($destPrefix, [StringComparison]::OrdinalIgnoreCase)
    } catch {
        $false
    }
}
if ($running) {
    throw "Cierra Flow Rust antes de instalar o actualizar en: $destFull"
}

# Instalar por usuario sin requerir administrador y respetando %LOCALAPPDATA%\FlowRust\actualizaciones.
New-Item -ItemType Directory -Force -Path $destFull | Out-Null
$updatesDir = Join-Path $dataFull 'actualizaciones'
New-Item -ItemType Directory -Force -Path $updatesDir | Out-Null

$exeDest = Join-Path $destFull 'flow_rust.exe'
Copy-Item -LiteralPath $exeSource -Destination $exeDest -Force

if ($hasShaFile) {
    Copy-Item -LiteralPath $shaSource -Destination (Join-Path $destFull 'SHA256.txt') -Force
}

$uninstallDest = Join-Path $destFull 'Desinstalar_Flow.ps1'
if (Test-Path -LiteralPath $uninstallScriptSource -PathType Leaf) {
    Copy-Item -LiteralPath $uninstallScriptSource -Destination $uninstallDest -Force
}

$startMenuShortcut = $null
if ($StartMenuDir) {
    $startMenuShortcut = Join-Path ([IO.Path]::GetFullPath($StartMenuDir)) 'Flow.lnk'
    New-ShortcutFile -ShortcutPath $startMenuShortcut -TargetPath $exeDest -WorkingDirectory $destFull
}

$desktopShortcut = $null
if ($CreateDesktopShortcut -and $DesktopDir) {
    $desktopShortcut = Join-Path ([IO.Path]::GetFullPath($DesktopDir)) 'Flow.lnk'
    New-ShortcutFile -ShortcutPath $desktopShortcut -TargetPath $exeDest -WorkingDirectory $destFull
}

if (-not $SkipRegistry -and $UninstallRegistryKey) {
    New-Item -Path $UninstallRegistryKey -Force | Out-Null
    $uninstCmd = if (Test-Path -LiteralPath $uninstallDest -PathType Leaf) {
        "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$uninstallDest`" -Destination `"$destFull`""
    } else {
        "powershell.exe -NoProfile -Command `"Remove-Item -LiteralPath '$destFull' -Recurse -Force`""
    }
    $quietCmd = if (Test-Path -LiteralPath $uninstallDest -PathType Leaf) {
        "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$uninstallDest`" -Destination `"$destFull`" -Silent"
    } else {
        $uninstCmd
    }
    New-ItemProperty -Path $UninstallRegistryKey -Name 'DisplayName' -Value 'Flow Rust' -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallRegistryKey -Name 'DisplayVersion' -Value $Version -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallRegistryKey -Name 'Publisher' -Value 'NuRus' -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallRegistryKey -Name 'InstallLocation' -Value $destFull -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallRegistryKey -Name 'DisplayIcon' -Value "$exeDest,0" -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallRegistryKey -Name 'UninstallString' -Value $uninstCmd -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallRegistryKey -Name 'QuietUninstallString' -Value $quietCmd -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallRegistryKey -Name 'NoModify' -Value 1 -PropertyType DWord -Force | Out-Null
    New-ItemProperty -Path $UninstallRegistryKey -Name 'NoRepair' -Value 1 -PropertyType DWord -Force | Out-Null
}

if (-not $Silent) {
    Write-Host "Flow Rust instalado para el usuario actual en: $destFull"
    Write-Host "SHA-256 verificado: $actualHash"
    Write-Host "Carpeta de actualizaciones (E-I2) preservada en: $updatesDir"
    if ($startMenuShortcut) {
        Write-Host "Acceso directo en menu Inicio: $startMenuShortcut"
    }
}
