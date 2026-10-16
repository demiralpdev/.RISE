; RISE CLI Windows installer (rise-core) — NSIS 3.x script.
;
; Build with:  ./build.sh [version]
; Direct:      makensis -INPUTCHARSET UTF8 -DVERSION=0.2.5 RISE-Setup.nsi
;
; The payload (rel-rise-core-x86_64-pc-windows-gnu.exe) is produced by the
; core cross-compile and shipped on the matching GitHub release; build.sh
; fetches it when missing. rise.ico is the installer/uninstaller icon.
;
; x64 only. The installer is unsigned, so SmartScreen will ask the user to
; click "More info" -> "Run anyway".

Unicode true
!cd "${__FILEDIR__}"

!include "MUI2.nsh"
!include "x64.nsh"
!include "StrFunc.nsh"
!include "WinMessages.nsh"
!include "LogicLib.nsh"

; String helpers: StrStr (installer PATH contains-check) and UnStrRep
; (uninstaller PATH entry removal).
${Using:StrFunc} StrStr
${Using:StrFunc} UnStrRep

; --- Version / build inputs -------------------------------------------------
!ifndef VERSION
  !define VERSION "0.2.5"
!endif
!define APP_VERSION "${VERSION}"
; EstimatedSize (KB, DWORD) shown in Add/Remove Programs; build.sh computes the
; real payload size and passes -DPAYLOAD_KB=<n>. Fallback keeps plain compiles
; working.
!ifndef PAYLOAD_KB
  !define PAYLOAD_KB 8100
!endif

!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\RISE"
!define ENV_KEY "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"

; --- Installer identity -----------------------------------------------------
Name "RISE ${APP_VERSION}"
OutFile "RISE-Setup-${APP_VERSION}.exe"
InstallDir "$PROGRAMFILES64\RISE"
InstallDirRegKey HKLM "Software\RISE" "InstallDir"
RequestExecutionLevel admin
SetCompressor /SOLID lzma
ManifestDPIAware true
ShowInstDetails show
ShowUninstDetails show

VIProductVersion "${APP_VERSION}.0"
VIAddVersionKey "ProductName" "RISE"
VIAddVersionKey "FileDescription" "RISE CLI installer"
VIAddVersionKey "FileVersion" "${APP_VERSION}"
VIAddVersionKey "ProductVersion" "${APP_VERSION}"
VIAddVersionKey "CompanyName" "RISE Project"
VIAddVersionKey "LegalCopyright" "Apache-2.0"

; --- Modern UI configuration ------------------------------------------------
!define MUI_ABORTWARNING
!define MUI_ICON "rise.ico"
!define MUI_UNICON "rise.ico"

; Finish page: open a console and print rise-core --help so the freshly
; installed CLI is immediately visible. Parameters wrap the inner command in an
; extra pair of quotes (cmd /k strips the outer pair and keeps the path quoted).
!define MUI_FINISHPAGE_RUN "$SYSDIR\cmd.exe"
!define MUI_FINISHPAGE_RUN_PARAMETERS '/k $\"$\"$INSTDIR\rise-core.exe$\" --help$\"'
!define MUI_FINISHPAGE_RUN_TEXT "$(MSG_RUN_CMD)"

; --- Pages ------------------------------------------------------------------
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "../../../LICENSE"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; --- Languages --------------------------------------------------------------
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Turkish"

; --- Localized strings ------------------------------------------------------
LangString MSG_NEED_X64 ${LANG_ENGLISH} "RISE requires 64-bit Windows (x64)."
LangString MSG_NEED_X64 ${LANG_TURKISH} "RISE 64-bit Windows (x64) gerektirir."

LangString MSG_RUN_CMD ${LANG_ENGLISH} "Open a command prompt and run 'rise-core --help'"
LangString MSG_RUN_CMD ${LANG_TURKISH} "Komut istemini aç ve 'rise-core --help' çalıştır"

LangString MSG_SEC_CORE ${LANG_ENGLISH} "RISE CLI (rise-core)"
LangString MSG_SEC_CORE ${LANG_TURKISH} "RISE CLI (rise-core)"

LangString MSG_SEC_PATH ${LANG_ENGLISH} "Add RISE to the system PATH"
LangString MSG_SEC_PATH ${LANG_TURKISH} "RISE'i sistem PATH'ine ekle"

LangString MSG_DESC_CORE ${LANG_ENGLISH} "Installs rise-core.exe and the uninstaller into $INSTDIR."
LangString MSG_DESC_CORE ${LANG_TURKISH} "rise-core.exe ve kaldırıcıyı $INSTDIR içine kurar."

LangString MSG_DESC_PATH ${LANG_ENGLISH} "Lets you run 'rise-core' from any command prompt (updates the system PATH)."
LangString MSG_DESC_PATH ${LANG_TURKISH} "'rise-core' komutunu her komut isteminden çalıştırmanızı sağlar (sistem PATH'ini günceller)."

; --- Init -------------------------------------------------------------------
Function .onInit
  ; Language picker first (installer is multi-language).
  !insertmacro MUI_LANGDLL_DISPLAY

  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP|MB_OK "$(MSG_NEED_X64)"
    Abort
  ${EndIf}

  SetRegView 64
FunctionEnd

Function un.onInit
  SetRegView 64
FunctionEnd

; --- PATH helpers -----------------------------------------------------------
Function AddToPath
  ReadRegStr $0 HKLM "${ENV_KEY}" "Path"
  ${StrStr} $1 "$0" "$INSTDIR"
  ${If} $1 != ""
    ; Already present — nothing to do.
    Return
  ${EndIf}

  ; Drop one trailing ';' so we never write ";;".
  StrLen $2 "$0"
  ${If} $2 > 0
    StrCpy $3 "$0" 1 -1
    ${If} $3 == ";"
      IntOp $2 $2 - 1
      StrCpy $0 "$0" $2
    ${EndIf}
  ${EndIf}

  ${If} $0 == ""
    StrCpy $0 "$INSTDIR"
  ${Else}
    StrCpy $0 "$0;$INSTDIR"
  ${EndIf}

  WriteRegExpandStr HKLM "${ENV_KEY}" "Path" "$0"
  Call BroadcastEnv
FunctionEnd

Function un.RemoveFromPath
  ReadRegStr $0 HKLM "${ENV_KEY}" "Path"
  ${UnStrRep} $0 "$0" ";$INSTDIR" ""
  ${If} $0 == "$INSTDIR"
    StrCpy $0 ""
  ${EndIf}
  ${If} $0 == ""
    DeleteRegValue HKLM "${ENV_KEY}" "Path"
  ${Else}
    WriteRegExpandStr HKLM "${ENV_KEY}" "Path" "$0"
    Call un.BroadcastEnv
  ${EndIf}
FunctionEnd

Function BroadcastEnv
  System::Call 'user32::SendMessageTimeoutW(i ${HWND_BROADCAST}, i ${WM_SETTINGCHANGE}, i 0, w "Environment", i 2, i 5000, *i .r0)'
FunctionEnd

Function un.BroadcastEnv
  System::Call 'user32::SendMessageTimeoutW(i ${HWND_BROADCAST}, i ${WM_SETTINGCHANGE}, i 0, w "Environment", i 2, i 5000, *i .r0)'
FunctionEnd

; --- Sections ---------------------------------------------------------------
Section "$(MSG_SEC_CORE)" SEC_CORE
  SectionIn RO
  SetOutPath "$INSTDIR"
  File "/oname=rise-core.exe" "rel-rise-core-x86_64-pc-windows-gnu.exe"
  File "rise.ico"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  ; Add/Remove Programs entry
  WriteRegStr   HKLM "${UNINST_KEY}" "DisplayName"          "RISE CLI"
  WriteRegStr   HKLM "${UNINST_KEY}" "DisplayVersion"       "${APP_VERSION}"
  WriteRegStr   HKLM "${UNINST_KEY}" "Publisher"            "RISE Project"
  WriteRegStr   HKLM "${UNINST_KEY}" "URLInfoAbout"         "https://riseproof.me"
  WriteRegStr   HKLM "${UNINST_KEY}" "InstallLocation"      "$INSTDIR"
  WriteRegStr   HKLM "${UNINST_KEY}" "DisplayIcon"          "$INSTDIR\rise.ico"
  WriteRegStr   HKLM "${UNINST_KEY}" "UninstallString"      "$INSTDIR\Uninstall.exe"
  WriteRegStr   HKLM "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegDWORD HKLM "${UNINST_KEY}" "EstimatedSize"        "${PAYLOAD_KB}"
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoModify"             1
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoRepair"             1

  WriteRegStr HKLM "Software\RISE" "InstallDir" "$INSTDIR"
SectionEnd

Section "$(MSG_SEC_PATH)" SEC_PATH
  Call AddToPath
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_CORE} "$(MSG_DESC_CORE)"
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_PATH} "$(MSG_DESC_PATH)"
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; --- Uninstall --------------------------------------------------------------
Section "Uninstall"
  Call un.RemoveFromPath
  Delete "$INSTDIR\rise-core.exe"
  Delete "$INSTDIR\rise.ico"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKLM "${UNINST_KEY}"
  DeleteRegKey HKLM "Software\RISE"
SectionEnd
