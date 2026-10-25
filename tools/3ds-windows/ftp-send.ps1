# Sends the latest build to the 3DS over ftpd: saves the old logs here, moves the old build and logs to /_to_delete/simgolf-old-<time>/
$out = "$PSScriptRoot\ftp-out.txt"
$ip = (Get-Content "$PSScriptRoot\ftp-ip.txt").Trim()
$u = "ftp://$ip`:5000"
"send to $ip $(Get-Date)" | Set-Content $out
& curl.exe -s --max-time 20 -o "$PSScriptRoot\console-panic.txt" "$u/3ds/opensgolf/panic.txt" 2>&1 | Add-Content $out
& curl.exe -s --max-time 20 -o "$PSScriptRoot\console-stage.txt" "$u/3ds/opensgolf/stage.txt" 2>&1 | Add-Content $out
$stamp = Get-Date -Format 'yyyyMMdd-HHmm'
$d = "/_to_delete/simgolf-old-$stamp"
& curl.exe -s --max-time 30 "$u/" -Q "*MKD $d" -Q "*MKD $d/opensgolf" `
  -Q "*RNFR /3ds/simgolf-3ds.3dsx" -Q "*RNTO $d/simgolf-3ds.3dsx" `
  -Q "*RNFR /3ds/opensgolf/panic.txt" -Q "*RNTO $d/opensgolf/panic.txt" `
  -Q "*RNFR /3ds/opensgolf/stage.txt" -Q "*RNTO $d/opensgolf/stage.txt" -o NUL 2>&1 | Add-Content $out
# (the texture cache in /3ds/opensgolf/cache stays: it only depends on the game files, and rebuilding it slows the first start)
"moves exit $LASTEXITCODE" | Add-Content $out
& curl.exe -s --max-time 300 -T "$PSScriptRoot\simgolf-3ds.3dsx" "$u/3ds/simgolf-3ds.3dsx" 2>&1 | Add-Content $out
"upload exit $LASTEXITCODE (local $((Get-Item "$PSScriptRoot\simgolf-3ds.3dsx").Length) bytes)" | Add-Content $out
"== /3ds/ (simgolf)" | Add-Content $out
& curl.exe -s --max-time 15 "$u/3ds/" 2>&1 | Select-String -Pattern 'simgolf' | Add-Content $out
"== /3ds/opensgolf/" | Add-Content $out
& curl.exe -s --max-time 15 --list-only "$u/3ds/opensgolf/" 2>&1 | Add-Content $out
"== $d" | Add-Content $out
& curl.exe -s --max-time 15 "$u$d/" 2>&1 | Add-Content $out
& curl.exe -s --max-time 15 "$u$d/opensgolf/" 2>&1 | Add-Content $out
"DONE" | Add-Content $out
