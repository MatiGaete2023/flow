; Build after PyInstaller: ISCC.exe installer\CSMP.iss
#ifndef AppVersion
  #define AppVersion "0.5.0-dev4"
#endif
[Setup]
AppId={{AB37C9B2-69F7-47B3-9D78-3BA9D86A63A9}
AppName=CSMP Assistant
AppVersion={#AppVersion}
AppPublisher=NuRus
DefaultDirName={localappdata}\Programs\CSMP_Assistant
DefaultGroupName=CSMP Assistant
PrivilegesRequired=lowest
DisableProgramGroupPage=yes
OutputDir=..\dist
OutputBaseFilename=Instalar_CSMP_{#AppVersion}_Windows
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
UninstallDisplayIcon={app}\CSMP_Integral.exe
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
[Tasks]
Name: "desktopicon"; Description: "Crear un acceso directo en el escritorio"; GroupDescription: "Accesos directos:"; Flags: unchecked
[Files]
Source: "..\dist\CSMP_Integral\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
[Icons]
Name: "{autoprograms}\CSMP Assistant"; Filename: "{app}\CSMP_Integral.exe"
Name: "{autodesktop}\CSMP Assistant"; Filename: "{app}\CSMP_Integral.exe"; Tasks: desktopicon
[Run]
Filename: "{app}\CSMP_Integral.exe"; Description: "Abrir CSMP Assistant"; Flags: nowait postinstall skipifsilent
