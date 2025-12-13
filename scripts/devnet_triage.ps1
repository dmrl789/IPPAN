param(
  [string]$User="ippan-devnet",
  [string[]]$Hosts=@("188.245.97.41","135.181.145.174","5.223.51.238","178.156.219.107")
)
$ErrorActionPreference="Stop"
$ts=Get-Date -Format "yyyyMMdd_HHmmss"
$root="tmp/devnet/triage_$ts"
New-Item -ItemType Directory -Force -Path $root | Out-Null

function Ssh([string]$h,[string]$cmd){
  try {
    $result = ssh -o StrictHostKeyChecking=accept-new "$User@$h" $cmd
    return $result
  } catch {
    return $_.Exception.Message
  }
}

foreach($h in $Hosts){
  $dir=Join-Path $root $h.Replace(".","_")
  New-Item -ItemType Directory -Force -Path $dir | Out-Null

  Ssh $h "hostname; date -u; echo ---; sudo -n systemctl is-active ippan-node; sudo -n systemctl status ippan-node --no-pager -l | tail -n 80" | Set-Content -Encoding UTF8 (Join-Path $dir "systemd.txt")
  Ssh $h "curl -fsS http://127.0.0.1:8080/status || true" | Set-Content -Encoding UTF8 (Join-Path $dir "status_8080.json")
  Ssh $h "curl -fsS http://127.0.0.1:8080/p2p/peers || true" | Set-Content -Encoding UTF8 (Join-Path $dir "p2p_peers_8080.json")
  Ssh $h "curl -fsS http://127.0.0.1:9000/p2p/peers || true" | Set-Content -Encoding UTF8 (Join-Path $dir "p2p_peers_9000.txt")
  Ssh $h "sudo -n journalctl -u ippan-node -n 250 --no-pager" | Set-Content -Encoding UTF8 (Join-Path $dir "journal_tail.txt")
  Ssh $h "ss -lntp || true; echo ---; ss -lnup || true" | Set-Content -Encoding UTF8 (Join-Path $dir "listeners.txt")
  Ssh $h "if command -v ufw >/dev/null 2>&1; then sudo -n ufw status numbered; else echo 'ufw not installed'; fi" | Set-Content -Encoding UTF8 (Join-Path $dir "ufw.txt")
  Ssh $h "ls -la /var/lib/ippan 2>/dev/null || true; ls -la /var/lib/ippan/*.pid 2>/dev/null || true" | Set-Content -Encoding UTF8 (Join-Path $dir "state_dir.txt")
}

Write-Host "TRIAGE saved to: $root" -ForegroundColor Green

