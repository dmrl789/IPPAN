Param(
  [Parameter(Mandatory=$true)][string]$Token,
  [string]$Server1IP = "188.245.97.41",
  [string]$Server2IP = "135.181.145.174"
)

$ErrorActionPreference = 'Stop'

function Write-Info([string]$m){ Write-Host "[INFO] $m" -ForegroundColor Green }
function Write-Err([string]$m){ Write-Host "[ERROR] $m" -ForegroundColor Red }

$BaseUrl = 'https://api.hetzner.cloud/v1'
$Headers = @{ 'Authorization' = "Bearer $Token"; 'Content-Type' = 'application/json' }

function Invoke-HcloudApi([string]$Method, [string]$Path, $BodyObj){
  $Uri = "$BaseUrl$Path"
  if ($null -ne $BodyObj){
    $Body = ($BodyObj | ConvertTo-Json -Depth 10)
    return Invoke-RestMethod -Method $Method -Uri $Uri -Headers $Headers -Body $Body
  } else {
    return Invoke-RestMethod -Method $Method -Uri $Uri -Headers $Headers
  }
}

function Get-ServerByIP([string]$ip){
  $resp = Invoke-HcloudApi GET "/servers" $null
  foreach($s in $resp.servers){
    if ($s.public_net -and $s.public_net.ipv4 -and $s.public_net.ipv4.ip -eq $ip){ return $s }
  }
  return $null
}

function Reboot-Server([int]$serverId){
  Write-Info "Rebooting server ID $serverId"
  Invoke-HcloudApi POST "/servers/$serverId/actions/reboot" $null | Out-Null
}

# Main
Write-Info "Finding servers by IP..."
$server1 = Get-ServerByIP -ip $Server1IP
$server2 = Get-ServerByIP -ip $Server2IP

if ($null -eq $server1) { throw "Server with IP $Server1IP not found" }
if ($null -eq $server2) { throw "Server with IP $Server2IP not found" }

Write-Info "Server1: $($server1.name) (ID: $($server1.id))"
Write-Info "Server2: $($server2.name) (ID: $($server2.id))"

Reboot-Server -serverId $server1.id
Reboot-Server -serverId $server2.id

Write-Info "Both servers rebooting. Wait 2-3 minutes then test SSH access."
