; Razer Taskbar - Inno Setup installer script
; Build with: ISCC.exe /DAppVersion=X.Y.Z /DSourceExe=target\...\razer-taskbar.exe installer.iss

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

#ifndef SourceExe
  #define SourceExe "target\release\razer-taskbar.exe"
#endif

[Setup]
AppName=Razer Taskbar
AppVersion={#AppVersion}
AppVerName=Razer Taskbar {#AppVersion}
AppPublisher=tex1988
AppPublisherURL=https://github.com/tex1988/razer-taskbar
AppSupportURL=https://github.com/tex1988/razer-taskbar/issues
AppUpdatesURL=https://github.com/tex1988/razer-taskbar/releases
DefaultDirName={autopf}\Razer Taskbar
DefaultGroupName=Razer Taskbar
OutputBaseFilename=razer-taskbar-{#AppVersion}-setup
OutputDir=Output
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
SetupIconFile=src\assets\app_icon.ico
UninstallDisplayIcon={app}\razer-taskbar.exe
; Require Razer Synapse (and thus Windows 10+)
MinVersion=10.0
; Install for current user only — no UAC prompt needed
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=commandline
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "startupicon"; Description: "Start Razer Taskbar automatically with Windows"; GroupDescription: "Startup options:"

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "razer-taskbar.exe"; Flags: ignoreversion

[Icons]
Name: "{group}\Razer Taskbar"; Filename: "{app}\razer-taskbar.exe"
Name: "{group}\{cm:UninstallProgram,Razer Taskbar}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Razer Taskbar"; Filename: "{app}\razer-taskbar.exe"; Tasks: desktopicon
Name: "{userstartup}\Razer Taskbar"; Filename: "{app}\razer-taskbar.exe"; Tasks: startupicon

[Run]
Filename: "{app}\razer-taskbar.exe"; Description: "{cm:LaunchProgram,Razer Taskbar}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "taskkill.exe"; Parameters: "/f /im razer-taskbar.exe"; Flags: runhidden; RunOnceId: "KillApp"
