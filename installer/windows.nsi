# -*- coding: utf-8 -*-
Unicode true

!ifndef APP_VERSION
  !error "APP_VERSION must be provided by package-windows.ps1"
!endif
!ifndef OUTPUT_FILE
  !error "OUTPUT_FILE must be provided by package-windows.ps1"
!endif
!ifndef APP_BINARY
  !error "APP_BINARY must be provided by package-windows.ps1"
!endif
!ifndef APP_ICON
  !error "APP_ICON must be provided by package-windows.ps1"
!endif

!include "MUI2.nsh"

!define APP_NAME "Jump1"
!define APP_REG_KEY "Software\JCLee777\Jump1"
!define UNINSTALL_REG_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Jump1"

Name "${APP_NAME} ${APP_VERSION}"
OutFile "${OUTPUT_FILE}"
InstallDir "$LOCALAPPDATA\Programs\${APP_NAME}"
InstallDirRegKey HKCU "${APP_REG_KEY}" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
Icon "${APP_ICON}"
UninstallIcon "${APP_ICON}"
ShowInstDetails show
ShowUninstDetails show

VIProductVersion "${APP_VERSION}.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "FileDescription" "${APP_NAME} Windows Installer"
VIAddVersionKey "CompanyName" "JCLee777"
VIAddVersionKey "LegalCopyright" "Copyright (c) JCLee777"
VIAddVersionKey "FileVersion" "${APP_VERSION}"
VIAddVersionKey "ProductVersion" "${APP_VERSION}"

!define MUI_ABORTWARNING
!define MUI_ICON "${APP_ICON}"
!define MUI_UNICON "${APP_ICON}"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

Section "Jump1" MainSection
  SectionIn RO
  SetShellVarContext current
  SetOutPath "$INSTDIR"
  File /oname=Jump1.exe "${APP_BINARY}"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  CreateDirectory "$SMPROGRAMS\Jump1"
  CreateShortCut "$SMPROGRAMS\Jump1\Jump1.lnk" "$INSTDIR\Jump1.exe" "" "$INSTDIR\Jump1.exe" 0
  CreateShortCut "$SMPROGRAMS\Jump1\Uninstall Jump1.lnk" "$INSTDIR\Uninstall.exe"

  WriteRegStr HKCU "${APP_REG_KEY}" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_REG_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "${UNINSTALL_REG_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "${UNINSTALL_REG_KEY}" "Publisher" "JCLee777"
  WriteRegStr HKCU "${UNINSTALL_REG_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_REG_KEY}" "DisplayIcon" "$INSTDIR\Jump1.exe"
  WriteRegStr HKCU "${UNINSTALL_REG_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegDWORD HKCU "${UNINSTALL_REG_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_REG_KEY}" "NoRepair" 1
SectionEnd

Function un.onInit
  SetShellVarContext current
FunctionEnd

Section un.Uninstall
  Delete "$SMPROGRAMS\Jump1\Jump1.lnk"
  Delete "$SMPROGRAMS\Jump1\Uninstall Jump1.lnk"
  RMDir "$SMPROGRAMS\Jump1"

  Delete "$INSTDIR\Jump1.exe"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"

  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Jump1"
  DeleteRegKey HKCU "${UNINSTALL_REG_KEY}"
  DeleteRegKey HKCU "${APP_REG_KEY}"
SectionEnd
