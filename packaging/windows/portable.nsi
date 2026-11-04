Unicode true
RequestExecutionLevel user
Name "Medict ${VERSION}"
OutFile "${OUTPUT_FILE}"
InstallDir "$TEMP\Medict-${VERSION}"
SilentInstall silent
AutoCloseWindow true
ShowInstDetails nevershow
SetCompressor /SOLID lzma
SetCompressorDictSize 64
SetOverwrite on

VIProductVersion "${VERSION}.0"
VIAddVersionKey /LANG=1033 "ProductName" "Medict"
VIAddVersionKey /LANG=1033 "FileDescription" "Portable Medict dictionary"
VIAddVersionKey /LANG=1033 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "LegalCopyright" "Copyright (c) 2026 Medict contributors"

Section "Medict"
  SetOutPath "$INSTDIR"
  File /r "${SOURCE_DIR}\*.*"
  ExecWait '"$INSTDIR\Medict.exe"'
SectionEnd
