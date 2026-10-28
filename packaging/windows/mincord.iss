; Mincord's Windows installer. scripts/package-windows.py compiles it with
; Inno Setup 7 under Wine, from the same files as the portable ZIP.
;
; It installs for the person running it, without administrator rights, and can
; start Mincord in the tray when they sign in. That is the Run key value
; Settings > Advanced > "Start when you sign in" writes (src/autostart.rs), so
; the installer and the setting turn the same thing on and off.

#ifndef AppVersion
  #error Compile through scripts/package-windows.py, which defines AppVersion, VersionNumber and SourceDir
#endif
; The full preview defines Edition and FileSuffix; the Discord release neither.
; Both share the AppId, so installing one replaces the other.
#ifdef Edition
  #define DisplayName "Mincord (" + Edition + ")"
#else
  #define DisplayName "Mincord"
  #define FileSuffix ""
#endif

[Setup]
; Windows knows an installed Mincord by this; never change it.
AppId={{8496B4DE-F625-4A5F-999E-3C03E0E7C5D2}
AppName=Mincord
AppVersion={#AppVersion}
AppVerName={#DisplayName} {#AppVersion}
AppPublisher=Mincord
AppPublisherURL=https://mincord.com
AppSupportURL=https://github.com/mincord/mincord/issues
AppUpdatesURL=https://github.com/mincord/mincord/releases
VersionInfoVersion={#VersionNumber}
; Per user: %LOCALAPPDATA%\Programs\Mincord, with no UAC prompt.
PrivilegesRequired=lowest
DefaultDirName={autopf}\Mincord
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputBaseFilename=Mincord-{#AppVersion}{#FileSuffix}-windows-x64-setup
SetupIconFile=mincord.ico
UninstallDisplayIcon={app}\mincord.exe
UninstallDisplayName={#DisplayName}
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
; Mincord keeps running in the tray with its window closed, so an update or
; uninstall closes it rather than asking.
CloseApplications=force
RestartApplications=no
#ifdef LicenseFile
LicenseFile={#LicenseFile}
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
english.StartOnSignIn=Start Mincord automatically when Windows starts
english.StartOnSignInGroup=Startup:

[Tasks]
Name: "startonsignin"; Description: "{cm:StartOnSignIn}"; GroupDescription: "{cm:StartOnSignInGroup}"
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; README.txt explains the ZIP; the installer needs no instructions.
Source: "{#SourceDir}\*"; Excludes: "README.txt"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\Mincord"; Filename: "{app}\mincord.exe"
Name: "{autodesktop}\Mincord"; Filename: "{app}\mincord.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Mincord"; ValueData: """{app}\mincord.exe"" --background"; Tasks: startonsignin
; Chosen here, it starts even if Task Manager had turned it off before.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"; ValueType: none; ValueName: "Mincord"; Flags: deletevalue dontcreatekey; Tasks: startonsignin
; Uninstalling stops starting at sign-in, however it was turned on.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "Mincord"; Flags: uninsdeletevalue dontcreatekey
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"; ValueType: none; ValueName: "Mincord"; Flags: uninsdeletevalue dontcreatekey

[Run]
Filename: "{app}\mincord.exe"; Description: "{cm:LaunchProgram,Mincord}"; Flags: nowait postinstall skipifsilent
; Mincord updating itself runs this installer with /VERYSILENT and
; /restartapp=1, and wants the new Mincord started once it is in place; the
; line above is skipped in a silent run. See src/update.rs.
Filename: "{app}\mincord.exe"; Flags: nowait postinstall skipifnotsilent; Check: RestartApp

[Code]
// Whether the person running this installer asked for Mincord to be started
// again afterwards: /restartapp=1, which only Mincord's own updater passes.
function RestartApp: Boolean;
begin
  Result := ExpandConstant('{param:restartapp|0}') = '1';
end;
