$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$exe = Join-Path $scriptDir 'target\release\barnes-hut.exe'
if (-not (Test-Path $exe)) {
    Write-Host 'Fehler: Release-Binary nicht gefunden.' -ForegroundColor Red
    Write-Host 'Bitte zuerst `cargo build --release` im Projektordner ausfuehren.'
    exit 1
}
Write-Host 'Starte die Release-Version...'
Start-Process -FilePath $exe -WorkingDirectory $scriptDir
