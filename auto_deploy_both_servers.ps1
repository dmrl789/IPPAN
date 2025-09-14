# Auto Deploy IPPAN to Both Servers
Import-Module Posh-SSH -ErrorAction SilentlyContinue

$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "=== Auto Deploying IPPAN to Both Servers ===" -ForegroundColor Cyan

# Function to get rescue password
function Get-RescuePassword {
    param($ServerId, $ServerName)
    
    try {
        $actions = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$ServerId/actions" -Headers $headers -Method GET
        $rescueAction = $actions.actions | Where-Object { $_.command -eq "enable_rescue" } | Sort-Object -Property id -Descending | Select-Object -First 1
        
        if ($rescueAction -and $rescueAction.root_password) {
            return $rescueAction.root_password
        } else {
            Write-Host "No rescue password found for $ServerName. Enabling fresh rescue mode..." -ForegroundColor Yellow
            
            # Get SSH key
            $sshKeysResponse = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/ssh_keys" -Headers $headers -Method GET
            $sshKeyId = $sshKeysResponse.ssh_keys[0].id
            
            # Enable rescue mode
            $rescueBody = @{
                rescue = "linux64"
                ssh_keys = @($sshKeyId)
            } | ConvertTo-Json
            
            $rescueResponse = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$ServerId/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody
            return $rescueResponse.action.root_password
        }
    } catch {
        Write-Host "Failed to get rescue password for $ServerName`: $($_.Exception.Message)" -ForegroundColor Red
        return $null
    }
}

# Function to deploy to server
function Deploy-ToServer {
    param($ServerIP, $ServerName, $DeployCommands)
    
    Write-Host "Deploying to $ServerName ($ServerIP)..." -ForegroundColor Green
    
    try {
        $securePassword = ConvertTo-SecureString $DeployCommands.Password -AsPlainText -Force
        $credential = New-Object System.Management.Automation.PSCredential("root", $securePassword)
        
        $session = New-SSHSession -ComputerName $ServerIP -Credential $credential -AcceptKey -ConnectionTimeout 30
        
        if ($session) {
            Write-Host "✅ Connected to $ServerName!" -ForegroundColor Green
            
            # Execute deployment commands
            Write-Host "Starting IPPAN deployment on $ServerName..." -ForegroundColor Yellow
            $deployResult = Invoke-SSHCommand -SessionId $session.SessionId -Command $DeployCommands.Commands -Timeout 1800
            
            if ($deployResult.ExitStatus -eq 0) {
                Write-Host "✅ IPPAN deployment completed on $ServerName!" -ForegroundColor Green
            } else {
                Write-Host "⚠️ Deployment completed with warnings on $ServerName" -ForegroundColor Yellow
                Write-Host "Error: $($deployResult.Error)" -ForegroundColor Red
            }
            
            # Check service status
            $statusResult = Invoke-SSHCommand -SessionId $session.SessionId -Command "docker ps --filter 'name=ippan' --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'" -Timeout 30
            
            if ($statusResult.ExitStatus -eq 0) {
                Write-Host "Service status on $ServerName`:" -ForegroundColor Green
                Write-Host $statusResult.Output -ForegroundColor Cyan
            }
            
            Remove-SSHSession -SessionId $session.SessionId
            return $true
        } else {
            Write-Host "❌ Failed to connect to $ServerName" -ForegroundColor Red
            return $false
        }
    } catch {
        Write-Host "❌ Deployment failed on $ServerName`: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

# Get rescue passwords
Write-Host "Getting rescue passwords..." -ForegroundColor Yellow
$SERVER1_PASSWORD = Get-RescuePassword -ServerId $SERVER1_ID -ServerName "Server 1"
$SERVER2_PASSWORD = Get-RescuePassword -ServerId $SERVER2_ID -ServerName "Server 2"

if (-not $SERVER1_PASSWORD -or -not $SERVER2_PASSWORD) {
    Write-Host "❌ Failed to get rescue passwords. Cannot proceed with deployment." -ForegroundColor Red
    exit 1
}

Write-Host "✅ Got rescue passwords for both servers" -ForegroundColor Green

# Wait for servers to restart if rescue mode was just enabled
Write-Host "Waiting for servers to restart..." -ForegroundColor Yellow
Start-Sleep -Seconds 60

# Read deployment commands
$SERVER1_COMMANDS = Get-Content "server1_deploy_commands.txt" -Raw
$SERVER2_COMMANDS = Get-Content "server2_deploy_commands.txt" -Raw

# Deploy to Server 1
$server1Deploy = @{
    Password = $SERVER1_PASSWORD
    Commands = $SERVER1_COMMANDS
}
$server1Success = Deploy-ToServer -ServerIP $SERVER1_IP -ServerName "Server 1" -DeployCommands $server1Deploy

# Deploy to Server 2
$server2Deploy = @{
    Password = $SERVER2_PASSWORD
    Commands = $SERVER2_COMMANDS
}
$server2Success = Deploy-ToServer -ServerIP $SERVER2_IP -ServerName "Server 2" -DeployCommands $server2Deploy

# Test APIs
Write-Host ""
Write-Host "=== Testing APIs ===" -ForegroundColor Cyan
Start-Sleep -Seconds 30

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
    Write-Host "⚠️ Deployment completed with issues. Check the logs above." -ForegroundColor Yellow
    Write-Host "You may need to manually deploy using the command files." -ForegroundColor Yellow
}
