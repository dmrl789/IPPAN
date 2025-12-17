# Canary-first devnet rollout for ippan-node + HTTP-only health verification.
#
# Requirements (Windows):
# - OpenSSH (ssh)
# - curl is optional; this script uses Invoke-RestMethod
#
# Verifies:
# - /status.status == ok
# - /status.peer_count == 4
# - /status.dataset_export.enabled == true
# - /status.dataset_export.last_age_seconds <= 28800
# - /time monotonic over a short sample window
# - build_sha consistent across all nodes (single unique)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RpcPort = $env:RPC_PORT
if (-not $RpcPort) { $RpcPort = 8080 }
$ExpectedPeers = $env:EXPECTED_PEERS
if (-not $ExpectedPeers) { $ExpectedPeers = 4 }
$MaxDatasetAgeSeconds = $env:MAX_DATASET_AGE_SECONDS
if (-not $MaxDatasetAgeSeconds) { $MaxDatasetAgeSeconds = 28800 }
$TimeSamples = $env:TIME_SAMPLES
if (-not $TimeSamples) { $TimeSamples = 10 }

$Canary = "5.223.51.238"
$Others = @("188.245.97.41","135.181.145.174","178.156.219.107")

function Get-Status([string]$ip) {
  Invoke-RestMethod -Uri ("http://{0}:{1}/status" -f $ip,$RpcPort) -TimeoutSec 8
}

function Assert-TimeMonotonic([string]$ip) {
  $prev = $null
  for ($i=0; $i -lt [int]$TimeSamples; $i++) {
    $t = (Invoke-RestMethod -Uri ("http://{0}:{1}/time" -f $ip,$RpcPort) -TimeoutSec 8).time_us
    if ($null -eq $t) { throw "node=$ip time_us missing" }
    if ($prev -ne $null -and [int64]$t -lt [int64]$prev) { throw "node=$ip time not monotonic (prev=$prev now=$t)" }
    $prev = $t
    Start-Sleep -Milliseconds 200
  }
}

function Verify-NodeHttp([string]$ip) {
  $s = Get-Status $ip
  if ($s.status -ne "ok") { throw "node=$ip status != ok" }
  if ([int]$s.peer_count -ne [int]$ExpectedPeers) { throw "node=$ip peer_count != $ExpectedPeers (got $($s.peer_count))" }
  if ($null -eq $s.dataset_export) { throw "node=$ip dataset_export missing" }
  if ($s.dataset_export.enabled -ne $true) { throw "node=$ip dataset_export.enabled != true" }
  if ($null -eq $s.dataset_export.last_age_seconds) { throw "node=$ip dataset_export.last_age_seconds missing" }
  if ([int]$s.dataset_export.last_age_seconds -gt [int]$MaxDatasetAgeSeconds) { throw "node=$ip dataset_export stale: $($s.dataset_export.last_age_seconds)s" }
  Assert-TimeMonotonic $ip
  return $s.build_sha
}

function Deploy-Node([string]$ip) {
  Write-Host "=== DEPLOY $ip ==="

  # Harden exporter unit (best-effort)
  ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new ("root@{0}" -f $ip) @"
set -euo pipefail
cat >/etc/systemd/system/ippan-export-dataset.service <<'EOF'
[Unit]
Description=IPPAN Devnet Dataset Export (D-GBDT telemetry)
After=network-online.target ippan-node.service
Wants=network-online.target

[Service]
Type=oneshot
User=root
Nice=10
IOSchedulingClass=best-effort
IOSchedulingPriority=7
TimeoutStartSec=900
WorkingDirectory=/root/IPPAN
StandardOutput=journal
StandardError=journal
ExecStart=/usr/local/lib/ippan/export-dataset.sh
EOF
systemctl daemon-reload
"@ | Out-Null

  # Update + build as ippan-devnet
  ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new ("root@{0}" -f $ip) @"
set -euo pipefail
sudo -u ippan-devnet -H bash -lc 'set -euo pipefail
  git config --global --add safe.directory /opt/ippan || true
  cd /opt/ippan
  git fetch origin
  git checkout master
  git pull --rebase origin master
  cargo build -p ippan-node --release
'
"@

  # Stop → install → start
  ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new ("root@{0}" -f $ip) @"
set -euo pipefail
TS=\$(date -u +%Y%m%dT%H%M%SZ)
cp -a /usr/local/bin/ippan-node /usr/local/bin/ippan-node.bak.\${TS} || true
systemctl stop ippan-node
sleep 1
install -m 0755 /opt/ippan/target/release/ippan-node /usr/local/bin/ippan-node
systemctl start ippan-node
systemctl status ippan-node --no-pager | head -n 8
"@
}

Write-Host "=== CANARY FIRST: $Canary ==="
Deploy-Node $Canary
$canarySha = Verify-NodeHttp $Canary
Write-Host "CANARY OK build_sha=$canarySha"

foreach ($ip in $Others) {
  Deploy-Node $ip
  $sha = Verify-NodeHttp $ip
  if ($sha -ne $canarySha) { throw "build_sha drift: canary=$canarySha node=$ip sha=$sha" }
}

Write-Host "OK: rollout complete (build_sha=$canarySha)"


