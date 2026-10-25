# Builds OpenSGolf's 3DS frontend. Run in PowerShell from this folder after setup-3ds.ps1:
#   Set-ExecutionPolicy -Scope Process Bypass; .\build-3ds.ps1          (full build)
#   .\build-3ds.ps1 check                                                (just check that sg-core compiles for the 3DS)
# The result, simgolf-3ds.3dsx, is copied next to this script. Build output goes outside OneDrive (it is large and churns).
param([string]$Mode = "build")
# cargo writes warnings to stderr; "Continue" keeps PowerShell from treating those as failures
$ErrorActionPreference = "Continue"
$log = Join-Path $PSScriptRoot "build-log.txt"
$env:DEVKITPRO = "C:/devkitPro"
$env:DEVKITARM = "C:/devkitPro/devkitARM"
$env:Path = "C:\devkitPro\devkitARM\bin;C:\devkitPro\tools\bin;$env:USERPROFILE\.cargo\bin;" + $env:Path
$env:CARGO_TARGET_DIR = Join-Path $env:LOCALAPPDATA "simgolf-3ds-target"
# the host toolchain for cargo-3ds and the build scripts (MSVC; Visual Studio Build Tools from setup-3ds.ps1)
$env:RUSTUP_TOOLCHAIN = "nightly"
# ctru-sys runs bindgen over libctru's headers at build time, which needs libclang (LLVM; setup-3ds.ps1 installs it)
if (-not $env:LIBCLANG_PATH) {
    foreach ($c in @("C:\Program Files\LLVM\bin", "C:\Program Files (x86)\LLVM\bin")) {
        if (Test-Path (Join-Path $c "libclang.dll")) { $env:LIBCLANG_PATH = $c; break }
    }
}
if (-not $env:LIBCLANG_PATH) { Write-Host "LLVM (libclang) not found: run .\setup-3ds.ps1 again, it installs it." -ForegroundColor Yellow }
Push-Location (Join-Path $PSScriptRoot "opensgolf")
try {

if ($Mode -eq "triangle") {
    # the stock citro3d-rs triangle example: a known-good draw to compare the console against
    Pop-Location
    Push-Location (Join-Path $PSScriptRoot "opensgolf\vendor\citro3d-rs")
    & "C:\devkitPro\tools\bin\picasso.exe" -o citro3d\examples\assets\vshader.shbin citro3d\examples\assets\vshader.pica
    cargo +nightly 3ds build --release -p citro3d --example triangle 2>&1 | Tee-Object -FilePath $log
    if ($LASTEXITCODE -ne 0) { Write-Host "Build failed; the full log is in build-log.txt" -ForegroundColor Yellow; exit $LASTEXITCODE }
    Copy-Item (Join-Path $env:CARGO_TARGET_DIR "armv6k-nintendo-3ds\release\examples\triangle.3dsx") (Join-Path $PSScriptRoot "triangle.3dsx") -Force
    Write-Host "Built: $PSScriptRoot\triangle.3dsx"
    exit 0
}
if ($Mode -eq "check") {
    cargo +nightly 3ds check -p sg-core 2>&1 | Tee-Object -FilePath $log
    exit $LASTEXITCODE
}
cargo +nightly 3ds build --release -p simgolf-3ds 2>&1 | Tee-Object -FilePath $log
if ($LASTEXITCODE -ne 0) { Write-Host "Build failed; the full log is in build-log.txt" -ForegroundColor Yellow; exit $LASTEXITCODE }
$out = Join-Path $env:CARGO_TARGET_DIR "armv6k-nintendo-3ds\release\simgolf-3ds.3dsx"
Copy-Item $out (Join-Path $PSScriptRoot "simgolf-3ds.3dsx") -Force
Write-Host ""
Write-Host "Built: $PSScriptRoot\simgolf-3ds.3dsx  (open it in Azahar, or copy to the SD card's 3ds folder)"
} finally { Pop-Location }
