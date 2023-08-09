; Source: https://gist.github.com/drewchapin/246de6d0c404a79ee66a5ead35b480bc

;-------------------------------------------------------------------------------
; Includes
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "WinVer.nsh"
!include "x64.nsh"

;-------------------------------------------------------------------------------
; Constants
!define PRODUCT_NAME "One Tagger"
!define PRODUCT_DESCRIPTION "App to tag your music library."
!define COPYRIGHT "Marekkon5"
!define PRODUCT_VERSION "1.0.0.0"
!define SETUP_VERSION 1.0.0.0

;-------------------------------------------------------------------------------
; Attributes
Name "One Tagger"
OutFile "..\dist\OneTagger-windows-setup.exe"
InstallDir "$PROGRAMFILES\OneTagger"
RequestExecutionLevel admin ; user|highest|admin
SetCompressor /SOLID lzma

;-------------------------------------------------------------------------------
; Version Info
VIProductVersion "${PRODUCT_VERSION}"
VIAddVersionKey "ProductName" "${PRODUCT_NAME}"
VIAddVersionKey "ProductVersion" "${PRODUCT_VERSION}"
VIAddVersionKey "FileDescription" "${PRODUCT_DESCRIPTION}"
VIAddVersionKey "LegalCopyright" "${COPYRIGHT}"
VIAddVersionKey "FileVersion" "${SETUP_VERSION}"

;-------------------------------------------------------------------------------
; Modern UI Appearance
!define MUI_ICON "..\assets\installer-icon.ico"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP "..\assets\headerimage.bmp"
!define MUI_WELCOMEFINISHPAGE_BITMAP "..\assets\welcomebanner.bmp"
!define MUI_FINISHPAGE_NOAUTOCLOSE

; Modern UI Desktop Shortcut
!define MUI_FINISHPAGE_SHOWREADME ""
!define MUI_FINISHPAGE_SHOWREADME_NOTCHECKED
!define MUI_FINISHPAGE_SHOWREADME_TEXT "Create Desktop Shortcut"
!define MUI_FINISHPAGE_SHOWREADME_FUNCTION desktopshortcut

;-------------------------------------------------------------------------------
; Installer Pages
!insertmacro MUI_PAGE_WELCOME
;!insertmacro MUI_PAGE_LICENSE "${NSISDIR}\Docs\Modern UI\License.txt"
;!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

;-------------------------------------------------------------------------------
; Uninstaller Pages
!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

;-------------------------------------------------------------------------------
; Languages
!insertmacro MUI_LANGUAGE "English"

;-------------------------------------------------------------------------------
; Installer Sections
Section "One Tagger" OneTagger
	; Clean old
	ExecWait "taskkill /f /im onetagger.exe"
	RMDir /r "$INSTDIR\*"
	; Copy new
	SetOutPath $INSTDIR
	File "..\target\release\onetagger.exe"
	File "..\assets\icon.ico"
	File "..\vc_redist.x64.exe"
	File "..\MicrosoftEdgeWebview2Setup.exe"
	; Uninstaller
	WriteUninstaller "$INSTDIR\Uninstall.exe"
	CreateDirectory "$SMPROGRAMS\OneTagger"
	CreateShortcut "$SMPROGRAMS\OneTagger\${PRODUCT_NAME}.lnk" "$INSTDIR\onetagger.exe" "" "$INSTDIR\icon.ico"
	; Registry
	WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" \
		"DisplayName" "${PRODUCT_NAME}"
	WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" \
		"UninstallString" "$INSTDIR\Uninstall.exe"
	WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" \
		"EstimatedSize" "70656"
	WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" \
		"DisplayIcon" "$INSTDIR\icon.ico"
	WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" \
		"InstallLocation" "$INSTDIR"
	; Dependencies
	ExecWait '"$INSTDIR\vc_redist.x64.exe" /install /quiet /norestart'
	ExecWait '"$INSTDIR\MicrosoftEdgeWebview2Setup.exe'
	Delete "$INSTDIR\vc_redist.x64.exe"
	Delete "$INSTDIR\MicrosoftEdgeWebview2Setup.exe"
SectionEnd

;-------------------------------------------------------------------------------
; Uninstaller Sections
Section "Uninstall"
	Delete "$SMPROGRAMS\OneTagger\${PRODUCT_NAME}.lnk"
	Delete "$DESKTOP\${PRODUCT_NAME}.lnk" 
	RMDir "$SMPROGRAMS\OneTagger"
	Delete "$INSTDIR\*"
	RMDir /r "$INSTDIR\*"
	RMDir "$INSTDIR"
SectionEnd

;------
; Desktop icon
Function desktopshortcut
	CreateShortcut "$DESKTOP\${PRODUCT_NAME}.lnk" "$INSTDIR\onetagger.exe" "" "$INSTDIR\icon.ico"
FunctionEnd