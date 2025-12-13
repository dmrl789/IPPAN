param(
  [string]$User="ippan-devnet",
  [string]$N1="188.245.97.41",
  [string]$N2="135.181.145.174",
  [string]$N3="5.223.51.238",
  [string]$N4="178.156.219.107"
)
$ErrorActionPreference="Stop"
function Ssh([string]$h,[string]$cmd){
  $output = & ssh -o StrictHostKeyChecking=accept-new "$User@$h" $cmd 2>&1
  return $output
}

# Stop leaf nodes first
Write-Host "Restarting leaf nodes..." -ForegroundColor Cyan
Ssh $N2 "sudo -n systemctl restart ippan-node"
Ssh $N3 "sudo -n systemctl restart ippan-node"
Ssh $N4 "sudo -n systemctl restart ippan-node"
Start-Sleep -Seconds 5

# Restart bootstrap last
Write-Host "Restarting bootstrap node..." -ForegroundColor Cyan
Ssh $N1 "sudo -n systemctl restart ippan-node"
Start-Sleep -Seconds 8

# Verify from inside node4
Write-Host "Verifying status..." -ForegroundColor Cyan
Write-Host (Ssh $N4 "curl -fsS http://127.0.0.1:8080/status")
Write-Host (Ssh $N1 "curl -fsS http://127.0.0.1:8080/status")

