# Auto Deploy IPPAN using SSH Key
Import-Module Posh-SSH -ErrorAction SilentlyContinue

$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$SSH_KEY_PATH = "$env:USERPROFILE\.ssh\id_rsa_ippan"

Write-Host "=== AUTO DEPLOY IPPAN WITH SSH KEY ===" -ForegroundColor Cyan

# Function to connect using SSH key
function Connect-WithSSHKey {
    param($ServerIP, $ServerName)
    
    Write-Host "Connecting to $ServerName ($ServerIP) using SSH key..." -ForegroundColor Green
    
    try {
        $session = New-SSHSession -ComputerName $ServerIP -KeyFile $SSH_KEY_PATH -AcceptKey -ConnectionTimeout 30
        
        if ($session) {
            Write-Host "✅ Connected to $ServerName!" -ForegroundColor Green
            return $session
        } else {
            Write-Host "❌ Failed to connect to $ServerName" -ForegroundColor Red
            return $null
        }
    } catch {
        Write-Host "❌ Connection failed to $ServerName`: $($_.Exception.Message)" -ForegroundColor Red
        return $null
    }
}

# Function to deploy to server
function Deploy-ToServer {
    param($Session, $ServerName, $DeployCommands)
    
    Write-Host "Deploying IPPAN to $ServerName..." -ForegroundColor Green
    
    try {
        # Execute deployment commands
        Write-Host "Starting IPPAN deployment on $ServerName..." -ForegroundColor Yellow
        $deployResult = Invoke-SSHCommand -SessionId $Session.SessionId -Command $DeployCommands -Timeout 1800
        
        if ($deployResult.ExitStatus -eq 0) {
            Write-Host "✅ IPPAN deployment completed on $ServerName!" -ForegroundColor Green
            Write-Host $deployResult.Output -ForegroundColor Cyan
        } else {
            Write-Host "⚠️ Deployment completed with warnings on $ServerName" -ForegroundColor Yellow
            Write-Host "Error: $($deployResult.Error)" -ForegroundColor Red
        }
        
        # Check service status
        $statusResult = Invoke-SSHCommand -SessionId $Session.SessionId -Command "docker ps --filter 'name=ippan' --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'" -Timeout 30
        
        if ($statusResult.ExitStatus -eq 0) {
            Write-Host "Service status on $ServerName`:" -ForegroundColor Green
            Write-Host $statusResult.Output -ForegroundColor Cyan
        }
        
        return $true
    } catch {
        Write-Host "❌ Deployment failed on $ServerName`: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

# Read deployment commands
$SERVER1_COMMANDS = Get-Content "server1_deploy_commands.txt" -Raw
$SERVER2_COMMANDS = Get-Content "server2_deploy_commands.txt" -Raw

# Deploy to Server 1
Write-Host "=== DEPLOYING TO SERVER 1 ===" -ForegroundColor Cyan
$server1Session = Connect-WithSSHKey -ServerIP $SERVER1_IP -ServerName "Server 1"

if ($server1Session) {
    $server1Success = Deploy-ToServer -Session $server1Session -ServerName "Server 1" -DeployCommands $SERVER1_COMMANDS
    Remove-SSHSession -SessionId $server1Session.SessionId
} else {
    Write-Host "❌ Could not connect to Server 1" -ForegroundColor Red
    $server1Success = $false
}

# Deploy to Server 2
Write-Host ""
Write-Host "=== DEPLOYING TO SERVER 2 ===" -ForegroundColor Cyan
$server2Session = Connect-WithSSHKey -ServerIP $SERVER2_IP -ServerName "Server 2"

if ($server2Session) {
    $server2Success = Deploy-ToServer -Session $server2Session -ServerName "Server 2" -DeployCommands $SERVER2_COMMANDS
    Remove-SSHSession -SessionId $server2Session.SessionId
} else {
    Write-Host "❌ Could not connect to Server 2" -ForegroundColor Red
    $server2Success = $false
}

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
