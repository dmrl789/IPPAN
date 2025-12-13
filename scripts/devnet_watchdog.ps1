param(
  [string]$User="ippan-devnet",
  [string]$N1="188.245.97.41",
  [string]$N2="135.181.145.174",
  [string]$N3="5.223.51.238",
  [string]$N4="178.156.219.107",
  [int]$DurationHours=24,
  [int]$CheckIntervalSeconds=60
)

$ErrorActionPreference="Continue"
$endTime = (Get-Date).AddHours($DurationHours)
$iterations = [math]::Floor(($DurationHours * 3600) / $CheckIntervalSeconds)

Write-Host "Starting devnet watchdog for $DurationHours hours ($iterations iterations, every $CheckIntervalSeconds seconds)" -ForegroundColor Cyan
Write-Host "Will end at: $endTime" -ForegroundColor Gray

function Ssh([string]$h,[string]$cmd){
  $result = cmd.exe /c "ssh $User@$h $cmd" 2>&1
  return $result
}

function CollectIncident([string]$incidentDir){
  Write-Host "`n=== INCIDENT DETECTED: Collecting diagnostics ===" -ForegroundColor Red
  Write-Host "Incident directory: $incidentDir" -ForegroundColor Yellow
  
  New-Item -ItemType Directory -Force -Path $incidentDir | Out-Null
  
  foreach($h in @($N1,$N2,$N3,$N4)){
    $safe = $h.Replace(".","_")
    Write-Host "Collecting from $h..." -ForegroundColor Gray
    
    # Journalctl tail
    Ssh $h "sudo -n journalctl -u ippan-node -n 200 --no-pager" | Out-File -Encoding utf8 "$incidentDir/$safe.journalctl.txt"
    
    # Listeners
    Ssh $h "sudo -n ss -lntup | grep -E ':(8080|9000)' || true" | Out-File -Encoding utf8 "$incidentDir/$safe.listeners.txt"
    
    # UFW status
    Ssh $h "sudo -n ufw status verbose || true" | Out-File -Encoding utf8 "$incidentDir/$safe.ufw.txt"
    
    # Service status
    Ssh $h "sudo -n systemctl status ippan-node --no-pager -n 50 || true" | Out-File -Encoding utf8 "$incidentDir/$safe.service.txt"
    
    # Local status
    Ssh $h "curl -fsS http://127.0.0.1:8080/status || echo FAILED" | Out-File -Encoding utf8 "$incidentDir/$safe.status.txt"
  }
  
  Write-Host "`nIncident diagnostics saved to: $incidentDir" -ForegroundColor Yellow
}

$iteration = 0
while((Get-Date) -lt $endTime){
  $iteration++
  $elapsed = (Get-Date) - ($endTime.AddHours(-$DurationHours))
  $remaining = $endTime - (Get-Date)
  
  Write-Host "[$iteration/$iterations] $(Get-Date -Format 'HH:mm:ss') - Elapsed: $($elapsed.ToString('hh\:mm\:ss')) - Remaining: $($remaining.ToString('hh\:mm\:ss'))" -ForegroundColor Cyan
  
  # Run healthcheck
  $hcResult = powershell.exe -NoProfile -ExecutionPolicy Bypass -File "scripts/devnet_healthcheck.ps1" -Iterations 1 2>&1
  $hcExitCode = $LASTEXITCODE
  
  if($hcExitCode -ne 0){
    $incidentTs = Get-Date -Format "yyyyMMdd_HHmmss"
    $incidentDir = "tmp/devnet/incident_$incidentTs"
    CollectIncident $incidentDir
    
    # Also save healthcheck output
    $hcResult | Out-File -Encoding utf8 "$incidentDir/healthcheck_output.txt"
    
    Write-Host "`nHEALTHCHECK FAILED - Incident logged" -ForegroundColor Red
    Write-Host "Continuing monitoring..." -ForegroundColor Yellow
  } else {
    Write-Host "  Healthcheck: PASS" -ForegroundColor Green
  }
  
  if((Get-Date) -lt $endTime){
    Start-Sleep -Seconds $CheckIntervalSeconds
  }
}

Write-Host "`n=== Watchdog completed ===" -ForegroundColor Green
Write-Host "End time: $(Get-Date)" -ForegroundColor Gray






