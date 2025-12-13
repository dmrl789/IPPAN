# Export localnet dataset on node -> SCP back -> store timestamped CSV under ai_assets/datasets/localnet/
# Usage:
#   .\scripts\export-localnet-dataset.ps1 -Node root@188.245.97.41
# Optional:
#   -Samples 120 -IntervalSec 5 -Rpc http://127.0.0.1:8080 -Commit

[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$Node,

  [string]$RepoRoot = (Get-Location).Path,

  [string]$Rpc = "http://127.0.0.1:8080",

  [int]$Samples = 120,

  [int]$IntervalSec = 5,

  [switch]$Commit
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Require-Command([string]$cmd) {
  if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
    throw "Missing required command: $cmd"
  }
}

Require-Command ssh
Require-Command scp

# Ensure we're in a repo with the expected folder
$datasetDir = Join-Path $RepoRoot "ai_assets\datasets\localnet"
New-Item -ItemType Directory -Force -Path $datasetDir | Out-Null

# Timestamp format: YYYYMMDDTHHMMSSZ (UTC)
$ts = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
$localCsv = Join-Path $datasetDir "localnet_${ts}.csv"

# Remote paths
$remoteRepo = "/root/IPPAN"
$remoteCsv = "/tmp/localnet_${ts}.csv"
$remoteScript = "$remoteRepo/ai_training/export_localnet_dataset.py"

Write-Host "=== 1) Export on node: $Node ==="
# Note: exporter defaults to 120 samples / 5s in your logs; we pass explicit flags if supported
# If flags aren't supported in your version, it will still run with its defaults and write the file.
$remoteCmd = @"
set -euo pipefail
cd "$remoteRepo"
if [ ! -f "$remoteScript" ]; then
  echo "ERROR: exporter not found at $remoteScript" >&2
  exit 1
fi
python3 "$remoteScript" --rpc "$Rpc" --out "$remoteCsv" --samples "$Samples" --interval "$IntervalSec" || \
python3 "$remoteScript" --rpc "$Rpc" --out "$remoteCsv"
echo "=== Remote CSV ==="
ls -lah "$remoteCsv"
echo "=== Line count ==="
wc -l "$remoteCsv"
echo "=== Head ==="
head -n 5 "$remoteCsv"
"@

ssh $Node $remoteCmd

Write-Host "`n=== 2) SCP back to laptop ==="
scp "$Node`:$remoteCsv" "$localCsv"

Write-Host "`n=== 3) Local checks ==="
Write-Host "Saved: $localCsv"
# PowerShell equivalents of wc/head
$lineCount = (Get-Content $localCsv | Measure-Object -Line).Lines
Write-Host "Lines: $lineCount"
Get-Content $localCsv -TotalCount 5 | ForEach-Object { $_ }

if ($Commit) {
  Require-Command git
  Write-Host "`n=== 4) Git add + commit ==="
  Push-Location $RepoRoot
  try {
    git add -- "$localCsv"
    git status --porcelain
    git commit -m "data: localnet dataset export $ts"
  } finally {
    Pop-Location
  }
}

Write-Host "`n[OK] Done."

