#!/usr/bin/env pwsh
# IPPAN Server 1 Deployment Script
# Use this script when Server 1 becomes accessible

Write-Host "IPPAN Server 1 Deployment Script" -ForegroundColor Cyan
Write-Host "=================================" -ForegroundColor Cyan

$server1 = @{
    Host = "188.245.97.41"
    User = "root"
    ApiPort = 8080
    P2PPort = 9000
}

Write-Host "`nTarget Server: $($server1.Host)" -ForegroundColor Yellow
Write-Host "API Port: $($server1.ApiPort)" -ForegroundColor Yellow
Write-Host "P2P Port: $($server1.P2PPort)" -ForegroundColor Yellow

# Test SSH connectivity
Write-Host "`nTesting SSH connectivity..." -ForegroundColor Yellow
$sshTest = Test-NetConnection -ComputerName $server1.Host -Port 22 -InformationLevel Quiet

if ($sshTest) {
    Write-Host "[OK] SSH port 22 is accessible" -ForegroundColor Green
    
    # Try different SSH keys
    $sshKeys = @(
        "~/.ssh/id_ed25519_ippan",
        "~/.ssh/id_rsa_ippan", 
        "~/.ssh/ippan_ci",
        "~/.ssh/id_ed25519",
        "~/.ssh/id_rsa"
    )
    
    $sshConnected = $false
    foreach ($key in $sshKeys) {
        Write-Host "  Trying SSH key: $key" -NoNewline
        try {
            $sshCmd = "ssh -i $key -o ConnectTimeout=10 -o BatchMode=yes $($server1.User)@$($server1.Host) 'echo SSH_OK'"
            $result = Invoke-Expression $sshCmd 2>$null
            if ($result -eq "SSH_OK") {
                Write-Host " [OK] SSH connection successful" -ForegroundColor Green
                $sshConnected = $true
                $workingKey = $key
                break
            } else {
                Write-Host " [FAIL] SSH connection failed" -ForegroundColor Red
            }
        } catch {
            Write-Host " [FAIL] SSH error: $($_.Exception.Message)" -ForegroundColor Red
        }
    }
    
    if ($sshConnected) {
        Write-Host "`nDeploying IPPAN node to Server 1..." -ForegroundColor Yellow
        
        # Copy deployment script
        Write-Host "  Copying deployment script..." -NoNewline
        try {
            $copyCmd = "scp -i $workingKey deploy/deploy-server1.sh $($server1.User)@$($server1.Host)`:/tmp/"
            Invoke-Expression $copyCmd 2>$null
            Write-Host " [OK] Script copied" -ForegroundColor Green
        } catch {
            Write-Host " [FAIL] Failed to copy script: $($_.Exception.Message)" -ForegroundColor Red
        }
        
        # Execute deployment script
        Write-Host "  Executing deployment script..." -NoNewline
        try {
            $deployCmd = "ssh -i $workingKey $($server1.User)@$($server1.Host) 'chmod +x /tmp/deploy-server1.sh && /tmp/deploy-server1.sh'"
            $deployResult = Invoke-Expression $deployCmd 2>$null
            Write-Host " [OK] Deployment script executed" -ForegroundColor Green
        } catch {
            Write-Host " [FAIL] Deployment failed: $($_.Exception.Message)" -ForegroundColor Red
        }
        
        # Wait for deployment
        Write-Host "  Waiting for deployment to complete..." -NoNewline
        Start-Sleep -Seconds 60
        Write-Host " [OK] Wait completed" -ForegroundColor Green
        
        # Test connectivity
        Write-Host "`nTesting Server 1 connectivity..." -ForegroundColor Yellow
        $apiTest = Test-NetConnection -ComputerName $server1.Host -Port $server1.ApiPort -InformationLevel Quiet
        $p2pTest = Test-NetConnection -ComputerName $server1.Host -Port $server1.P2PPort -InformationLevel Quiet
        
        if ($apiTest) {
            Write-Host "  [OK] API port $($server1.ApiPort) accessible" -ForegroundColor Green
        } else {
            Write-Host "  [FAIL] API port $($server1.ApiPort) not accessible" -ForegroundColor Red
        }
        
        if ($p2pTest) {
            Write-Host "  [OK] P2P port $($server1.P2PPort) accessible" -ForegroundColor Green
        } else {
            Write-Host "  [FAIL] P2P port $($server1.P2PPort) not accessible" -ForegroundColor Red
        }
        
        if ($apiTest -and $p2pTest) {
            Write-Host "`n[SUCCESS] Server 1 deployment completed!" -ForegroundColor Green
            Write-Host "Next steps:" -ForegroundColor Yellow
            Write-Host "  1. Update Server 2 bootstrap configuration" -ForegroundColor White
            Write-Host "  2. Restart Server 2 to connect to Server 1" -ForegroundColor White
            Write-Host "  3. Verify peer connections" -ForegroundColor White
        } else {
            Write-Host "`n[FAIL] Server 1 deployment incomplete" -ForegroundColor Red
        }
        
    } else {
        Write-Host "`n[FAIL] Could not establish SSH connection with any key" -ForegroundColor Red
        Write-Host "Manual deployment required:" -ForegroundColor Yellow
        Write-Host "  1. SSH to $($server1.Host) manually" -ForegroundColor White
        Write-Host "  2. Run: chmod +x /tmp/deploy-server1.sh && /tmp/deploy-server1.sh" -ForegroundColor White
        Write-Host "  3. Check Docker containers are running" -ForegroundColor White
    }
    
} else {
    Write-Host "[FAIL] SSH port 22 not accessible" -ForegroundColor Red
    Write-Host "`nServer 1 access issues:" -ForegroundColor Yellow
    Write-Host "  - SSH port 22 is blocked or server is down" -ForegroundColor White
    Write-Host "  - Server may be behind a firewall" -ForegroundColor White
    Write-Host "  - Server may need to be restarted" -ForegroundColor White
    Write-Host "  - Contact server provider for assistance" -ForegroundColor White
    
    Write-Host "`nAlternative solutions:" -ForegroundColor Yellow
    Write-Host "  1. Use Server 2 as standalone node" -ForegroundColor White
    Write-Host "  2. Deploy additional nodes on accessible servers" -ForegroundColor White
    Write-Host "  3. Set up local development environment" -ForegroundColor White
}

Write-Host "`nDeployment script completed!" -ForegroundColor Cyan
