param(
    [string]$Destination = $PSScriptRoot,
    [string]$StartMenuDir = (Join-Path ([Environment]::GetFolderPath('Programs')) ''),
    [string]$DesktopDir = ([Environment]::GetFolderPath('Desktop')),
    [string]$UninstallRegistryKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\FlowRust',
    [switch]$SkipRegistry,
    [switch]$Silent
)

$ErrorActionPreference = 'Stop'

if (-not $Destination) {
    throw 'Se requiere la carpeta de instalacion (-Destination).'
}

$destFull = [IO.Path]::GetFullPath($Destination)
$localApp = [IO.Path]::GetFullPath($env:LOCALAPPDATA)
$roamingApp = [IO.Path]::GetFullPath($env:APPDATA)
$flowDataDir = [IO.Path]::GetFullPath((Join-Path $localApp 'FlowRust'))
$flowRoamingDir = [IO.Path]::GetFullPath((Join-Path $roamingApp 'FlowRust'))

# Guardia de seguridad: nunca tocar datos locales ni carpeta padre del sistema.
if ($destFull -eq $localApp -or $destFull -eq $roamingApp -or $destFull -eq $flowDataDir -or $destFull -eq $flowRoamingDir) {
    throw "Ruta de desinstalacion rechazada por seguridad (protege datos locales): $destFull"
}
if ($destFull.Length -lt 10) {
    throw "Ruta de desinstalacion demasiado corta: $destFull"
}

if (Test-Path -LiteralPath $destFull) {
    $hasProgram = (Test-Path -LiteralPath (Join-Path $destFull 'flow_rust.exe')) -or
                  (Test-Path -LiteralPath (Join-Path $destFull 'Desinstalar_Flow.ps1'))
    if (-not $hasProgram) {
        throw "La carpeta indicada no corresponde a una instalacion de Flow Rust: $destFull"
    }

    $destPrefix = $destFull.TrimEnd('\', '/') + '\'
    $running = Get-Process -Name 'flow_rust' -ErrorAction SilentlyContinue | Where-Object {
        try {
            $_.Path -and $_.Path.StartsWith($destPrefix, [StringComparison]::OrdinalIgnoreCase)
        } catch {
            $false
        }
    }
    if ($running) {
        throw "Cierra Flow Rust antes de desinstalarlo desde: $destFull"
    }

    # Si PowerShell esta corriendo con cwd dentro de $destFull, salir antes de borrar.
    $cwd = [IO.Path]::GetFullPath((Get-Location).Path)
    if ($cwd.Equals($destFull, [StringComparison]::OrdinalIgnoreCase) -or
        $cwd.StartsWith($destPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        Set-Location -LiteralPath (Split-Path -Parent $destFull)
    }

    Remove-Item -LiteralPath $destFull -Recurse -Force
}

if ($StartMenuDir) {
    $lnk = Join-Path ([IO.Path]::GetFullPath($StartMenuDir)) 'Flow.lnk'
    if (Test-Path -LiteralPath $lnk) {
        Remove-Item -LiteralPath $lnk -Force
    }
}

if ($DesktopDir) {
    $deskLnk = Join-Path ([IO.Path]::GetFullPath($DesktopDir)) 'Flow.lnk'
    if (Test-Path -LiteralPath $deskLnk) {
        Remove-Item -LiteralPath $deskLnk -Force
    }
}

if (-not $SkipRegistry -and $UninstallRegistryKey -and (Test-Path -LiteralPath $UninstallRegistryKey)) {
    Remove-Item -LiteralPath $UninstallRegistryKey -Recurse -Force
}

if (-not $Silent) {
    Write-Host "Flow Rust desinstalado de: $destFull"
    Write-Host "Las actualizaciones y datos del usuario (%LOCALAPPDATA%\FlowRust y %APPDATA%\FlowRust) se conservaron."
}
