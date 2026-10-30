# Construye el instalador de SITFA Lite en Windows (PowerShell, desde la carpeta descargador_lite).
#   powershell -ExecutionPolicy Bypass -File build\build.ps1
# Requisitos: Python 3.12 x64 en el PATH (py -3.12) e Inno Setup 6 (choco install innosetup, o instalador oficial).
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)

$version = (Select-String -Path app\version.py -Pattern "^VERSION = '([0-9.]+)'").Matches[0].Groups[1].Value
if (-not $version) { throw 'No se encontró VERSION en app\version.py' }
$manifest = Get-Content extension\manifest.json -Raw | ConvertFrom-Json
if ($manifest.version -ne $version) { throw "La extensión declara $($manifest.version) y el programa $version" }

if (-not (Test-Path .venv-build)) { py -3.12 -m venv .venv-build }
$python = Resolve-Path .venv-build\Scripts\python.exe
& $python -m pip install --upgrade pip | Out-Null
& $python -m pip install -r requirements-build.txt
if ($LASTEXITCODE -ne 0) { throw 'Falló la instalación de dependencias' }

& $python -m pytest -q tests
if ($LASTEXITCODE -ne 0) { throw 'Fallaron las pruebas' }

& $python build\manuales.py dist\manuales
if ($LASTEXITCODE -ne 0) { throw 'Falló la generación de manuales' }

& $python -m PyInstaller --noconfirm --clean --distpath dist --workpath build\_work build\SITFA_Lite.spec
if ($LASTEXITCODE -ne 0) { throw 'Falló PyInstaller' }

$iscc = Get-ChildItem "${env:ProgramFiles(x86)}\Inno Setup*\ISCC.exe", "$env:ProgramFiles\Inno Setup*\ISCC.exe", "$env:LOCALAPPDATA\Programs\Inno Setup*\ISCC.exe" -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName
if (-not $iscc) { throw 'No se encuentra Inno Setup 6 (ISCC.exe)' }
& $iscc "/DAppVersion=$version" build\SITFA_Lite.iss
if ($LASTEXITCODE -ne 0) { throw 'Falló Inno Setup' }

$installer = Get-Item "dist\Instalar_SITFA_Lite_$version.exe"
$hash = (Get-FileHash $installer -Algorithm SHA256).Hash
"$hash  $($installer.Name)" | Set-Content "dist\Instalar_SITFA_Lite_$version.sha256" -Encoding ascii
Write-Host "Listo: $($installer.FullName)"
Write-Host "SHA256: $hash"
