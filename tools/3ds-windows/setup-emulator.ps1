# Installs the Azahar 3DS emulator (winget) and prepares its virtual SD card with OpenSGolf's free asset set.
# Run from this folder:   .\setup-emulator.ps1
$ErrorActionPreference = "Continue"
$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path

# 1. Azahar
$exe = Get-ChildItem -Path "$env:LOCALAPPDATA\Programs","$env:ProgramFiles","${env:ProgramFiles(x86)}" -Recurse -Filter azahar.exe -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $exe) {
    Write-Host "== Azahar via winget =="
    winget install --id AzaharEmulator.Azahar.MSVC -e --accept-source-agreements --accept-package-agreements
    if ($LASTEXITCODE -ne 0) { winget install --id AzaharEmu.Azahar -e --accept-source-agreements --accept-package-agreements }
    $exe = Get-ChildItem -Path "$env:LOCALAPPDATA\Programs","$env:ProgramFiles","${env:ProgramFiles(x86)}" -Recurse -Filter azahar.exe -ErrorAction SilentlyContinue | Select-Object -First 1
}
if ($exe) { Write-Host "Azahar: $($exe.FullName)" } else { Write-Host "azahar.exe not found after install; run 'winget search azahar' and tell Claude the id" -ForegroundColor Yellow }

# 2. The free asset set, made on the PC (make-free-set.ps1), onto Azahar's virtual SD card
$free = Join-Path $PSScriptRoot "free"
if (-not (Test-Path (Join-Path $free "Data"))) {
    Write-Host "== making the free asset set first =="
    & (Join-Path $PSScriptRoot "make-free-set.ps1")
}
$sd = Join-Path $env:APPDATA "Azahar\sdmc\3ds\opensgolf"
New-Item -ItemType Directory -Force (Join-Path $sd "free") | Out-Null
Write-Host "== copying the free set to the emulator's SD card: $sd\free =="
robocopy $free (Join-Path $sd "free") /E /NFL /NDL /NJH /NJS /NP | Out-Null
Write-Host ""
Write-Host "Emulator ready. Build with .\build-3ds.ps1, then start it with .\run-emulator.ps1"
Write-Host "Your own SimGolf files (optional): copy the folder with Data, Flics, Sounds to $sd\game"
