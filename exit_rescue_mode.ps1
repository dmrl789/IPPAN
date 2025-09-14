# Exit Rescue Mode
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "Exiting rescue mode on both servers..." -ForegroundColor Green

# Exit rescue mode on Server 1
try {
    $exitResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/disable_rescue" -Headers $headers -Method POST
    Write-Host "âœ… Rescue mode disabled on Server 1" -ForegroundColor Green
} catch {
    Write-Host "âŒ Failed to exit rescue mode on Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

# Exit rescue mode on Server 2
try {
    $exitResponse2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/disable_rescue" -Headers $headers -Method POST
    Write-Host "âœ… Rescue mode disabled on Server 2" -ForegroundColor Green
} catch {
    Write-Host "âŒ Failed to exit rescue mode on Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host "Waiting for servers to restart..." -ForegroundColor Yellow
Start-Sleep -Seconds 60

Write-Host "Testing APIs..." -ForegroundColor Green
try {
    $api1 = Invoke-WebRequest -Uri "http://188.245.97.41:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    Write-Host "âœ… Server 1 API responding: $($api1.StatusCode)" -ForegroundColor Green
} catch {
    Write-Host "âŒ Server 1 API not responding" -ForegroundColor Red
}

try {
    $api2 = Invoke-WebRequest -Uri "http://135.181.145.174:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    Write-Host "âœ… Server 2 API responding: $($api2.StatusCode)" -ForegroundColor Green
} catch {
    Write-Host "âŒ Server 2 API not responding" -ForegroundColor Red
}
