#define AppName "Orbit"
#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif

[Setup]
AppId={{BDF8B61C-26F2-4CEE-A76D-B24780DDC2E0}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=Orbit contributors
AppPublisherURL=https://github.com/Hi9841/Orbit
AppSupportURL=https://github.com/Hi9841/Orbit/issues
AppUpdatesURL=https://github.com/Hi9841/Orbit/releases
DefaultDirName={localappdata}\Programs\Orbit
DefaultGroupName=Orbit
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.19045
WizardStyle=modern
UninstallDisplayIcon={app}\orbit.exe
SetupIconFile=..\assets\orbit.ico
OutputDir=..\dist
OutputBaseFilename=OrbitSetup-{#AppVersion}-x64
Compression=lzma2
SolidCompression=yes
CloseApplications=yes
RestartApplications=no
LicenseFile=..\LICENSE

[Files]
Source: "..\target\release\orbit.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\NOTICE.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\THIRD-PARTY-NOTICES.txt"; DestDir: "{app}"; Flags: ignoreversion

[Registry]
Root: HKCU; Subkey: "Software\Classes\orbit"; ValueType: string; ValueData: "URL:Orbit window action"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\orbit"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""
Root: HKCU; Subkey: "Software\Classes\orbit\DefaultIcon"; ValueType: string; ValueData: "{app}\orbit.exe,0"
Root: HKCU; Subkey: "Software\Classes\orbit\shell\open\command"; ValueType: string; ValueData: """{app}\orbit.exe"" ""%1"""

[Icons]
Name: "{group}\Orbit"; Filename: "{app}\orbit.exe"

[Run]
Filename: "{app}\orbit.exe"; Description: "Launch Orbit"; Flags: postinstall nowait skipifsilent; Check: not IsUpdate
Filename: "{app}\orbit.exe"; Parameters: "--resident"; Flags: nowait; Check: IsUpdate

[Code]
var
  RemoveSettings: Boolean;

function IsUpdate(): Boolean;
begin
  Result := ExpandConstant('{param:UPDATE|0}') = '1';
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ExitCode: Integer;
begin
  Result := '';
  if FileExists(ExpandConstant('{app}\orbit.exe')) then begin
    if not Exec(ExpandConstant('{app}\orbit.exe'), '--quit', '', SW_HIDE, ewWaitUntilTerminated, ExitCode) then
      Result := 'Close Orbit from the notification area, then retry the installation.'
    else if ExitCode <> 0 then
      Result := 'Orbit could not close. Close it from the notification area, then retry.';
  end;
end;

function InitializeUninstall(): Boolean;
var
  ExitCode: Integer;
begin
  Result := False;
  if FileExists(ExpandConstant('{app}\orbit.exe')) then begin
    if not Exec(ExpandConstant('{app}\orbit.exe'), '--quit', '', SW_HIDE, ewWaitUntilTerminated, ExitCode) then begin
      if not UninstallSilent then
        MsgBox('Close Orbit from the notification area, then retry uninstalling.', mbError, MB_OK);
      Exit;
    end;
    if ExitCode <> 0 then begin
      if not UninstallSilent then
        MsgBox('Orbit could not close. Close it from the notification area, then retry uninstalling.', mbError, MB_OK);
      Exit;
    end;
  end;
  RemoveSettings := ExpandConstant('{param:REMOVESETTINGS|0}') = '1';
  if not UninstallSilent then
    RemoveSettings := MsgBox('Remove Orbit settings as well?', mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES;
  Result := True;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then begin
    RegDeleteValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'Orbit');
    if RemoveSettings then
      DelTree(ExpandConstant('{localappdata}\Orbit'), True, True, True);
  end;
end;
