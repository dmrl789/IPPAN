# Check Server Status and Deploy IPPAN
$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"

Write-Host "=== IPPAN Server Status Check ===" -ForegroundColor Cyan
Write-Host ""

# Test Server 1
Write-Host "Testing Server 1 ($SERVER1_IP)..." -ForegroundColor Green
$server1Online = Test-Connection -ComputerName $SERVER1_IP -Count 1 -Quiet
if ($server1Online) {
    Write-Host "✅ Server 1 is online" -ForegroundColor Green
} else {
    Write-Host "❌ Server 1 is offline" -ForegroundColor Red
}

# Test Server 2
Write-Host "Testing Server 2 ($SERVER2_IP)..." -ForegroundColor Green
$server2Online = Test-Connection -ComputerName $SERVER2_IP -Count 1 -Quiet
if ($server2Online) {
    Write-Host "✅ Server 2 is online" -ForegroundColor Green
} else {
    Write-Host "❌ Server 2 is offline" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Testing IPPAN APIs ===" -ForegroundColor Yellow

# Test Server 1 API
$server1API = $false
try {
    $response1 = Invoke-WebRequest -Uri "http://$SERVER1_IP`:3000/health" -TimeoutSec 5 -UseBasicParsing 2>$null
    if ($response1.StatusCode -eq 200) {
        $server1API = $true
        Write-Host "✅ Server 1 IPPAN API is running" -ForegroundColor Green
    }
} catch {
    Write-Host "❌ Server 1 IPPAN API is not running" -ForegroundColor Red
}

# Test Server 2 API
$server2API = $false
try {
    $response2 = Invoke-WebRequest -Uri "http://$SERVER2_IP`:3000/health" -TimeoutSec 5 -UseBasicParsing 2>$null
    if ($response2.StatusCode -eq 200) {
        $server2API = $true
        Write-Host "✅ Server 2 IPPAN API is running" -ForegroundColor Green
    }
} catch {
    Write-Host "❌ Server 2 IPPAN API is not running" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Current Status ===" -ForegroundColor Cyan
Write-Host "Server 1: $(if($server1Online){'Online'}else{'Offline'}) - IPPAN: $(if($server1API){'Running'}else{'Not Running'})" -ForegroundColor White
Write-Host "Server 2: $(if($server2Online){'Online'}else{'Offline'}) - IPPAN: $(if($server2API){'Running'}else{'Not Running'})" -ForegroundColor White

if ($server1API -and $server2API) {
    Write-Host ""
    Write-Host "🎉 Both servers are running IPPAN services!" -ForegroundColor Green
    Write-Host "The servers should be connected automatically." -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host "⚠️ IPPAN services need to be deployed" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "=== Next Steps ===" -ForegroundColor Cyan
    Write-Host "1. Put Server 2 in rescue mode via Hetzner Console" -ForegroundColor White
    Write-Host "2. I will deploy IPPAN on both servers" -ForegroundColor White
    Write-Host "3. Test the connection between servers" -ForegroundColor White
}

Write-Host ""
Write-Host "=== Access URLs ===" -ForegroundColor Cyan
Write-Host "Server 1 API: http://$SERVER1_IP`:3000" -ForegroundColor White
Write-Host "Server 1 Grafana: http://$SERVER1_IP`:3001" -ForegroundColor White
Write-Host "Server 1 Prometheus: http://$SERVER1_IP`:9090" -ForegroundColor White
Write-Host "Server 2 API: http://$SERVER2_IP`:3000" -ForegroundColor White
Write-Host "Server 2 Grafana: http://$SERVER2_IP`:3001" -ForegroundColor White
Write-Host "Server 2 Prometheus: http://$SERVER2_IP`:9090" -ForegroundColor White
