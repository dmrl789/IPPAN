param(
  [string]$User="ippan-devnet",
  [string]$N1="188.245.97.41",
  [string]$N2="135.181.145.174",
  [string]$N3="5.223.51.238",
  [string]$N4="178.156.219.107",
  [int]$Iterations=1,
  [int]$SleepSeconds=5
)
$ErrorActionPreference="Stop"
$ts=Get-Date -Format "yyyyMMdd_HHmmss"
$out="tmp/devnet/health_$ts.ndjson"

function Ssh-Cmd([string]$hostname,[string]$command){
  $output = ssh -o StrictHostKeyChecking=accept-new "$User@$hostname" $command 2>&1
  return $output
}

function GetJson([string]$url){
  $raw = curl.exe -fsS $url
  return ($raw | ConvertFrom-Json)
}

for($i=0;$i -lt $Iterations;$i++){
  $ok=$true
  $svc=@{}
  foreach($h in @($N1,$N2,$N3,$N4)){
    $r=Ssh-Cmd $h "sudo -n systemctl is-active ippan-node"
    $svc[$h]=($r.Trim())
    if($svc[$h] -ne "active"){ $ok=$false }
  }

  # Internal RPC reachability to bootstrap
  $t21 = (Ssh-Cmd $N2 "curl -fsS http://$N1`:8080/status >/dev/null && echo OK || echo FAIL").Trim()
  $t31 = (Ssh-Cmd $N3 "curl -fsS http://$N1`:8080/status >/dev/null && echo OK || echo FAIL").Trim()
  if($t21 -ne "OK" -or $t31 -ne "OK"){ $ok=$false }

  $s1 = (Ssh-Cmd $N1 "curl -fsS http://127.0.0.1:8080/status" | ConvertFrom-Json)
  $s4 = GetJson "http://$N4`:8080/status"

  if([int]$s1.peer_count -lt 2){ $ok=$false }
  if([int]$s4.peer_count -lt 2){ $ok=$false }

  $row = [ordered]@{
    ts_utc = (Get-Date).ToUniversalTime().ToString("o")
    services = $svc
    node2_to_node1 = $t21
    node3_to_node1 = $t31
    node1_peer_count = $s1.peer_count
    node4_peer_count = $s4.peer_count
    node4_network_active = $s4.network_active
    pass = $ok
  } | ConvertTo-Json -Compress
  Add-Content -Encoding utf8 $out $row

  if($Iterations -gt 1){ Start-Sleep -Seconds $SleepSeconds }
}

Write-Host "Wrote: $out"
$last = Get-Content $out | Select-Object -Last 1 | ConvertFrom-Json
if($last.pass -eq $true){
  Write-Host "DEVNET HEALTHCHECK: PASS"
  exit 0
}else{
  Write-Host "DEVNET HEALTHCHECK: FAIL"
  exit 2
}

