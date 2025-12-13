param(
  [string]$User="ippan-devnet",
  [string[]]$Hosts=@("188.245.97.41","135.181.145.174","5.223.51.238","178.156.219.107")
)
$ErrorActionPreference="Stop"
function Ssh([string]$h,[string]$cmd){
  $output = & ssh -o StrictHostKeyChecking=accept-new "$User@$h" $cmd 2>&1
  return $output
}

foreach($h in $Hosts){
  Write-Host "== $h ==" -ForegroundColor Cyan
  $st = Ssh $h "curl -fsS http://127.0.0.1:8080/status"
  Write-Host $st
  $pe = Ssh $h "curl -fsS http://127.0.0.1:8080/p2p/peers || true"
  Write-Host "peers:"
  Write-Host $pe
  Write-Host ""
}

