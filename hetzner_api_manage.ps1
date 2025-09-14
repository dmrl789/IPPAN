# Hetzner API Management Script
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "=== Hetzner API Server Management ===" -ForegroundColor Cyan
Write-Host ""

# Get all servers
Write-Host "Fetching server information..." -ForegroundColor Green
try {
    $serversResponse = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers" -Headers $headers -Method GET
    $servers = $serversResponse.servers
    
    Write-Host "Found $($servers.Count) servers:" -ForegroundColor Yellow
    foreach ($server in $servers) {
        $status = $server.status
        $name = $server.name
        $ip = $server.public_net.ipv4.ip
        $id = $server.id
        
        Write-Host "Server: $name (ID: $id)" -ForegroundColor White
        Write-Host "  IP: $ip" -ForegroundColor Gray
        Write-Host "  Status: $status" -ForegroundColor $(if ($status -eq "running") { "Green" } else { "Red" })
        Write-Host ""
    }
    
    # Find our specific servers
    $server1 = $servers | Where-Object { $_.public_net.ipv4.ip -eq $SERVER1_IP }
    $server2 = $servers | Where-Object { $_.public_net.ipv4.ip -eq $SERVER2_IP }
    
    if ($server1) {
        Write-Host "Server 1 Details:" -ForegroundColor Green
        Write-Host "  Name: $($server1.name)" -ForegroundColor White
        Write-Host "  ID: $($server1.id)" -ForegroundColor Gray
        Write-Host "  Status: $($server1.status)" -ForegroundColor $(if ($server1.status -eq "running") { "Green" } else { "Red" })
        Write-Host "  Rescue: $($server1.rescue_enabled)" -ForegroundColor Yellow
    }
    
    if ($server2) {
        Write-Host "Server 2 Details:" -ForegroundColor Green
        Write-Host "  Name: $($server2.name)" -ForegroundColor White
        Write-Host "  ID: $($server2.id)" -ForegroundColor Gray
        Write-Host "  Status: $($server2.status)" -ForegroundColor $(if ($server2.status -eq "running") { "Red" } else { "Green" })
        Write-Host "  Rescue: $($server2.rescue_enabled)" -ForegroundColor Yellow
    }
    
} catch {
    Write-Host "❌ Failed to fetch server information: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Testing Server Connectivity ===" -ForegroundColor Cyan

# Test basic connectivity
Write-Host "Testing Server 1 connectivity..." -ForegroundColor Green
$ping1 = Test-NetConnection -ComputerName $SERVER1_IP -Port 22 -InformationLevel Quiet
if ($ping1) {
    Write-Host "✅ Server 1 SSH port (22) is reachable" -ForegroundColor Green
} else {
    Write-Host "❌ Server 1 SSH port (22) is not reachable" -ForegroundColor Red
}

Write-Host "Testing Server 2 connectivity..." -ForegroundColor Green
$ping2 = Test-NetConnection -ComputerName $SERVER2_IP -Port 22 -InformationLevel Quiet
if ($ping2) {
    Write-Host "✅ Server 2 SSH port (22) is reachable" -ForegroundColor Green
} else {
    Write-Host "❌ Server 2 SSH port (22) is not reachable" -ForegroundColor Red
}

# Test API endpoints
Write-Host ""
Write-Host "Testing IPPAN API endpoints..." -ForegroundColor Green

try {
    $api1 = Invoke-WebRequest -Uri "http://$SERVER1_IP`:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    Write-Host "✅ Server 1 API responding: $($api1.StatusCode)" -ForegroundColor Green
} catch {
    Write-Host "❌ Server 1 API not responding" -ForegroundColor Red
}

try {
    $api2 = Invoke-WebRequest -Uri "http://$SERVER2_IP`:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    Write-Host "✅ Server 2 API responding: $($api2.StatusCode)" -ForegroundColor Green
} catch {
    Write-Host "❌ Server 2 API not responding" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Next Steps ===" -ForegroundColor Cyan
Write-Host "1. If servers are not in rescue mode, we can enable rescue mode via API" -ForegroundColor Yellow
Write-Host "2. We can also reboot servers via API if needed" -ForegroundColor Yellow
Write-Host "3. We can inject SSH keys via API" -ForegroundColor Yellow
Write-Host ""
Write-Host "Would you like me to:" -ForegroundColor White
Write-Host "- Enable rescue mode on both servers?" -ForegroundColor Gray
Write-Host "- Reboot servers?" -ForegroundColor Gray
Write-Host "- Inject SSH keys?" -ForegroundColor Gray
