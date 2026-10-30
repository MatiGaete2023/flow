; Instalador autoinstalable de SITFA Lite. Compilar después de PyInstaller y de generar los manuales HTML:
;   ISCC.exe "/DAppVersion=1.0.0" build\SITFA_Lite.iss
#ifndef AppVersion
  #error AppVersion es obligatoria: pasarla con /DAppVersion (app/version.py)
#endif
[Setup]
AppId={{6E0D5A7C-3B4F-4C1E-9F57-2B8E1C0A5D11}
AppName=SITFA Lite
AppVersion={#AppVersion}
AppVerName=SITFA Lite {#AppVersion}
AppPublisher=Flow · uso local
DefaultDirName={localappdata}\Programs\SITFA_Lite
DefaultGroupName=SITFA Lite
PrivilegesRequired=lowest
DisableProgramGroupPage=yes
OutputDir=..\dist
OutputBaseFilename=Instalar_SITFA_Lite_{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
UninstallDisplayIcon={app}\SITFA_Lite.exe
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
InfoBeforeFile=..\dist\manuales\ANTES_DE_INSTALAR.txt

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[Tasks]
Name: "desktopicon"; Description: "Crear un acceso directo en el escritorio"; GroupDescription: "Accesos directos:"

[Files]
Source: "..\dist\SITFA_Lite\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\extension\*"; DestDir: "{app}\extension"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\dist\manuales\*.html"; DestDir: "{app}\manuales"; Flags: ignoreversion
Source: "..\docs\SITFA_Lite_*.pdf"; DestDir: "{app}\manuales"; Flags: ignoreversion
Source: "..\docs\SITFA_Lite_Video_*.mp4"; DestDir: "{app}\manuales"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\SITFA Lite\SITFA Lite"; Filename: "{app}\SITFA_Lite.exe"
Name: "{autoprograms}\SITFA Lite\Manual de instalación (PDF)"; Filename: "{app}\manuales\SITFA_Lite_Manual_Instalacion.pdf"
Name: "{autoprograms}\SITFA Lite\Manual de uso (PDF)"; Filename: "{app}\manuales\SITFA_Lite_Manual_Uso.pdf"
Name: "{autoprograms}\SITFA Lite\Video · instalar la extensión"; Filename: "{app}\manuales\SITFA_Lite_Video_Instalar_Extension.mp4"
Name: "{autoprograms}\SITFA Lite\Video · instalación y uso completo"; Filename: "{app}\manuales\SITFA_Lite_Video_Instalacion_y_Uso.mp4"
Name: "{autoprograms}\SITFA Lite\Carpeta de la extensión de Chrome"; Filename: "{app}\extension"
Name: "{autodesktop}\SITFA Lite"; Filename: "{app}\SITFA_Lite.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\manuales\SITFA_Lite_Video_Instalar_Extension.mp4"; Description: "Ver el video para instalar la extensión de Chrome (paso obligatorio, una vez)"; Flags: postinstall shellexec skipifsilent
Filename: "{app}\SITFA_Lite.exe"; Description: "Abrir SITFA Lite"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: filesandordirs; Name: "{app}\_internal"
