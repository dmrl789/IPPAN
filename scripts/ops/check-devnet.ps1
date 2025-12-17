<#
.SYNOPSIS
  Devnet verifier: checks /status, /peers count, /time, and ippan-node sha256 across all devnet nodes.

.DESCRIPTION
  - HTTP checks hit RPC on port 8080 (default devnet config).
  - Binary check uses SSH to run sha256sum on /usr/local/bin/ippan-node.
  - Exits non-zero if any node is unhealthy or hashes differ.

.REQUIRES
  - PowerShell 5+ (or PowerShell 7+)
  - ssh in PATH (OpenSSH)
#>

$ErrorActionPreference = "Stop"

$RpcPort = 8080
$Nodes = @(
  @{ Name = "Node 1 (Nuremberg)"; Ip = "188.245.97.41" },
  @{ Name = "Node 2 (Helsinki)";  Ip = "135.181.145.174" },
  @{ Name = "Node 3 (Singapore)"; Ip = "5.223.51.238" },
  @{ Name = "Node 4 (Ashburn)";   Ip = "178.156.219.107" }
)

function Get-Json($Uri) {
  Invoke-RestMethod -Uri $Uri -Method Get -TimeoutSec 8
}

function Get-RemoteSha256($Ip) {
  $cmd = "sha256sum /usr/local/bin/ippan-node 2>/dev/null | awk '{print `$1}'"
  $out = & ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new ("root@$Ip") $cmd 2>$null
  return ($out | Select-Object -First 1).Trim()
}

$results = @()
$failed = $false

foreach ($n in $Nodes) {
  $ip = $n.Ip
  $name = $n.Name
  Write-Host ("=== {0} {1} ===" -f $name, $ip)

  try {
    $status = Get-Json ("http://${ip}:${RpcPort}/status")
    $peers  = Get-Json ("http://${ip}:${RpcPort}/peers")
    $time   = Get-Json ("http://${ip}:${RpcPort}/time")
    $sha    = Get-RemoteSha256 $ip

    $peerCount = 0
    if ($null -ne $peers) {
      if ($peers -is [System.Array]) { $peerCount = $peers.Count } else { $peerCount = 1 }
    }

    $row = [pscustomobject]@{
      ip          = $ip
      status      = $status.status
      peer_count  = $peerCount
      version     = $status.version
      node_id     = $status.node_id
      uptime_s    = $status.uptime_seconds
      time_us     = $time.time_us
      sha256      = $sha
    }
    $results += $row

    $ok = ($row.status -eq "ok") -and ($row.peer_count -ge 1) -and ($row.time_us -ne $null) -and ($row.sha256 -match "^[0-9a-fA-F]{64}$")
    if (-not $ok) { $failed = $true }

    $row | ConvertTo-Json -Compress
  } catch {
    $failed = $true
    Write-Host ("ERROR {0} {1}: {2}" -f $name, $ip, $_.Exception.Message)
  }
}

$uniqueHashes = @($results | Where-Object { $_.sha256 } | Select-Object -ExpandProperty sha256 | Sort-Object -Unique)

Write-Host "=== SUMMARY ==="
Write-Host ("nodes={0} unique_hashes={1}" -f $results.Count, $uniqueHashes.Count)
if ($uniqueHashes.Count -gt 0) { Write-Host ("sha256={0}" -f $uniqueHashes[0]) }

if ($uniqueHashes.Count -ne 1) {
  Write-Host "FAIL: sha256 mismatch across nodes"
  exit 2
}

if ($failed) {
  Write-Host "FAIL: one or more node checks failed"
  exit 1
}

Write-Host "OK: all nodes healthy and binary hashes match"
exit 0


