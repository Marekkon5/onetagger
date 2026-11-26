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
!ifndef PRODUCT_VERSION
!error "Pass /DPRODUCT_VERSION=X.Y.Z.0 using the current VERSION file"
!endif
!define SETUP_VERSION "${PRODUCT_VERSION}"
!define WEBVIEW2_KEY "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"

;-------------------------------------------------------------------------------
; Attributes
Name "One Tagger"
OutFile "..\dist\OneTagger-windows-setup.exe"
InstallDir "$PROGRAMFILES64\OneTagger"
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
Function ReadWebView2Version
	SetRegView 32
	ReadRegStr $0 HKLM "${WEBVIEW2_KEY}" "pv"
	${If} $0 == ""
	${OrIf} $0 == "0.0.0.0"
		ReadRegStr $0 HKCU "${WEBVIEW2_KEY}" "pv"
	${EndIf}
	SetRegView 64
FunctionEnd

Function EnsureWebView2
	Call ReadWebView2Version
	${If} $0 != ""
	${AndIf} $0 != "0.0.0.0"
		DetailPrint "WebView2 Runtime $0 is already installed"
		Return
	${EndIf}

	DetailPrint "Installing the Microsoft Edge WebView2 Runtime"
	InitPluginsDir
	SetOutPath "$PLUGINSDIR"
	File "..\MicrosoftEdgeWebview2Setup.exe"
	ExecWait '"$PLUGINSDIR\MicrosoftEdgeWebview2Setup.exe" /silent /install' $1
	Call ReadWebView2Version
	${If} $0 == ""
	${OrIf} $0 == "0.0.0.0"
		SetErrorLevel 1
		Abort "Microsoft Edge WebView2 Runtime setup failed. Install the Runtime and try again."
	${EndIf}
FunctionEnd

Section "One Tagger" OneTagger
	Call EnsureWebView2
	; Clean old
	ExecWait "taskkill /f /im onetagger.exe"
	; Keep unrelated files in the selected installation directory.
	; Copy new
	SetOutPath $INSTDIR
	File "..\target\release\onetagger.exe"
	File "..\assets\icon.ico"
	; Uninstaller
	WriteUninstaller "$INSTDIR\Uninstall.exe"
	CreateDirectory "$SMPROGRAMS\OneTagger"
	CreateShortcut "$SMPROGRAMS\OneTagger\${PRODUCT_NAME}.lnk" "$INSTDIR\onetagger.exe" "" "$INSTDIR\icon.ico"
	; Registry
	WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" \
		"DisplayName" "${PRODUCT_NAME}"
	WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" \
		"UninstallString" '"$INSTDIR\Uninstall.exe"'
	WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" \
		"EstimatedSize" "70656"
	WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" \
		"DisplayIcon" "$INSTDIR\icon.ico"
	WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" \
		"InstallLocation" "$INSTDIR"
	WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "DisplayVersion" "${PRODUCT_VERSION}"
	WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "URLInfoAbout" "https://github.com/fantuz/onetagger"
SectionEnd

;-------------------------------------------------------------------------------
; Uninstaller Sections
Section "Uninstall"
	SetRegView 64
	DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"
	Delete "$SMPROGRAMS\OneTagger\${PRODUCT_NAME}.lnk"
	Delete "$DESKTOP\${PRODUCT_NAME}.lnk"
	RMDir "$SMPROGRAMS\OneTagger"
	Delete "$INSTDIR\onetagger.exe"
	Delete "$INSTDIR\icon.ico"
	Delete "$INSTDIR\Uninstall.exe"
	; Keep unrelated files in the selected installation directory.
	RMDir "$INSTDIR"
SectionEnd

;------
; Desktop icon
Function desktopshortcut
	CreateShortcut "$DESKTOP\${PRODUCT_NAME}.lnk" "$INSTDIR\onetagger.exe" "" "$INSTDIR\icon.ico"
FunctionEnd
