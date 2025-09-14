# SSH Deploy IPPAN - Direct SSH Command Approach
$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$SSH_KEY_PATH = "$env:USERPROFILE\.ssh\id_rsa_ippan"

Write-Host "=== SSH DEPLOY IPPAN ===" -ForegroundColor Cyan

# Function to execute commands via SSH
function Invoke-SSHCommand {
    param($ServerIP, $ServerName, $Commands)
    
    Write-Host "Deploying to $ServerName ($ServerIP)..." -ForegroundColor Green
    
    # Create temporary script file
    $tempScript = "temp_deploy_$ServerName.sh"
    $Commands | Out-File -FilePath $tempScript -Encoding UTF8
    
    try {
        # Execute via SSH
        $sshCommand = "ssh -o IdentitiesOnly=yes -i `"$SSH_KEY_PATH`" -o StrictHostKeyChecking=no root@$ServerIP `"bash -s`" < `"$tempScript`""
        
        Write-Host "Executing SSH command..." -ForegroundColor Yellow
        $result = Invoke-Expression $sshCommand
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ Deployment completed on $ServerName!" -ForegroundColor Green
            Write-Host $result -ForegroundColor Cyan
            return $true
        } else {
            Write-Host "⚠️ Deployment completed with warnings on $ServerName" -ForegroundColor Yellow
            Write-Host $result -ForegroundColor Red
            return $false
        }
    } catch {
        Write-Host "❌ Deployment failed on $ServerName`: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    } finally {
        # Clean up temp file
        if (Test-Path $tempScript) {
            Remove-Item $tempScript -Force
        }
    }
}

# Read deployment commands
$SERVER1_COMMANDS = Get-Content "server1_deploy_commands.txt" -Raw
$SERVER2_COMMANDS = Get-Content "server2_deploy_commands.txt" -Raw

# Deploy to Server 1
Write-Host "=== DEPLOYING TO SERVER 1 ===" -ForegroundColor Cyan
$server1Success = Invoke-SSHCommand -ServerIP $SERVER1_IP -ServerName "Server 1" -Commands $SERVER1_COMMANDS

# Deploy to Server 2
Write-Host ""
Write-Host "=== DEPLOYING TO SERVER 2 ===" -ForegroundColor Cyan
$server2Success = Invoke-SSHCommand -ServerIP $SERVER2_IP -ServerName "Server 2" -Commands $SERVER2_COMMANDS

# Test APIs
Write-Host ""
Write-Host "=== TESTING APIs ===" -ForegroundColor Cyan
Start-Sleep -Seconds 60

if ($server1Success) {
    try {
        $api1 = Invoke-WebRequest -Uri "http://$SERVER1_IP`:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
        if ($api1.StatusCode -eq 200) {
            Write-Host "✅ Server 1 API is responding: $($api1.StatusCode)" -ForegroundColor Green
        } else {
            Write-Host "⚠️ Server 1 API returned status: $($api1.StatusCode)" -ForegroundColor Yellow
        }
    } catch {
        Write-Host "❌ Server 1 API is not responding yet" -ForegroundColor Red
    }
}

if ($server2Success) {
    try {
        $api2 = Invoke-WebRequest -Uri "http://$SERVER2_IP`:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
        if ($api2.StatusCode -eq 200) {
            Write-Host "✅ Server 2 API is responding: $($api2.StatusCode)" -ForegroundColor Green
        } else {
            Write-Host "⚠️ Server 2 API returned status: $($api2.StatusCode)" -ForegroundColor Yellow
        }
    } catch {
        Write-Host "❌ Server 2 API is not responding yet" -ForegroundColor Red
    }
}

# Summary
Write-Host ""
Write-Host "=== DEPLOYMENT SUMMARY ===" -ForegroundColor Cyan
Write-Host "Server 1 ($SERVER1_IP): $(if($server1Success){'✅ Success'}else{'❌ Failed'})" -ForegroundColor $(if($server1Success){'Green'}else{'Red'})
Write-Host "Server 2 ($SERVER2_IP): $(if($server2Success){'✅ Success'}else{'❌ Failed'})" -ForegroundColor $(if($server2Success){'Green'}else{'Red'})

if ($server1Success -and $server2Success) {
    Write-Host ""
    Write-Host "🎉 IPPAN BLOCKCHAIN NETWORK DEPLOYED SUCCESSFULLY!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Access URLs:" -ForegroundColor Cyan
    Write-Host "- Server 1 API: http://$SERVER1_IP`:3000" -ForegroundColor White
    Write-Host "- Server 1 Metrics: http://$SERVER1_IP`:9090" -ForegroundColor White
    Write-Host "- Server 2 API: http://$SERVER2_IP`:3000" -ForegroundColor White
    Write-Host "- Server 2 Metrics: http://$SERVER2_IP`:9090" -ForegroundColor White
    Write-Host ""
    Write-Host "P2P Network:" -ForegroundColor Cyan
    Write-Host "- Node 1: $SERVER1_IP`:8080" -ForegroundColor White
    Write-Host "- Node 2: $SERVER2_IP`:8080" -ForegroundColor White
} else {
    Write-Host ""
    Write-Host "⚠️ Deployment completed with issues." -ForegroundColor Yellow
    Write-Host "You may need to manually deploy using the command files." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "=== DEPLOYMENT COMPLETE ===" -ForegroundColor Green
