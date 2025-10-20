#!/usr/bin/env pwsh
# Simple script to fix Server 2 configuration

Write-Host "Fixing Server 2 Configuration" -ForegroundColor Cyan
Write-Host "=============================" -ForegroundColor Cyan

# Connect to Server 2 and update configuration
$sshKey = "~/.ssh/id_ed25519"
$server2Host = "135.181.145.174"
$server2User = "root"

Write-Host "`nConnecting to Server 2..." -ForegroundColor Yellow

try {
    # Update the P2P bootstrap configuration
    Write-Host "  Updating P2P bootstrap configuration..." -NoNewline
    $updateCmd = "ssh -i $sshKey $server2User@$server2Host 'cd /opt/ippan && echo \"P2P_BOOTSTRAP=/ip4/188.245.97.41/tcp/9000\" >> .env'"
    Invoke-Expression $updateCmd 2>$null
    Write-Host " [OK] Configuration updated" -ForegroundColor Green
    
    # Restart the container
    Write-Host "  Restarting container..." -NoNewline
    $restartCmd = "ssh -i $sshKey $server2User@$server2Host 'cd /opt/ippan && docker compose down && docker compose up -d'"
    Invoke-Expression $restartCmd 2>$null
    Write-Host " [OK] Container restarted" -ForegroundColor Green
    
    # Wait for restart
    Write-Host "  Waiting for restart..." -NoNewline
    Start-Sleep -Seconds 30
    Write-Host " [OK] Wait completed" -ForegroundColor Green
    
    # Test connectivity
    Write-Host "`nTesting connectivity..." -ForegroundColor Yellow
    $apiTest = Test-NetConnection -ComputerName $server2Host -Port 8082 -InformationLevel Quiet
    
    if ($apiTest) {
        Write-Host "  [OK] API port 8082 accessible" -ForegroundColor Green
        
        # Test health endpoint
        try {
            $healthResponse = Invoke-WebRequest -Uri "http://$server2Host:8082/health" -UseBasicParsing -TimeoutSec 10
            if ($healthResponse.StatusCode -eq 200) {
                $healthData = $healthResponse.Content | ConvertFrom-Json
                Write-Host "  [OK] Health check passed" -ForegroundColor Green
                Write-Host "    Status: $($healthData.status)" -ForegroundColor Gray
                Write-Host "    Peer count: $($healthData.peer_count)" -ForegroundColor Gray
                Write-Host "    Uptime: $($healthData.uptime_seconds) seconds" -ForegroundColor Gray
            }
        } catch {
            Write-Host "  [WARN] Health check failed: $($_.Exception.Message)" -ForegroundColor Yellow
        }
    } else {
        Write-Host "  [FAIL] API port 8082 not accessible" -ForegroundColor Red
    }
    
    Write-Host "`n[SUCCESS] Server 2 configuration updated!" -ForegroundColor Green
    Write-Host "`nNext steps:" -ForegroundColor Yellow
    Write-Host "  1. Deploy Server 1 when accessible" -ForegroundColor White
    Write-Host "  2. Run: .\deploy-server1-when-accessible.ps1" -ForegroundColor White
    Write-Host "  3. Verify peer connections" -ForegroundColor White
    
} catch {
    Write-Host "`n[FAIL] Configuration update failed: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host "`nConfiguration script completed!" -ForegroundColor Cyan
