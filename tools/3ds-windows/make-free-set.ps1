# Generates OpenSGolf's free asset set (opensgolf-free) on the PC, so the 3DS never has to draw its 5,465 files itself.
# Run from this folder:   .\make-free-set.ps1
# Then copy the resulting "free" folder to the SD card as  3ds\opensgolf\free  (so that 3ds\opensgolf\free\Data exists).
$ErrorActionPreference = "Continue"
$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path
$env:CARGO_TARGET_DIR = Join-Path $env:LOCALAPPDATA "simgolf-3ds-target"
$out = Join-Path $PSScriptRoot "free"
Push-Location (Join-Path $PSScriptRoot "opensgolf")
try {
    cargo build --release -p sgtool
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $tool = Join-Path $env:CARGO_TARGET_DIR "release\sgtool.exe"
    if (Test-Path $out) { Remove-Item -Recurse -Force $out }
    & $tool freeset $out
    Write-Host ""
    Write-Host "Written: $out   -> copy this folder to the SD card as 3ds\opensgolf\free"
} finally { Pop-Location }
