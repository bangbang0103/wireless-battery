#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif

#define AppName "Wireless Battery"
#define AppExeName "wireless-battery-tray.exe"
#define CliExeName "wireless-battery.exe"
#define AppPublisher "Wireless Battery Contributors"

[Setup]
AppId={{8D6B5F2B-9A45-4C1B-B6F2-24C49F8A7B11}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
AllowNoIcons=yes
PrivilegesRequiredOverridesAllowed=dialog
LicenseFile=..\..\..\..\LICENSE
OutputDir=..\..\..\..\dist
OutputBaseFilename=wireless-battery-setup-windows-x64
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\{#AppExeName}
ChangesEnvironment=yes
AppMutex=Local\WirelessBatteryTraySingleInstance,Local\WirelessBatteryTrayDevSingleInstance

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "addpath"; Description: "Add wireless-battery CLI to PATH"; GroupDescription: "CLI:"

[Files]
Source: "..\..\..\target\release\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\..\..\target\release\{#CliExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\..\..\NOTICE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"
Name: "{group}\{cm:UninstallProgram,{#AppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: addpath; Check: (not IsAdminInstallMode) and NeedsAddPath(ExpandConstant('{app}'))
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: addpath; Check: IsAdminInstallMode and NeedsAddPath(ExpandConstant('{app}'))

[Run]
Filename: "{app}\{#AppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(AppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
  RootKey: Integer;
  SubKey: string;
begin
  if IsAdminInstallMode then begin
    RootKey := HKEY_LOCAL_MACHINE;
    SubKey := 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment';
  end else begin
    RootKey := HKEY_CURRENT_USER;
    SubKey := 'Environment';
  end;

  if not RegQueryStringValue(RootKey, SubKey, 'Path', OrigPath) then begin
    Result := True;
    exit;
  end;

  Result := Pos(';' + UpperCase(Param) + ';', ';' + UpperCase(OrigPath) + ';') = 0;
end;

procedure RemoveDirFromPath(const RootKey: Integer; const SubKey: string; const TargetDir: string);
var
  OrigPath, NewPath, CleanTarget, Entry: string;
  SepPos: Integer;
  Changed: Boolean;
begin
  if not RegQueryStringValue(RootKey, SubKey, 'Path', OrigPath) then
    exit;

  CleanTarget := RemoveBackslashUnlessRoot(TargetDir);
  NewPath := '';
  Changed := False;
  while OrigPath <> '' do begin
    SepPos := Pos(';', OrigPath);
    if SepPos > 0 then begin
      Entry := Copy(OrigPath, 1, SepPos - 1);
      OrigPath := Copy(OrigPath, SepPos + 1, Length(OrigPath) - SepPos);
    end else begin
      Entry := OrigPath;
      OrigPath := '';
    end;
    if Entry = '' then
      continue;
    if CompareText(RemoveBackslashUnlessRoot(Entry), CleanTarget) = 0 then begin
      Changed := True;
    end else begin
      if NewPath <> '' then
        NewPath := NewPath + ';';
      NewPath := NewPath + Entry;
    end;
  end;

  if Changed then
    RegWriteExpandStringValue(RootKey, SubKey, 'Path', NewPath);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then begin
    RegDeleteValue(HKEY_CURRENT_USER, 'Software\Microsoft\Windows\CurrentVersion\Run', 'WirelessBatteryTray');
    if IsAdminInstallMode then
      RemoveDirFromPath(HKEY_LOCAL_MACHINE, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', ExpandConstant('{app}'))
    else
      RemoveDirFromPath(HKEY_CURRENT_USER, 'Environment', ExpandConstant('{app}'));
  end;
end;
