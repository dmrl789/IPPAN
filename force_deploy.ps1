# Force Deploy IPPAN - Direct API Approach
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "=== FORCE DEPLOYING IPPAN ===" -ForegroundColor Cyan

# Function to enable rescue mode and get password
function Enable-RescueMode {
    param($ServerId, $ServerName)
    
    Write-Host "Enabling rescue mode for $ServerName..." -ForegroundColor Yellow
    
    # Get SSH key
    try {
        $sshKeysResponse = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/ssh_keys" -Headers $headers -Method GET
        $sshKeyId = $sshKeysResponse.ssh_keys[0].id
        Write-Host "Using SSH key ID: $sshKeyId" -ForegroundColor Green
    } catch {
        Write-Host "Failed to get SSH keys: $($_.Exception.Message)" -ForegroundColor Red
        return $null
    }
    
    # Enable rescue mode
    $rescueBody = @{
        rescue = "linux64"
        ssh_keys = @($sshKeyId)
    } | ConvertTo-Json
    
    try {
        $rescueResponse = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$ServerId/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody
        $password = $rescueResponse.action.root_password
        Write-Host "✅ Rescue mode enabled for $ServerName" -ForegroundColor Green
        Write-Host "Password: $password" -ForegroundColor Yellow
        return $password
    } catch {
        Write-Host "Failed to enable rescue mode for $ServerName`: $($_.Exception.Message)" -ForegroundColor Red
        return $null
    }
}

# Function to deploy using rescue mode
function Deploy-WithRescue {
    param($ServerIP, $ServerName, $Password, $DeployCommands)
    
    Write-Host "Deploying to $ServerName ($ServerIP)..." -ForegroundColor Green
    
    # Wait for server to restart
    Write-Host "Waiting for server to restart..." -ForegroundColor Yellow
    Start-Sleep -Seconds 60
    
    try {
        $securePassword = ConvertTo-SecureString $Password -AsPlainText -Force
        $credential = New-Object System.Management.Automation.PSCredential("root", $securePassword)
        
        $session = New-SSHSession -ComputerName $ServerIP -Credential $credential -AcceptKey -ConnectionTimeout 30
        
        if ($session) {
            Write-Host "✅ Connected to $ServerName!" -ForegroundColor Green
            
            # Execute deployment commands
            Write-Host "Starting IPPAN deployment on $ServerName..." -ForegroundColor Yellow
            $deployResult = Invoke-SSHCommand -SessionId $session.SessionId -Command $DeployCommands -Timeout 1800
            
            if ($deployResult.ExitStatus -eq 0) {
                Write-Host "✅ IPPAN deployment completed on $ServerName!" -ForegroundColor Green
                Write-Host $deployResult.Output -ForegroundColor Cyan
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

# Enable rescue mode for both servers
Write-Host "Enabling rescue mode for both servers..." -ForegroundColor Cyan
$SERVER1_PASSWORD = Enable-RescueMode -ServerId $SERVER1_ID -ServerName "Server 1"
$SERVER2_PASSWORD = Enable-RescueMode -ServerId $SERVER2_ID -ServerName "Server 2"

if (-not $SERVER1_PASSWORD -or -not $SERVER2_PASSWORD) {
    Write-Host "❌ Failed to enable rescue mode. Cannot proceed." -ForegroundColor Red
    exit 1
}

# Read deployment commands
$SERVER1_COMMANDS = Get-Content "server1_deploy_commands.txt" -Raw
$SERVER2_COMMANDS = Get-Content "server2_deploy_commands.txt" -Raw

# Deploy to Server 1
Write-Host ""
Write-Host "=== DEPLOYING TO SERVER 1 ===" -ForegroundColor Cyan
$server1Success = Deploy-WithRescue -ServerIP "188.245.97.41" -ServerName "Server 1" -Password $SERVER1_PASSWORD -DeployCommands $SERVER1_COMMANDS

# Deploy to Server 2
Write-Host ""
Write-Host "=== DEPLOYING TO SERVER 2 ===" -ForegroundColor Cyan
$server2Success = Deploy-WithRescue -ServerIP "135.181.145.174" -ServerName "Server 2" -Password $SERVER2_PASSWORD -DeployCommands $SERVER2_COMMANDS

# Test APIs
Write-Host ""
Write-Host "=== TESTING APIs ===" -ForegroundColor Cyan
Start-Sleep -Seconds 60

if ($server1Success) {
    try {
        $api1 = Invoke-WebRequest -Uri "http://188.245.97.41:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
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
        $api2 = Invoke-WebRequest -Uri "http://135.181.145.174:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
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
Write-Host "Server 1 (188.245.97.41): $(if($server1Success){'✅ Success'}else{'❌ Failed'})" -ForegroundColor $(if($server1Success){'Green'}else{'Red'})
Write-Host "Server 2 (135.181.145.174): $(if($server2Success){'✅ Success'}else{'❌ Failed'})" -ForegroundColor $(if($server2Success){'Green'}else{'Red'})

if ($server1Success -and $server2Success) {
    Write-Host ""
    Write-Host "🎉 IPPAN BLOCKCHAIN NETWORK DEPLOYED SUCCESSFULLY!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Access URLs:" -ForegroundColor Cyan
    Write-Host "- Server 1 API: http://188.245.97.41:3000" -ForegroundColor White
    Write-Host "- Server 1 Metrics: http://188.245.97.41:9090" -ForegroundColor White
    Write-Host "- Server 2 API: http://135.181.145.174:3000" -ForegroundColor White
    Write-Host "- Server 2 Metrics: http://135.181.145.174:9090" -ForegroundColor White
    Write-Host ""
    Write-Host "P2P Network:" -ForegroundColor Cyan
    Write-Host "- Node 1: 188.245.97.41:8080" -ForegroundColor White
    Write-Host "- Node 2: 135.181.145.174:8080" -ForegroundColor White
} else {
    Write-Host ""
    Write-Host "⚠️ Deployment completed with issues. Check the logs above." -ForegroundColor Yellow
}

# Exit rescue mode
Write-Host ""
Write-Host "=== EXITING RESCUE MODE ===" -ForegroundColor Cyan
try {
    $exitResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/disable_rescue" -Headers $headers -Method POST
    Write-Host "✅ Rescue mode disabled on Server 1" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to exit rescue mode on Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

try {
    $exitResponse2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/disable_rescue" -Headers $headers -Method POST
    Write-Host "✅ Rescue mode disabled on Server 2" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to exit rescue mode on Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== DEPLOYMENT COMPLETE ===" -ForegroundColor Green
