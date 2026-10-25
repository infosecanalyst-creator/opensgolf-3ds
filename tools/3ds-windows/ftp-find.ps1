# Finds the 3DS running ftpd (port 5000) on this PC's network and lists its SimGolf files into ftp-out.txt.
$out = "$PSScriptRoot\ftp-out.txt"
"started $(Get-Date)" | Set-Content $out
$ips = Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.IPAddress -notlike '127.*' -and $_.IPAddress -notlike '169.254*' -and $_.PrefixOrigin -ne 'WellKnown' }
$found = $null
foreach ($ip in $ips) {
  $base = ($ip.IPAddress -split '\.')[0..2] -join '.'
  "scanning $base.0/24" | Add-Content $out
  $tasks = @{}
  foreach ($i in 1..254) { $c = New-Object System.Net.Sockets.TcpClient; $tasks["$base.$i"] = @($c, $c.ConnectAsync("$base.$i", 5000)) }
  Start-Sleep -Milliseconds 1500
  foreach ($k in $tasks.Keys) {
    $t = $tasks[$k]
    if ($t[1].Status -eq 'RanToCompletion' -and $t[0].Connected) {
      # an FTP server greets with 220 at once; the router's port 5000 does not
      $st = $t[0].GetStream(); $st.ReadTimeout = 1500; $buf = New-Object byte[] 256
      try { $n = $st.Read($buf, 0, 256); $greet = [Text.Encoding]::ASCII.GetString($buf, 0, $n) } catch { $greet = '' }
      "open $k : $($greet.Trim())" | Add-Content $out
      if ($greet -like '220*') { $found = $k }
    }
    $t[0].Close()
  }
  if ($found) { break }
}
if (-not $found) { "NOT FOUND" | Add-Content $out; exit 1 }
"FOUND $found" | Add-Content $out
$found | Set-Content "$PSScriptRoot\ftp-ip.txt"
foreach ($d in @('/3ds/', '/3ds/opensgolf/', '/')) {
  "== $d" | Add-Content $out
  & curl.exe -s --max-time 15 --list-only "ftp://$found`:5000$d" 2>&1 | Add-Content $out
}
"DONE" | Add-Content $out
