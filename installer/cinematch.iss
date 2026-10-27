; CineMatch - installer Windows per-utente (nessun UAC).
; Compilare dalla root del progetto:
;   ISCC.exe /DMyAppVersion=0.4.6 installer\cinematch.iss
; Il setup completo aggiorna l'installazione esistente e non tocca
; %APPDATA%\CineMatch (configurazione, chiavi e cache).

#ifndef MyAppVersion
  #define MyAppVersion "0.4.6"
#endif

#define MyAppName "CineMatch"
#define MyAppPublisher "Emanuele Paris"
#define MyAppURL "https://github.com/Emanueleparis98/cinematch"
#define MyAppExeName "cinematch.exe"

[Setup]
AppId={{E32D740A-B06A-430A-953A-F623C1C6B3E3}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases/latest
AppComments=Esplorazione cinematografica a vincoli, con chat e ricerca TMDB.
AppCopyright=Copyright (C) 2026 {#MyAppPublisher}
VersionInfoVersion={#MyAppVersion}
VersionInfoCompany={#MyAppPublisher}
VersionInfoDescription={#MyAppName} installer
VersionInfoProductName={#MyAppName}
VersionInfoProductVersion={#MyAppVersion}
DefaultDirName={localappdata}\Programs\CineMatch
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
AllowNoIcons=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
OutputDir=..\dist
OutputBaseFilename=CineMatch-Setup-{#MyAppVersion}
SetupIconFile=..\assets\cinematch.ico
UninstallDisplayName={#MyAppName}
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
UsePreviousAppDir=yes
UsePreviousTasks=yes
SetupLogging=no
ChangesAssociations=no

[Languages]
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"

[Tasks]
Name: "desktopicon"; Description: "Crea un collegamento sul desktop"; GroupDescription: "Collegamenti:"; Flags: unchecked

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\Disinstalla {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
; Anche in modalita silenziosa (aggiornamento) rilancia l'app alla fine.
Filename: "{app}\{#MyAppExeName}"; Description: "Avvia {#MyAppName}"; Flags: nowait
