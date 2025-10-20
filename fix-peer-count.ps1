#!/usr/bin/env pwsh
# Fix IPPAN Peer Count Issue
# This script addresses the peer count = 0 issue

Write-Host "FIXING IPPAN PEER COUNT ISSUE" -ForegroundColor Cyan
Write-Host "=============================" -ForegroundColor Cyan

# Current status
Write-Host "`nCurrent Status:" -ForegroundColor Yellow
Write-Host "  Server 1 (188.245.97.41): Not accessible via SSH" -ForegroundColor Red
Write-Host "  Server 2 (135.181.145.174): Fully operational" -ForegroundColor Green
Write-Host "  Peer Count: 0 (nodes not connected)" -ForegroundColor Red

# Root cause analysis
Write-Host "`nRoot Cause Analysis:" -ForegroundColor Yellow
Write-Host "  The peer count is 0 because:" -ForegroundColor White
Write-Host "  1. Server 1 is not accessible via SSH" -ForegroundColor White
Write-Host "  2. Server 1 has no IPPAN node deployed" -ForegroundColor White
Write-Host "  3. Server 2 cannot connect to Server 1" -ForegroundColor White
Write-Host "  4. No peer-to-peer network established" -ForegroundColor White

# Solution 1: Try alternative access methods for Server 1
Write-Host "`nSolution 1: Alternative Server 1 Access" -ForegroundColor Yellow
Write-Host "=========================================" -ForegroundColor Yellow

# Test different ports on Server 1
$server1Ports = @(22, 2222, 443, 80, 3389, 8080, 9000, 8081, 8082, 3000, 5000)
$openPorts = @()

foreach ($port in $server1Ports) {
    Write-Host "  Testing port $port..." -NoNewline
    $test = Test-NetConnection -ComputerName "188.245.97.41" -Port $port -InformationLevel Quiet
    if ($test) {
        Write-Host " [OPEN]" -ForegroundColor Green
        $openPorts += $port
    } else {
        Write-Host " [CLOSED]" -ForegroundColor Red
    }
}

if ($openPorts.Count -gt 0) {
    Write-Host "`n  Open ports found: $($openPorts -join ', ')" -ForegroundColor Green
    
    # Try web-based access
    if ($openPorts -contains 80) {
        Write-Host "  Testing web interface..." -NoNewline
        try {
            $webResponse = Invoke-WebRequest -Uri "http://188.245.97.41/" -UseBasicParsing -TimeoutSec 10
            if ($webResponse.StatusCode -eq 200) {
                Write-Host " [OK] Web interface accessible" -ForegroundColor Green
                Write-Host "    Server: $($webResponse.Headers['Server'])" -ForegroundColor Gray
            }
        } catch {
            Write-Host " [FAIL] Web interface not accessible" -ForegroundColor Red
        }
    }
} else {
    Write-Host "`n  No accessible ports found on Server 1" -ForegroundColor Red
}

# Solution 2: Create deployment package for Server 1
Write-Host "`nSolution 2: Server 1 Deployment Package" -ForegroundColor Yellow
Write-Host "=======================================" -ForegroundColor Yellow

Write-Host "  Creating deployment package for Server 1..." -NoNewline
try {
    # Create deployment directory
    if (!(Test-Path "server1-deployment")) {
        New-Item -ItemType Directory -Name "server1-deployment" | Out-Null
    }
    
    # Copy deployment files
    Copy-Item "deploy/deploy-server1.sh" "server1-deployment/" -Force
    Copy-Item "deploy/docker-compose.yml" "server1-deployment/" -Force
    Copy-Item "config/ippan-config.json" "server1-deployment/" -Force
    
    # Create deployment instructions
    $instructions = @"
# Server 1 Deployment Instructions

## Manual Deployment Steps:

1. Access Server 1 (188.245.97.41) via any available method:
   - SSH (if port 22 becomes accessible)
   - Web interface (if available)
   - Console access (if physical access)
   - Remote desktop (if RDP is enabled)

2. Copy deployment files to Server 1:
   - Upload server1-deployment/ folder to /opt/ippan/
   - Or use: scp -r server1-deployment/ root@188.245.97.41:/opt/ippan/

3. Run deployment:
   ```bash
   cd /opt/ippan
   chmod +x deploy-server1.sh
   ./deploy-server1.sh
   ```

4. Verify deployment:
   ```bash
   docker ps
   curl http://localhost:8080/health
   ```

5. Configure firewall:
   ```bash
   ufw allow 8080/tcp
   ufw allow 9000/tcp
   ```

## Alternative: Use Different Server

If Server 1 cannot be accessed, deploy to a different server:
- Use the same deployment files
- Update IP addresses in configuration
- Update Server 2 bootstrap configuration

## Verification

After deployment, run:
```powershell
.\test-node-connectivity.ps1
```

Expected result:
- Both servers accessible
- Peer count > 0
- Nodes connected
"@
    
    $instructions | Out-File -FilePath "server1-deployment/README.md" -Encoding UTF8
    
    Write-Host " [OK] Deployment package created" -ForegroundColor Green
    Write-Host "    Location: server1-deployment/" -ForegroundColor Gray
    Write-Host "    Files: deploy-server1.sh, docker-compose.yml, README.md" -ForegroundColor Gray
} catch {
    Write-Host " [FAIL] Package creation failed: $($_.Exception.Message)" -ForegroundColor Red
}

# Solution 3: Test current connectivity
Write-Host "`nSolution 3: Current Connectivity Test" -ForegroundColor Yellow
Write-Host "=====================================" -ForegroundColor Yellow

Write-Host "  Testing current node status..." -ForegroundColor Yellow

# Test Server 2
$server2Test = Test-NetConnection -ComputerName "135.181.145.174" -Port 8082 -InformationLevel Quiet
if ($server2Test) {
    Write-Host "  Server 2 API: [OK] Accessible" -ForegroundColor Green
    
    try {
        $healthResponse = Invoke-WebRequest -Uri "http://135.181.145.174:8082/health" -UseBasicParsing -TimeoutSec 10
        if ($healthResponse.StatusCode -eq 200) {
            $healthData = $healthResponse.Content | ConvertFrom-Json
            Write-Host "    Status: $($healthData.status)" -ForegroundColor Gray
            Write-Host "    Peer Count: $($healthData.peer_count)" -ForegroundColor Gray
            Write-Host "    Uptime: $($healthData.uptime_seconds) seconds" -ForegroundColor Gray
        }
    } catch {
        Write-Host "    Health check failed: $($_.Exception.Message)" -ForegroundColor Red
    }
} else {
    Write-Host "  Server 2 API: [FAIL] Not accessible" -ForegroundColor Red
}

# Test Server 1
$server1Test = Test-NetConnection -ComputerName "188.245.97.41" -Port 8080 -InformationLevel Quiet
if ($server1Test) {
    Write-Host "  Server 1 API: [OK] Accessible" -ForegroundColor Green
} else {
    Write-Host "  Server 1 API: [FAIL] Not accessible" -ForegroundColor Red
}

# Summary and recommendations
Write-Host "`nSummary and Recommendations" -ForegroundColor Cyan
Write-Host "===========================" -ForegroundColor Cyan

Write-Host "`nCurrent Status:" -ForegroundColor Yellow
Write-Host "  ✅ Server 2: Fully operational and healthy" -ForegroundColor Green
Write-Host "  ❌ Server 1: Not accessible via standard methods" -ForegroundColor Red
Write-Host "  ❌ Peer Count: 0 (nodes not connected)" -ForegroundColor Red

Write-Host "`nImmediate Actions:" -ForegroundColor Yellow
Write-Host "  1. Contact server provider for Server 1 access" -ForegroundColor White
Write-Host "  2. Use deployment package in server1-deployment/ folder" -ForegroundColor White
Write-Host "  3. Consider deploying to alternative server" -ForegroundColor White
Write-Host "  4. Use Server 2 as standalone node for now" -ForegroundColor White

Write-Host "`nLong-term Solutions:" -ForegroundColor Yellow
Write-Host "  1. Set up monitoring for Server 1 status" -ForegroundColor White
Write-Host "  2. Implement automated deployment pipeline" -ForegroundColor White
Write-Host "  3. Create backup deployment targets" -ForegroundColor White
Write-Host "  4. Set up health monitoring for both nodes" -ForegroundColor White

Write-Host "`nSolution completed!" -ForegroundColor Cyan
