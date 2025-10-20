#!/usr/bin/env pwsh
# IPPAN Node Connectivity Test Script
# Tests the corrected deployment with proper port binding

Write-Host "IPPAN Node Connectivity Test" -ForegroundColor Cyan
Write-Host "=================================" -ForegroundColor Cyan

# Server configurations
$servers = @(
    @{
        Name = "Server 1 (Node 1)"
        Host = "188.245.97.41"
        ApiPort = 8080
        P2PPort = 9000
    },
    @{
        Name = "Server 2 (Node 2)"
        Host = "135.181.145.174"
        ApiPort = 8082
        P2PPort = 9001
    }
)

# Test function for connectivity
function Test-NodeConnectivity {
    param(
        [string]$ServerName,
        [string]$ServerHost,
        [int]$ApiPort,
        [int]$P2PPort
    )
    
    Write-Host "`nTesting $ServerName ($ServerHost)" -ForegroundColor Yellow
    
    # Test basic connectivity
    Write-Host "  Testing basic connectivity..." -NoNewline
    $pingResult = Test-Connection -ComputerName $ServerHost -Count 1 -Quiet
    if ($pingResult) {
        Write-Host " [OK] Ping successful" -ForegroundColor Green
    } else {
        Write-Host " [FAIL] Ping failed" -ForegroundColor Red
        return $false
    }
    
    # Test API port
    Write-Host "  Testing API port $ApiPort..." -NoNewline
    $apiTest = Test-NetConnection -ComputerName $ServerHost -Port $ApiPort -InformationLevel Quiet
    if ($apiTest) {
        Write-Host " [OK] API port accessible" -ForegroundColor Green
        
        # Try to get health status
        try {
            $healthUrl = "http://$ServerHost`:$ApiPort/health"
            Write-Host "  Testing health endpoint..." -NoNewline
            $response = Invoke-WebRequest -Uri $healthUrl -TimeoutSec 10 -UseBasicParsing
            if ($response.StatusCode -eq 200) {
                Write-Host " [OK] Health check passed" -ForegroundColor Green
                Write-Host "    Response: $($response.Content)" -ForegroundColor Gray
            } else {
                Write-Host " [WARN] Health check returned status $($response.StatusCode)" -ForegroundColor Yellow
            }
        } catch {
            Write-Host " [FAIL] Health check failed: $($_.Exception.Message)" -ForegroundColor Red
        }
    } else {
        Write-Host " [FAIL] API port not accessible" -ForegroundColor Red
    }
    
    # Test P2P port
    Write-Host "  Testing P2P port $P2PPort..." -NoNewline
    $p2pTest = Test-NetConnection -ComputerName $ServerHost -Port $P2PPort -InformationLevel Quiet
    if ($p2pTest) {
        Write-Host " [OK] P2P port accessible" -ForegroundColor Green
    } else {
        Write-Host " [FAIL] P2P port not accessible" -ForegroundColor Red
    }
    
    return ($pingResult -and $apiTest -and $p2pTest)
}

# Test all servers
$allHealthy = $true
foreach ($server in $servers) {
    $result = Test-NodeConnectivity -ServerName $server.Name -ServerHost $server.Host -ApiPort $server.ApiPort -P2PPort $server.P2PPort
    if (-not $result) {
        $allHealthy = $false
    }
}

# Summary
Write-Host "`nTest Summary" -ForegroundColor Cyan
Write-Host "==============" -ForegroundColor Cyan

if ($allHealthy) {
    Write-Host "[OK] All nodes are accessible and healthy!" -ForegroundColor Green
    Write-Host "`nNext steps:" -ForegroundColor Yellow
    Write-Host "  - Test API endpoints manually" -ForegroundColor White
    Write-Host "  - Verify P2P connectivity between nodes" -ForegroundColor White
    Write-Host "  - Check node logs for any issues" -ForegroundColor White
} else {
    Write-Host "[FAIL] Some nodes are not accessible" -ForegroundColor Red
    Write-Host "`nTroubleshooting steps:" -ForegroundColor Yellow
    Write-Host "  - Check if servers are running" -ForegroundColor White
    Write-Host "  - Verify Docker containers are started" -ForegroundColor White
    Write-Host "  - Check firewall rules" -ForegroundColor White
    Write-Host "  - Verify deployment scripts were executed" -ForegroundColor White
}

# Test inter-node connectivity
Write-Host "`nTesting Inter-Node Connectivity" -ForegroundColor Cyan
Write-Host "====================================" -ForegroundColor Cyan

Write-Host "Testing if nodes can reach each other..." -ForegroundColor Yellow
$node1ToNode2 = Test-NetConnection -ComputerName "135.181.145.174" -Port 9001 -InformationLevel Quiet
$node2ToNode1 = Test-NetConnection -ComputerName "188.245.97.41" -Port 9000 -InformationLevel Quiet

if ($node1ToNode2 -and $node2ToNode1) {
    Write-Host "[OK] Nodes can communicate with each other" -ForegroundColor Green
} else {
    Write-Host "[FAIL] Nodes cannot communicate with each other" -ForegroundColor Red
    Write-Host "  - Check P2P port configurations" -ForegroundColor White
    Write-Host "  - Verify bootstrap node settings" -ForegroundColor White
    Write-Host "  - Check firewall rules between servers" -ForegroundColor White
}

Write-Host "`nConnectivity test completed!" -ForegroundColor Cyan
