$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition

$possibleCargo = @()
$cmd = Get-Command cargo -ErrorAction SilentlyContinue
if ($cmd) { $possibleCargo += $cmd.Source }
$possibleCargo += "$env:USERPROFILE\.cargo\bin\cargo.exe"
$possibleCargo += "C:\Program Files\Rust stable MSVC\bin\cargo.exe"
$possibleCargo += "C:\Program Files\Rust\bin\cargo.exe"

$cargo = $possibleCargo | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $cargo) {
    Write-Host 'Cargo wurde nicht gefunden.' -ForegroundColor Red
    Write-Host 'Bitte starte ein neues Terminal oder installiere Rust erneut.'
    Write-Host 'Alternativ:'
    Write-Host "  & 'C:\Users\Soliak 2. PC\.cargo\bin\cargo.exe' build --release"
    exit 1
}

Set-Location $scriptDir
Write-Host "Benutze Cargo: $cargo"
& $cargo build --release
