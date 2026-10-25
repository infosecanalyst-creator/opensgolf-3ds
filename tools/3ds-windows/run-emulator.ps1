# Starts Azahar with the latest build. Run from this folder:   .\run-emulator.ps1
# The emulator's log (the program's own messages, and any crash) is at %APPDATA%\Azahar\log\azahar_log.txt
$exe = Get-ChildItem -Path "$env:LOCALAPPDATA\Programs","$env:ProgramFiles","${env:ProgramFiles(x86)}" -Recurse -Filter azahar.exe -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $exe) { Write-Host "Azahar not installed: run .\setup-emulator.ps1" -ForegroundColor Yellow; exit 1 }
$app = Join-Path $PSScriptRoot "simgolf-3ds.3dsx"
if (-not (Test-Path $app)) { Write-Host "No simgolf-3ds.3dsx yet: run .\build-3ds.ps1" -ForegroundColor Yellow; exit 1 }
$log = Join-Path $env:APPDATA "Azahar\log\azahar_log.txt"
if (Test-Path $log) { Remove-Item $log -Force }
Start-Process -FilePath $exe.FullName -ArgumentList "`"$app`""
Write-Host "Started. Log: $log"
