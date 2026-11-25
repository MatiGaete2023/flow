; Instalador Inno Setup por usuario para Flow Rust (E-I1).
; Respeta %LOCALAPPDATA%\FlowRust\actualizaciones de E-I2 y no requiere administrador.
; Uso: ISCC.exe "/DAppVersion=1.0.0" "/DSourceDir=..\app\target\entrega" installer\FlowRust.iss
#ifndef AppVersion
  #define AppVersion "1.0.0"
#endif
#ifndef SourceDir
  #define SourceDir "..\app\target\entrega"
#endif
#ifndef OutputDir
  #define OutputDir "..\app\target\entrega"
#endif
[Setup]
AppId={{8D4F2E1A-9C3B-4B7E-A12F-6C9E4D8A1B20}
AppName=Flow Rust
AppVersion={#AppVersion}
AppPublisher=NuRus
DefaultDirName={localappdata}\Programs\FlowRust
DefaultGroupName=Flow Rust
PrivilegesRequired=lowest
DisableProgramGroupPage=yes
OutputDir={#OutputDir}
OutputBaseFilename=Instalar_Flow_Rust_{#AppVersion}_Windows
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
UninstallDisplayIcon={app}\flow_rust.exe
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
[Tasks]
Name: "desktopicon"; Description: "Crear un acceso directo en el escritorio"; GroupDescription: "Accesos directos:"; Flags: unchecked
[Dirs]
Name: "{localappdata}\FlowRust\actualizaciones"; Flags: uninsneveruninstall
[Files]
Source: "{#SourceDir}\flow_rust.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\SHA256.txt"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
[Icons]
Name: "{autoprograms}\Flow"; Filename: "{app}\flow_rust.exe"
Name: "{autodesktop}\Flow"; Filename: "{app}\flow_rust.exe"; Tasks: desktopicon
[UninstallDelete]
Type: files; Name: "{app}\flow_rust.exe.anterior"
Type: files; Name: "{app}\flow_rust.exe.intercambio"
Type: files; Name: "{app}\SHA256.txt"
[Run]
Filename: "{app}\flow_rust.exe"; Description: "Abrir Flow Rust"; Flags: nowait postinstall skipifsilent
