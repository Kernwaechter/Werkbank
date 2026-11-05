; Inno Setup 6 – baut den Windows-Installer (Setup.exe).
; Aufruf unter Windows:  iscc packaging\windows\werkbank.iss
; Erwartet target\x86_64-pc-windows-msvc\release\werkbank-app.exe (siehe windows-bauen.sh).
#define Version "1.2.2"

[Setup]
AppId={{5D0B6C4A-7E1F-4B7E-9A55-DAEHN-WERKBANK}
AppName=Werkbank
AppVersion={#Version}
AppPublisher=Volker Dähn
DefaultDirName={autopf}\Werkbank
DefaultGroupName=Werkbank
UninstallDisplayIcon={app}\werkbank-app.exe
SetupIconFile=..\..\assets\icons\werkbank.ico
OutputDir=..\..\target\windows
OutputBaseFilename=Werkbank-{#Version}-Setup
Compression=lzma2
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequiredOverridesAllowed=dialog
WizardStyle=modern
LicenseFile=..\..\LIZENZVEREINBARUNG.md

[Languages]
Name: "german"; MessagesFile: "compiler:Languages\German.isl"

[Tasks]
Name: "desktopicon"; Description: "Symbol auf dem Desktop anlegen"; Flags: unchecked
Name: "mdassoc"; Description: "Markdown-Dateien (.md) mit der Werkbank öffnen anbieten"; Flags: unchecked

[Files]
Source: "..\..\target\x86_64-pc-windows-msvc\release\werkbank-app.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\assets\handbuch.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\assets\lizenzen.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\assets\fonts\OFL.txt"; DestDir: "{app}"; DestName: "Fira-Sans-OFL.txt"; Flags: ignoreversion
Source: "..\..\LIZENZVEREINBARUNG.md"; DestDir: "{app}"; Flags: ignoreversion
; Hunspell-Wörterbücher (de_DE.aff/.dic samt Lizenztext), falls im Ordner packaging\windows\woerterbuecher abgelegt
Source: "woerterbuecher\*"; DestDir: "{app}\woerterbuecher"; Flags: ignoreversion recursesubdirs skipifsourcedoesntexist

[Icons]
Name: "{group}\Werkbank"; Filename: "{app}\werkbank-app.exe"
Name: "{group}\Werkbank deinstallieren"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Werkbank"; Filename: "{app}\werkbank-app.exe"; Tasks: desktopicon

[Registry]
Root: HKA; Subkey: "Software\Classes\Applications\werkbank-app.exe\shell\open\command"; ValueType: string; ValueData: """{app}\werkbank-app.exe"" ""%1"""; Tasks: mdassoc; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\.md\OpenWithList\werkbank-app.exe"; ValueType: none; Tasks: mdassoc; Flags: uninsdeletekey

[Run]
Filename: "{app}\werkbank-app.exe"; Description: "Werkbank starten"; Flags: nowait postinstall skipifsilent
