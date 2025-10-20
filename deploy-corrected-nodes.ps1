#!/usr/bin/env pwsh
# IPPAN Corrected Node Deployment Script
# Deploys the corrected configuration with proper port binding

Write-Host "IPPAN Corrected Node Deployment" -ForegroundColor Cyan
Write-Host "=================================" -ForegroundColor Cyan

# Server configurations
$servers = @(
    @{
        Name = "Server 1 (Node 1)"
        Host = "188.245.97.41"
        User = "root"
        Script = "deploy/deploy-server1.sh"
        ApiPort = 8080
        P2PPort = 9000
    },
    @{
        Name = "Server 2 (Node 2)"
        Host = "135.181.145.174"
        User = "root"
        Script = "deploy/deploy-server2.sh"
        ApiPort = 8081
        P2PPort = 9001
    }
)

# Function to deploy to a server
function Deploy-ToServer {
    param(
        [string]$ServerName,
        [string]$ServerHost,
        [string]$User,
        [string]$ScriptPath,
        [int]$ApiPort,
        [int]$P2PPort
    )
    
    Write-Host "`nDeploying to $ServerName ($ServerHost)" -ForegroundColor Yellow
    
    # Test SSH connectivity first
    Write-Host "  Testing SSH connectivity..." -NoNewline
    try {
        $sshTest = Test-NetConnection -ComputerName $ServerHost -Port 22 -InformationLevel Quiet
        if ($sshTest) {
            Write-Host " [OK] SSH port accessible" -ForegroundColor Green
        } else {
            Write-Host " [FAIL] SSH port not accessible" -ForegroundColor Red
            return $false
        }
    } catch {
        Write-Host " [FAIL] SSH test failed: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
    
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
            # Test SSH connection with this key
            $sshCmd = "ssh -i $key -o ConnectTimeout=10 -o BatchMode=yes $User@$ServerHost 'echo SSH_OK'"
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
    
    if (-not $sshConnected) {
        Write-Host "  [FAIL] Could not establish SSH connection with any key" -ForegroundColor Red
        Write-Host "  Manual deployment required:" -ForegroundColor Yellow
        Write-Host "    1. SSH to $ServerHost manually" -ForegroundColor White
        Write-Host "    2. Run the deployment script" -ForegroundColor White
        Write-Host "    3. Check Docker containers are running" -ForegroundColor White
        return $false
    }
    
    # Copy deployment script to server
    Write-Host "  Copying deployment script..." -NoNewline
    try {
        $copyCmd = "scp -i $workingKey $ScriptPath $User@$ServerHost`:/tmp/"
        Invoke-Expression $copyCmd 2>$null
        Write-Host " [OK] Script copied" -ForegroundColor Green
    } catch {
        Write-Host " [FAIL] Failed to copy script: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
    
    # Execute deployment script
    Write-Host "  Executing deployment script..." -NoNewline
    try {
        $deployCmd = "ssh -i $workingKey $User@$ServerHost 'chmod +x /tmp/deploy-server*.sh && /tmp/deploy-server*.sh'"
        $deployResult = Invoke-Expression $deployCmd 2>$null
        Write-Host " [OK] Deployment script executed" -ForegroundColor Green
    } catch {
        Write-Host " [FAIL] Deployment failed: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
    
    # Wait for deployment to complete
    Write-Host "  Waiting for deployment to complete..." -NoNewline
    Start-Sleep -Seconds 30
    Write-Host " [OK] Wait completed" -ForegroundColor Green
    
    # Test if ports are now accessible
    Write-Host "  Testing API port $ApiPort..." -NoNewline
    $apiTest = Test-NetConnection -ComputerName $ServerHost -Port $ApiPort -InformationLevel Quiet
    if ($apiTest) {
        Write-Host " [OK] API port accessible" -ForegroundColor Green
    } else {
        Write-Host " [FAIL] API port still not accessible" -ForegroundColor Red
    }
    
    Write-Host "  Testing P2P port $P2PPort..." -NoNewline
    $p2pTest = Test-NetConnection -ComputerName $ServerHost -Port $P2PPort -InformationLevel Quiet
    if ($p2pTest) {
        Write-Host " [OK] P2P port accessible" -ForegroundColor Green
    } else {
        Write-Host " [FAIL] P2P port still not accessible" -ForegroundColor Red
    }
    
    return ($apiTest -and $p2pTest)
}

# Deploy to all servers
$allDeployed = $true
foreach ($server in $servers) {
    $result = Deploy-ToServer -ServerName $server.Name -ServerHost $server.Host -User $server.User -ScriptPath $server.Script -ApiPort $server.ApiPort -P2PPort $server.P2PPort
    if (-not $result) {
        $allDeployed = $false
    }
}

# Summary
Write-Host "`nDeployment Summary" -ForegroundColor Cyan
Write-Host "==================" -ForegroundColor Cyan

if ($allDeployed) {
    Write-Host "[OK] All nodes deployed successfully!" -ForegroundColor Green
    Write-Host "`nNext steps:" -ForegroundColor Yellow
    Write-Host "  - Run connectivity test to verify deployment" -ForegroundColor White
    Write-Host "  - Test API endpoints manually" -ForegroundColor White
    Write-Host "  - Check node logs for any issues" -ForegroundColor White
} else {
    Write-Host "[FAIL] Some deployments failed" -ForegroundColor Red
    Write-Host "`nManual deployment required:" -ForegroundColor Yellow
    Write-Host "  1. SSH to each server manually" -ForegroundColor White
    Write-Host "  2. Run the deployment scripts" -ForegroundColor White
    Write-Host "  3. Check Docker containers are running" -ForegroundColor White
    Write-Host "  4. Verify firewall rules" -ForegroundColor White
}

Write-Host "`nDeployment script completed!" -ForegroundColor Cyan
