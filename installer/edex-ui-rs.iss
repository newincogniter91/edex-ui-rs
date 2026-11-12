; Installer di eDEX-UI RS (Inno Setup 6). Versione passata con /DAppVersion=x.y.z
#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif

[Setup]
AppId={{6F1B0D55-3C2E-4B8A-9E55-ED3C5A1B7A10}
AppName=eDEX-UI RS
AppVersion={#AppVersion}
AppPublisher=newincogniter91
DefaultDirName={autopf}\eDEX-UI RS
DefaultGroupName=eDEX-UI RS
DisableProgramGroupPage=yes
OutputDir=..\dist
OutputBaseFilename=edex-ui-rs-setup
SetupIconFile=..\assets\app.ico
UninstallDisplayIcon={app}\app.ico
LicenseFile=..\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\edex-ui-rs.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\app.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Dirs]
; qui si possono copiare i font United Sans (vedi README)
Name: "{app}\fonts"

[Icons]
Name: "{group}\eDEX-UI RS"; Filename: "{app}\edex-ui-rs.exe"; IconFilename: "{app}\app.ico"
Name: "{group}\Disinstalla eDEX-UI RS"; Filename: "{uninstallexe}"
Name: "{autodesktop}\eDEX-UI RS"; Filename: "{app}\edex-ui-rs.exe"; IconFilename: "{app}\app.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\edex-ui-rs.exe"; Description: "{cm:LaunchProgram,eDEX-UI RS}"; Flags: nowait postinstall skipifsilent
