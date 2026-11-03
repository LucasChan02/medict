#ifndef MyAppVersion
  #define MyAppVersion "4.0.1"
#endif

#ifndef StageDir
  #define StageDir "stage"
#endif

#ifndef OutputDir
  #define OutputDir "dist"
#endif

[Setup]
AppId={{6D2B8F98-9A77-4B75-9A15-0F05A37B1E26}
AppName=Medict
AppVersion={#MyAppVersion}
AppPublisher=Medict contributors
AppPublisherURL=https://github.com/terasum/medict
AppSupportURL=https://github.com/terasum/medict/issues
DefaultDirName={autopf}\Medict
DefaultGroupName=Medict
DisableProgramGroupPage=yes
OutputDir={#OutputDir}
OutputBaseFilename=Medict-{#MyAppVersion}-windows-x86_64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
SetupIconFile=..\..\build\windows\icon.ico
UninstallDisplayIcon={app}\Medict.exe
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog

[Files]
Source: "{#StageDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional icons:"

[Icons]
Name: "{autoprograms}\Medict"; Filename: "{app}\Medict.exe"
Name: "{autodesktop}\Medict"; Filename: "{app}\Medict.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\Medict.exe"; Description: "Launch Medict"; Flags: nowait postinstall skipifsilent
