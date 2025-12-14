param(
  [string]$Inventory = "ops/nodes/devnet_nodes.txt",
  [string]$Branch = "master",
  [int]$SshPort = 22,
  [switch]$SkipGitPull,
  [switch]$SkipRestart
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Read-Inventory([string]$path) {
  if (!(Test-Path $path)) { throw "Inventory not found: $path" }
  Get-Content $path |
    ForEach-Object { $_.Trim() } |
    Where-Object { $_ -and -not $_.StartsWith("#") }
}

function Run-Ssh([string]$target, [string]$cmd) {
  $args = @(
    "-p", "$SshPort",
    "-o", "StrictHostKeyChecking=accept-new",
    $target,
    "bash", "-lc", $cmd
  )
  $out = & ssh @args 2>&1
  return $out
}

# What we verify on nodes:
# - dlc.toml contains the pinned model path + hash (we only *read* it here)
# - /status returns consensus.metrics_available + peer_count
# - node is running (systemctl is-active)
$nodes = Read-Inventory $Inventory
if ($nodes.Count -eq 0) { throw "Inventory is empty: $Inventory" }

Write-Host "Inventory: $Inventory"
Write-Host "Nodes: $($nodes.Count)"
Write-Host "Branch: $Branch"
Write-Host "SshPort: $SshPort"
Write-Host "SkipGitPull: $SkipGitPull"
Write-Host "SkipRestart: $SkipRestart"
Write-Host ""

$results = @()

foreach ($n in $nodes) {
  Write-Host "=== $n ==="

  try {
    if (-not $SkipGitPull) {
      $pullCmd = @"
set -euo pipefail
cd /root/IPPAN 2>/dev/null || cd /opt/IPPAN 2>/dev/null || cd ~/IPPAN
git fetch origin
git checkout $Branch
git pull --ff-only origin $Branch
"@
      Run-Ssh $n $pullCmd | Out-Host
    }

    if (-not $SkipRestart) {
      $restartCmd = @"
set -euo pipefail
sudo systemctl restart ippan-node
sleep 2
sudo systemctl is-active ippan-node
"@
      Run-Ssh $n $restartCmd | Out-Host
    }

    # Read dlc.toml (path may vary; include common locations)
    $cfgCmd = @"
set -euo pipefail
for p in /etc/ippan/config/dlc.toml /etc/ippan/dlc.toml ./config/dlc.toml; do
  if [ -f "\$p" ]; then
    echo "DLC_TOML=\$p"
    sed -n '1,200p' "\$p"
    exit 0
  fi
done
echo "DLC_TOML=NOT_FOUND"
exit 0
"@
    $cfgOut = Run-Ssh $n $cfgCmd

    # Query status (best-effort)
    $statusCmd = @"
set -euo pipefail
curl -s --max-time 5 http://127.0.0.1:8080/status || true
"@
    $statusOut = Run-Ssh $n $statusCmd

    # Parse fields in PowerShell (avoid jq dependency)
    $metricsAvailable = $false
    $peerCount = ""
    $hashSeen = $false

    try {
      if ($statusOut.Trim().StartsWith("{")) {
        $json = $statusOut | ConvertFrom-Json
        $metricsAvailable = [bool]$json.consensus.metrics_available
        $peerCount = "$($json.peer_count)"
      }
    } catch {
      $peerCount = "parse_error"
    }

    # “hash seen” means dlc.toml contains a model_hash line (pinned)
    if ($cfgOut -match "model_hash\s*=") { $hashSeen = $true }

    $results += [pscustomobject]@{
      Node = $n
      HashSeen = if ($hashSeen) { "yes" } else { "no" }
      MetricsAvailable = if ($metricsAvailable) { "yes" } else { "no" }
      PeerCount = if ($peerCount) { $peerCount } else { "unknown" }
    }

    Write-Host "OK: HashSeen=$hashSeen MetricsAvailable=$metricsAvailable PeerCount=$peerCount"
    Write-Host ""
  } catch {
    $results += [pscustomobject]@{
      Node = $n
      HashSeen = "error"
      MetricsAvailable = "error"
      PeerCount = "error"
    }
    Write-Host "ERROR on $n:"
    Write-Host $_.Exception.Message
    Write-Host ""
  }
}

Write-Host ""
Write-Host "==== SUMMARY ===="
$results | Format-Table -AutoSize

# Exit non-zero if any error rows exist
if (($results | Where-Object { $_.HashSeen -eq "error" -or $_.MetricsAvailable -eq "error" }).Count -gt 0) {
  exit 1
}