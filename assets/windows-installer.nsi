!ifndef PRODUCT_VERSION
!error "PRODUCT_VERSION must be defined by the packaging script"
!endif

!ifndef SOURCE_DIR
!error "SOURCE_DIR must be defined by the packaging script"
!endif

!ifndef OUTPUT_FILE
!error "OUTPUT_FILE must be defined by the packaging script"
!endif

!define PRODUCT_NAME "NEO Emacs"
!define PRODUCT_PUBLISHER "eval-exec"
!define PRODUCT_WEB_SITE "https://github.com/eval-exec/neomacs"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"

Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "${OUTPUT_FILE}"
InstallDir "$PROGRAMFILES64\${PRODUCT_NAME}"
ShowInstDetails show
ShowUnInstDetails show
RequestExecutionLevel admin
SetCompressor /SOLID lzma

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"
!include "StrFunc.nsh"
!include "WinMessages.nsh"
!include "x64.nsh"

${Using:StrFunc} StrStr
${Using:StrFunc} UnStrRep

!define ENVIRONMENT_KEY "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"

!define MUI_ABORTWARNING
!define MUI_ICON "${NSISDIR}\Contrib\Graphics\Icons\modern-install.ico"
!define MUI_UNICON "${NSISDIR}\Contrib\Graphics\Icons\modern-uninstall.ico"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "English"

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_OK "${PRODUCT_NAME} requires 64-bit Windows."
    Abort
  ${EndIf}
  SetRegView 64
FunctionEnd

Function AddToSystemPath
  SetRegView 64
  ReadRegStr $0 HKLM "${ENVIRONMENT_KEY}" "Path"

  ${If} $0 == ""
    WriteRegExpandStr HKLM "${ENVIRONMENT_KEY}" "Path" "$INSTDIR\bin"
    StrCpy $4 "1"
  ${Else}
    StrCpy $1 ";$0;"
    StrCpy $2 ";$INSTDIR\bin;"
    ${StrStr} $3 "$1" "$2"
    ${If} $3 == ""
      WriteRegExpandStr HKLM "${ENVIRONMENT_KEY}" "Path" "$0;$INSTDIR\bin"
      StrCpy $4 "1"
    ${Else}
      StrCpy $4 "0"
    ${EndIf}
  ${EndIf}

  ${If} $4 == "1"
    SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
  ${EndIf}
  Push $4
FunctionEnd

Function un.RemoveFromSystemPath
  SetRegView 64
  ReadRegStr $0 HKLM "${ENVIRONMENT_KEY}" "Path"

  ${If} $0 != ""
    StrCpy $1 ";$0;"
    StrCpy $2 ";$INSTDIR\bin;"
    ${UnStrRep} $3 "$1" "$2" ";"
    ${If} $3 != "$1"
      ${If} $3 == ";"
        StrCpy $3 ""
      ${Else}
        StrLen $4 $3
        IntOp $4 $4 - 2
        StrCpy $3 $3 $4 1
      ${EndIf}
      WriteRegExpandStr HKLM "${ENVIRONMENT_KEY}" "Path" "$3"
      SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
    ${EndIf}
  ${EndIf}
FunctionEnd

Section "!${PRODUCT_NAME}" SEC_MAIN
  SetRegView 64
  SetOutPath "$INSTDIR"
  SetOverwrite on

  File /r "${SOURCE_DIR}\*.*"

  IfFileExists "$INSTDIR\vendor\gstreamer\gstreamer-runtime.msi" 0 +2
    ExecWait 'msiexec /i "$INSTDIR\vendor\gstreamer\gstreamer-runtime.msi" /qn ADDLOCAL=ALL'

  WriteUninstaller "$INSTDIR\uninstall.exe"

  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "DisplayName" "${PRODUCT_NAME}"
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "UninstallString" "$INSTDIR\uninstall.exe"
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "URLInfoAbout" "${PRODUCT_WEB_SITE}"
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "InstallLocation" "$INSTDIR"

  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKLM "${PRODUCT_UNINST_KEY}" "EstimatedSize" "$0"
SectionEnd

Section "Add to PATH" SEC_PATH
  Call AddToSystemPath
  Pop $0
  ${If} $0 == "1"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "AddedToPath" "1"
  ${EndIf}
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_MAIN} "Install ${PRODUCT_NAME} editor and runtime files."
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_PATH} "Add $INSTDIR\bin to your system PATH."
!insertmacro MUI_FUNCTION_DESCRIPTION_END

Section Uninstall
  SetRegView 64
  ReadRegStr $0 HKLM "${PRODUCT_UNINST_KEY}" "AddedToPath"
  ${If} $0 == "1"
    Call un.RemoveFromSystemPath
  ${EndIf}

  RMDir /r "$INSTDIR"

  DeleteRegKey HKLM "${PRODUCT_UNINST_KEY}"
SectionEnd
