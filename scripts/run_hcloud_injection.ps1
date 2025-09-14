Param(
  [Parameter(Mandatory=$true)][string]$Token,
  [string]$Server1IP = "188.245.97.41",
  [string]$Server2IP = "135.181.145.174"
)

$ErrorActionPreference = 'Stop'

function Write-Info([string]$m){ Write-Host "[INFO] $m" -ForegroundColor Green }
function Write-Err([string]$m){ Write-Host "[ERROR] $m" -ForegroundColor Red }

try {
  $pub1 = Join-Path $env:USERPROFILE '.ssh\id_ed25519.pub'
  $pub2 = Join-Path $env:USERPROFILE '.ssh\id_rsa.pub'
  if (Test-Path $pub1) {
    $p = $pub1
  } elseif (Test-Path $pub2) {
    $p = $pub2
  } else {
    Write-Info "No SSH key found; generating ed25519 key"
    New-Item -ItemType Directory -Force -Path (Join-Path $env:USERPROFILE '.ssh') | Out-Null
    ssh-keygen -t ed25519 -C 'ippan-deployment' -f (Join-Path $env:USERPROFILE '.ssh\id_ed25519') -N '' | Out-Null
    $p = $pub1
  }

  $scriptPath = Join-Path $PSScriptRoot 'hcloud_rescue_inject.ps1'
  if (-not (Test-Path $scriptPath)) { throw "Injector script not found at $scriptPath" }

  $keyName = 'LaptopKey-' + (Get-Date -Format yyyyMMddHHmmss)
  Write-Info "Starting rescue-based injection for $Server1IP and $Server2IP"
  & $scriptPath -Token $Token -Server1IP $Server1IP -Server2IP $Server2IP -SshPubKeyPath $p -HetznerKeyName $keyName
  Write-Info "Injection complete"
} catch {
  Write-Err $_.Exception.Message
  exit 1
}


