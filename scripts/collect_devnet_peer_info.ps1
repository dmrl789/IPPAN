param(
  [string]$User = "ippan-devnet",
  [string[]]$Hosts = @("188.245.97.41","135.181.145.174","5.223.51.238","178.156.219.107")
)

$ErrorActionPreference = "Stop"
$stamp = Get-Date -Format "yyyyMMdd_HHmmss"
$outDir = Join-Path "tmp/devnet" "collect_$stamp"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

function RunSsh([string]$hostname, [string]$cmd, [string]$outFile) {
  Write-Host "[$hostname] -> $outFile"
  $result = & ssh -o StrictHostKeyChecking=accept-new "$User@$hostname" "bash -c `"$cmd`"" 2>&1
  $result | Out-File -Encoding utf8 $outFile
}

# Remote script: gather status, listeners, ufw, systemd unit, env, and grep for bootstrap/peers.
$remote = @'
set -e
echo "=== META ==="
date -u || true
hostname || true
uname -a || true
echo

echo "=== IP ==="
ip -br a || true
echo

echo "=== LISTENERS (8080/9000) ==="
sudo -n ss -lntup | egrep ":(8080|9000)\b" || true
sudo -n ss -lnup  | egrep ":(9000)\b" || true
echo

echo "=== UFW ==="
if command -v ufw >/dev/null 2>&1; then sudo -n ufw status verbose || true; else echo "ufw: not installed"; fi
echo

echo "=== SERVICE STATUS ==="
sudo -n systemctl is-active ippan-node || true
sudo -n systemctl status ippan-node --no-pager -n 30 || true
echo

echo "=== SYSTEMD UNIT (includes drop-ins) ==="
sudo -n systemctl cat ippan-node || true
echo

echo "=== ENV / ARGS HINTS ==="
# Try common env files; ignore if missing
for f in /etc/default/ippan-node /etc/ippan/env /etc/ippan/*.env /etc/ippan/config/*.env; do
  [ -e "$f" ] && { echo "--- $f ---"; sudo -n cat "$f"; echo; } || true
done
echo

echo "=== LOCAL STATUS (127.0.0.1) ==="
curl -fsS http://127.0.0.1:8080/status || true
echo

echo "=== PEER/BOOTSTRAP CONFIG GREP ==="
paths="/etc/ippan /etc/ippan/config /opt/ippan /opt/ippan/config /var/lib/ippan"
for p in $paths; do
  [ -d "$p" ] || continue
  echo "# scan $p"
  sudo -n grep -RIn --include="*.toml" --include="*.yaml" --include="*.yml" --include="*.json" \
    "bootstrap|bootnode|seed|seeds|peer|peers|multiaddr|listen|p2p" "$p" 2>/dev/null | head -n 200 || true
done
echo

echo "=== LOGS (peer/connect/dial/listen keywords, last 500 lines) ==="
sudo -n journalctl -u ippan-node -n 500 --no-pager | egrep -i "peer|connect|dial|listen|bootstrap|discov|swarm|handshake|identify|gossip|multiaddr|p2p" || true
echo

echo "=== LOGS RAW (last 200 lines) ==="
sudo -n journalctl -u ippan-node -n 200 --no-pager || true
'@

# Collect main bundle per host
foreach($h in $Hosts){
  $safe = $h.Replace(".","_")
  $dir = Join-Path $outDir $safe
  New-Item -ItemType Directory -Force -Path $dir | Out-Null

  RunSsh $h $remote (Join-Path $dir "bundle.txt")

  # Extract candidate config files from grep output and pull first 220 lines of each (helps find exact keys).
  $bundlePath = Join-Path $dir "bundle.txt"
  $grepLines = Select-String -Path $bundlePath -Pattern "^\S+:\d+:" -ErrorAction SilentlyContinue | ForEach-Object { $_.Line }
  $files = @()
  foreach($ln in $grepLines){
    $parts = $ln.Split(":")
    if($parts.Length -ge 2){
      $f = $parts[0]
      if($f.StartsWith("/")){ $files += $f }
    }
  }
  $files = $files | Sort-Object -Unique
  if($files.Count -gt 0){
    $listFile = Join-Path $dir "config_files.txt"
    $files | Out-File -Encoding utf8 $listFile
    foreach($f in $files){
      $name = ($f.TrimStart("/").Replace("/","__").Replace(":","_"))
      $out = Join-Path $dir ("cfg__" + $name + ".head.txt")
      RunSsh $h "sudo -n sed -n '1,220p' '$f' || true" $out
    }
  }

  # Also capture full unit drop-in directory listing (for pidfile cleanup etc.)
  RunSsh $h "sudo -n ls -la /etc/systemd/system/ippan-node.service.d 2>/dev/null || true" (Join-Path $dir "unit_dropins_ls.txt")
}

# Summary file
$summary = Join-Path $outDir "SUMMARY.txt"
@"
Collected devnet evidence into:

$outDir

What to paste back to ChatGPT (minimal):

- node1 bundle.txt

- node2 bundle.txt

- node3 bundle.txt

- node4 bundle.txt

- any cfg__*.head.txt that mentions bootstrap/peers/multiaddr/listen/p2p
"@ | Out-File -Encoding utf8 $summary

Write-Host "`nDONE. Folder: $outDir"
Write-Host "Next: open node1 + node4 bundle.txt and paste the sections showing:"
Write-Host "- peer_id / listen addr / multiaddr"
Write-Host "- bootstrap/peers config lines"
Write-Host "- any dial/connect errors"

