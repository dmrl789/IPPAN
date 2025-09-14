# Quick IPPAN Server Status Check
$SERVER1_IP = "188.245.97.41"    # Nuremberg (Node 1)
$SERVER2_IP = "135.181.145.174"  # Helsinki (Node 2)

Write-Host "=== IPPAN Server Status Check ===" -ForegroundColor Cyan
Write-Host "Server 1 (Nuremberg): $SERVER1_IP" -ForegroundColor Blue
Write-Host "Server 2 (Helsinki): $SERVER2_IP" -ForegroundColor Blue
Write-Host ""

# Test basic connectivity
Write-Host "Testing basic connectivity..." -ForegroundColor Yellow

# Test Server 1
Write-Host "`nServer 1 Status:" -ForegroundColor Green
try {
    $ping1 = Test-Connection -ComputerName $SERVER1_IP -Count 1 -Quiet
    if ($ping1) {
        Write-Host "✅ Server 1 is reachable" -ForegroundColor Green
    } else {
        Write-Host "❌ Server 1 is not reachable" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ Server 1 ping failed: $_" -ForegroundColor Red
}

# Test Server 2
Write-Host "`nServer 2 Status:" -ForegroundColor Green
try {
    $ping2 = Test-Connection -ComputerName $SERVER2_IP -Count 1 -Quiet
    if ($ping2) {
        Write-Host "✅ Server 2 is reachable" -ForegroundColor Green
    } else {
        Write-Host "❌ Server 2 is not reachable" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ Server 2 ping failed: $_" -ForegroundColor Red
}

# Test API endpoints
Write-Host "`nTesting API endpoints..." -ForegroundColor Yellow

# Test Server 1 API
Write-Host "`nServer 1 API (port 3000):" -ForegroundColor Green
try {
    $api1 = Invoke-WebRequest -Uri "http://$SERVER1_IP`:3000/health" -TimeoutSec 5 -UseBasicParsing 2>$null
    if ($api1.StatusCode -eq 200) {
        Write-Host "✅ Server 1 API is responding" -ForegroundColor Green
    } else {
        Write-Host "⚠️ Server 1 API returned status: $($api1.StatusCode)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "❌ Server 1 API is not responding" -ForegroundColor Red
}

# Test Server 2 API
Write-Host "`nServer 2 API (port 3000):" -ForegroundColor Green
try {
    $api2 = Invoke-WebRequest -Uri "http://$SERVER2_IP`:3000/health" -TimeoutSec 5 -UseBasicParsing 2>$null
    if ($api2.StatusCode -eq 200) {
        Write-Host "✅ Server 2 API is responding" -ForegroundColor Green
    } else {
        Write-Host "⚠️ Server 2 API returned status: $($api2.StatusCode)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "❌ Server 2 API is not responding" -ForegroundColor Red
}

# Test P2P ports
Write-Host "`nTesting P2P ports (8080)..." -ForegroundColor Yellow

# Test Server 1 P2P
Write-Host "`nServer 1 P2P (port 8080):" -ForegroundColor Green
try {
    $tcp1 = New-Object System.Net.Sockets.TcpClient
    $connect1 = $tcp1.BeginConnect($SERVER1_IP, 8080, $null, $null)
    $wait1 = $connect1.AsyncWaitHandle.WaitOne(3000, $false)
    if ($wait1) {
        $tcp1.EndConnect($connect1)
        Write-Host "✅ Server 1 P2P port is open" -ForegroundColor Green
        $tcp1.Close()
    } else {
        Write-Host "❌ Server 1 P2P port connection timeout" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ Server 1 P2P port connection failed" -ForegroundColor Red
}

# Test Server 2 P2P
Write-Host "`nServer 2 P2P (port 8080):" -ForegroundColor Green
try {
    $tcp2 = New-Object System.Net.Sockets.TcpClient
    $connect2 = $tcp2.BeginConnect($SERVER2_IP, 8080, $null, $null)
    $wait2 = $connect2.AsyncWaitHandle.WaitOne(3000, $false)
    if ($wait2) {
        $tcp2.EndConnect($connect2)
        Write-Host "✅ Server 2 P2P port is open" -ForegroundColor Green
        $tcp2.Close()
    } else {
        Write-Host "❌ Server 2 P2P port connection timeout" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ Server 2 P2P port connection failed" -ForegroundColor Red
}

Write-Host "`n=== Summary ===" -ForegroundColor Cyan
Write-Host "Access URLs:" -ForegroundColor Blue
Write-Host "Server 1 API: http://$SERVER1_IP`:3000" -ForegroundColor White
Write-Host "Server 1 Grafana: http://$SERVER1_IP`:3001" -ForegroundColor White
Write-Host "Server 1 Prometheus: http://$SERVER1_IP`:9090" -ForegroundColor White
Write-Host "Server 2 API: http://$SERVER2_IP`:3000" -ForegroundColor White
Write-Host "Server 2 Grafana: http://$SERVER2_IP`:3001" -ForegroundColor White
Write-Host "Server 2 Prometheus: http://$SERVER2_IP`:9090" -ForegroundColor White

Write-Host "`nNext steps:" -ForegroundColor Blue
Write-Host "1. If services are not running, check Docker containers on each server" -ForegroundColor White
Write-Host "2. Use the deployment scripts to restart services if needed" -ForegroundColor White
Write-Host "3. Monitor logs for connection issues" -ForegroundColor White
