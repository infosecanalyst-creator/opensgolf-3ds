# One-time setup for building OpenSGolf for the 3DS on Windows. Run in PowerShell (no admin needed):
#   Set-ExecutionPolicy -Scope Process Bypass; .\setup-3ds.ps1
# It installs Rust (rustup + nightly + the std sources the 3DS target needs) and cargo-3ds, then opens the devkitPro
# installer, which you click through yourself: choose "3DS Development" when it asks what to install.
$ErrorActionPreference = "Stop"
$dl = Join-Path $env:TEMP "simgolf-3ds-setup"
New-Item -ItemType Directory -Force $dl | Out-Null

# 1. Rust
if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    Write-Host "== Installing Rust (rustup) =="
    $rustup = Join-Path $dl "rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustup
    & $rustup -y --default-toolchain stable --profile minimal
    $env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path
}
# 2a. Visual Studio Build Tools, C++ workload: the MSVC linker Rust on Windows uses (about 2 GB, unattended, 5-10 min)
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$hasVc = (Test-Path $vswhere) -and ((& $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath) -ne $null)
if (-not $hasVc) {
    Write-Host "== Visual Studio Build Tools (C++ workload) via winget: this one takes a while =="
    winget install --id Microsoft.VisualStudio.2022.BuildTools -e --accept-source-agreements --accept-package-agreements --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
}

Write-Host "== Rust nightly + rust-src (the 3DS target builds its own std) =="
rustup toolchain install nightly --profile minimal -c rust-src

# 2. cargo-3ds
if (-not (Test-Path "$env:USERPROFILE\.cargo\bin\cargo-3ds.exe")) {
    Write-Host "== cargo-3ds =="
    cargo +nightly install --git https://github.com/rust3ds/cargo-3ds
}

# 2b. LLVM: libclang for bindgen (the 3DS OS bindings are generated from libctru's C headers at build time)
if (-not (Test-Path "C:\Program Files\LLVM\bin\libclang.dll")) {
    Write-Host "== LLVM (libclang, for bindgen) via winget =="
    winget install --id LLVM.LLVM -e --accept-source-agreements --accept-package-agreements
}

# 3. devkitPro (devkitARM, picasso, 3dslink): a GUI installer
if (-not (Test-Path "C:\devkitPro\devkitARM\bin\arm-none-eabi-gcc.exe")) {
    Write-Host "== devkitPro installer =="
    $rel = Invoke-RestMethod "https://api.github.com/repos/devkitPro/installer/releases/latest"
    $asset = $rel.assets | Where-Object { $_.name -like "devkitProUpdater*.exe" } | Select-Object -First 1
    $inst = Join-Path $dl $asset.name
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $inst
    Write-Host "Starting $($asset.name): keep the default C:\devkitPro, pick '3DS Development', finish the wizard."
    Start-Process -FilePath $inst -Wait
}
Write-Host ""
Write-Host "Setup done. Close this PowerShell window, open a new one, and run .\build-3ds.ps1"
