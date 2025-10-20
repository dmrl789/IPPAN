#!/usr/bin/env pwsh
# Configure Server 2 to accept connections from Server 1
# This script updates Server 2's configuration to be ready for peer connections

Write-Host "Configuring Server 2 for Peer Connections" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

$server2 = @{
    Host = "135.181.145.174"
    User = "root"
    ApiPort = 8082
    P2PPort = 9001
}

Write-Host "`nTarget Server: $($server2.Host)" -ForegroundColor Yellow

# Test SSH connectivity to Server 2
Write-Host "`nTesting SSH connectivity to Server 2..." -ForegroundColor Yellow
$sshTest = Test-NetConnection -ComputerName $server2.Host -Port 22 -InformationLevel Quiet

if ($sshTest) {
    Write-Host "[OK] SSH port 22 is accessible" -ForegroundColor Green
    
    # Try SSH connection
    $sshKey = "~/.ssh/id_ed25519"
    Write-Host "  Testing SSH connection..." -NoNewline
    try {
        $sshCmd = "ssh -i $sshKey -o ConnectTimeout=10 -o BatchMode=yes $($server2.User)@$($server2.Host) 'echo SSH_OK'"
        $result = Invoke-Expression $sshCmd 2>$null
        if ($result -eq "SSH_OK") {
            Write-Host " [OK] SSH connection successful" -ForegroundColor Green
            
            # Update Server 2 configuration to accept connections
            Write-Host "`nUpdating Server 2 configuration..." -ForegroundColor Yellow
            
            # Update docker-compose.yml to accept connections from Server 1
            Write-Host "  Updating P2P configuration..." -NoNewline
            try {
                $updateCmd = "ssh -i $sshKey $($server2.User)@$($server2.Host) 'cd /opt/ippan && sed -i \"s/P2P_BOOTSTRAP=.*/P2P_BOOTSTRAP=\/ip4\/188.245.97.41\/tcp\/9000/\" docker-compose.yml'"
                Invoke-Expression $updateCmd 2>$null
                Write-Host " [OK] P2P configuration updated" -ForegroundColor Green
            } catch {
                Write-Host " [FAIL] Failed to update P2P configuration: $($_.Exception.Message)" -ForegroundColor Red
            }
            
            # Restart Server 2 with new configuration
            Write-Host "  Restarting Server 2..." -NoNewline
            try {
                $restartCmd = "ssh -i $sshKey $($server2.User)@$($server2.Host) 'cd /opt/ippan && docker compose down && docker compose up -d'"
                Invoke-Expression $restartCmd 2>$null
                Write-Host " [OK] Server 2 restarted" -ForegroundColor Green
            } catch {
                Write-Host " [FAIL] Failed to restart Server 2: $($_.Exception.Message)" -ForegroundColor Red
            }
            
            # Wait for restart
            Write-Host "  Waiting for Server 2 to restart..." -NoNewline
            Start-Sleep -Seconds 30
            Write-Host " [OK] Wait completed" -ForegroundColor Green
            
            # Test Server 2 connectivity
            Write-Host "`nTesting Server 2 connectivity..." -ForegroundColor Yellow
            $apiTest = Test-NetConnection -ComputerName $server2.Host -Port $server2.ApiPort -InformationLevel Quiet
            $p2pTest = Test-NetConnection -ComputerName $server2.Host -Port $server2.P2PPort -InformationLevel Quiet
            
            if ($apiTest) {
                Write-Host "  [OK] API port $($server2.ApiPort) accessible" -ForegroundColor Green
                
                # Test health endpoint
                try {
                    $healthResponse = Invoke-WebRequest -Uri "http://$($server2.Host):$($server2.ApiPort)/health" -UseBasicParsing -TimeoutSec 10
                    if ($healthResponse.StatusCode -eq 200) {
                        $healthData = $healthResponse.Content | ConvertFrom-Json
                        Write-Host "  [OK] Health check passed - Peer count: $($healthData.peer_count)" -ForegroundColor Green
                    }
                } catch {
                    Write-Host "  [WARN] Health check failed: $($_.Exception.Message)" -ForegroundColor Yellow
                }
            } else {
                Write-Host "  [FAIL] API port $($server2.ApiPort) not accessible" -ForegroundColor Red
            }
            
            if ($p2pTest) {
                Write-Host "  [OK] P2P port $($server2.P2PPort) accessible" -ForegroundColor Green
            } else {
                Write-Host "  [FAIL] P2P port $($server2.P2PPort) not accessible" -ForegroundColor Red
            }
            
            Write-Host "`nServer 2 configuration completed!" -ForegroundColor Green
            Write-Host "`nNext steps:" -ForegroundColor Yellow
            Write-Host "  1. Deploy Server 1 when accessible" -ForegroundColor White
            Write-Host "  2. Run: .\deploy-server1-when-accessible.ps1" -ForegroundColor White
            Write-Host "  3. Verify peer connections between nodes" -ForegroundColor White
            
        } else {
            Write-Host " [FAIL] SSH connection failed" -ForegroundColor Red
        }
    } catch {
        Write-Host " [FAIL] SSH error: $($_.Exception.Message)" -ForegroundColor Red
    }
    
} else {
    Write-Host "[FAIL] SSH port 22 not accessible" -ForegroundColor Red
}

Write-Host "`nConfiguration script completed!" -ForegroundColor Cyan
