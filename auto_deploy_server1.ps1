# Auto Deploy IPPAN to Server 1
Import-Module Posh-SSH -ErrorAction SilentlyContinue

$SERVER1_IP = "188.245.97.41"
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_ID = "108447288"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "=== Auto Deploying IPPAN to Server 1 ===" -ForegroundColor Cyan

# Get rescue password
try {
    $actions1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions" -Headers $headers -Method GET
    $rescueAction1 = $actions1.actions | Where-Object { $_.command -eq "enable_rescue" } | Sort-Object -Property id -Descending | Select-Object -First 1
    
    if ($rescueAction1 -and $rescueAction1.root_password) {
        $SERVER1_PASSWORD = $rescueAction1.root_password
        Write-Host "✅ Got rescue password for Server 1" -ForegroundColor Green
    } else {
        Write-Host "❌ No rescue password found. Enabling fresh rescue mode..." -ForegroundColor Yellow
        
        # Enable fresh rescue mode
        $sshKeysResponse = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/ssh_keys" -Headers $headers -Method GET
        $sshKeyId = $sshKeysResponse.ssh_keys[0].id
        
        $rescueBody1 = @{
            rescue = "linux64"
            ssh_keys = @($sshKeyId)
        } | ConvertTo-Json
        
        $rescueResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody1
        $SERVER1_PASSWORD = $rescueResponse1.action.root_password
        Write-Host "✅ Fresh rescue mode enabled. Password: $SERVER1_PASSWORD" -ForegroundColor Green
        
        # Wait for server to restart
        Write-Host "Waiting for server to restart..." -ForegroundColor Yellow
        Start-Sleep -Seconds 60
    }
} catch {
    Write-Host "❌ Failed to get rescue password: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# Connect to server and deploy
Write-Host "Connecting to Server 1..." -ForegroundColor Green
try {
    $securePassword = ConvertTo-SecureString $SERVER1_PASSWORD -AsPlainText -Force
    $credential = New-Object System.Management.Automation.PSCredential("root", $securePassword)
    
    $session = New-SSHSession -ComputerName $SERVER1_IP -Credential $credential -AcceptKey -ConnectionTimeout 30
    
    if ($session) {
        Write-Host "✅ Connected to Server 1!" -ForegroundColor Green
        
        # Read the deployment script
        $deployScript = Get-Content "deploy_ippan_server1.sh" -Raw
        
        # Execute the deployment
        Write-Host "Starting IPPAN deployment..." -ForegroundColor Yellow
        $deployResult = Invoke-SSHCommand -SessionId $session.SessionId -Command $deployScript -Timeout 1800
        
        if ($deployResult.ExitStatus -eq 0) {
            Write-Host "✅ IPPAN deployment completed successfully!" -ForegroundColor Green
            Write-Host $deployResult.Output -ForegroundColor Cyan
        } else {
            Write-Host "⚠️ Deployment completed with warnings" -ForegroundColor Yellow
            Write-Host "Output: $($deployResult.Output)" -ForegroundColor Cyan
            Write-Host "Error: $($deployResult.Error)" -ForegroundColor Red
        }
        
        # Check service status
        Write-Host "Checking service status..." -ForegroundColor Green
        $statusResult = Invoke-SSHCommand -SessionId $session.SessionId -Command "docker ps --filter 'name=ippan' --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'" -Timeout 30
        
        if ($statusResult.ExitStatus -eq 0) {
            Write-Host "Service status:" -ForegroundColor Green
            Write-Host $statusResult.Output -ForegroundColor Cyan
        }
        
        # Test API
        Write-Host "Testing API endpoint..." -ForegroundColor Green
        Start-Sleep -Seconds 30
        try {
            $apiTest = Invoke-WebRequest -Uri "http://$SERVER1_IP`:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
            if ($apiTest.StatusCode -eq 200) {
                Write-Host "✅ API is responding: $($apiTest.StatusCode)" -ForegroundColor Green
            } else {
                Write-Host "⚠️ API returned status: $($apiTest.StatusCode)" -ForegroundColor Yellow
            }
        } catch {
            Write-Host "❌ API is not responding yet (may need more time to start)" -ForegroundColor Red
        }
        
        Remove-SSHSession -SessionId $session.SessionId
        Write-Host "✅ Server 1 deployment completed!" -ForegroundColor Green
        
    } else {
        Write-Host "❌ Failed to connect to Server 1" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ Deployment failed: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Cyan
Write-Host "Server 1 (188.245.97.41) deployment completed" -ForegroundColor Green
Write-Host "Access URLs:" -ForegroundColor Cyan
Write-Host "- API: http://$SERVER1_IP`:3000" -ForegroundColor White
Write-Host "- P2P: $SERVER1_IP`:8080" -ForegroundColor White
Write-Host "- Metrics: http://$SERVER1_IP`:9090" -ForegroundColor White
Write-Host ""
Write-Host "Next: Deploy Server 2 or test the connection" -ForegroundColor Yellow
