# Sends the build to the console over Wi-Fi and shows its output live (no SD card swapping).
# On the console: Homebrew Launcher -> press Y (NetLoader) -> it shows "Waiting for 3dslink connection" and its IP.
# Then run this script; pass the IP if autodetection fails:   .\run-console.ps1 192.168.1.50
# Add -Log to also save the output to console-log.txt (3dslink then holds its output back until the game exits:
# it buffers when its output goes to a file).
param([string]$Ip = "", [switch]$Log)
$exe = "C:\devkitPro\tools\bin\3dslink.exe"
if (-not (Test-Path $exe)) { Write-Host "3dslink.exe not found at $exe" -ForegroundColor Yellow; exit 1 }
$file = Join-Path $PSScriptRoot "simgolf-3ds.3dsx"
$args3 = @("-s", $file)
if ($Ip) { $args3 = @("-a", $Ip) + $args3 }
if ($Log) {
    & $exe @args3 2>&1 | Tee-Object -FilePath (Join-Path $PSScriptRoot "console-log.txt")
} else {
    & $exe @args3
}
