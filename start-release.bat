@echo off
setlocal
pushd "%~dp0"
if not exist "target\release\barnes-hut.exe" (
  echo Fehler: Release-Binary nicht gefunden.
  echo Bitte zuerst "cargo build --release" im Projektordner ausfuehren.
  pause
  exit /b 1
)
echo Starte die Release-Version...
target\release\barnes-hut.exe
endlocal
