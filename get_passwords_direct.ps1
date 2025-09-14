# Get Rescue Passwords Directly
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "=== Getting Rescue Passwords Directly ===" -ForegroundColor Cyan

# Enable rescue mode and capture the response
$rescueBody = @{
    rescue = "linux64"
} | ConvertTo-Json

# Server 1
Write-Host "Enabling rescue mode on Server 1..." -ForegroundColor Green
try {
    $rescueResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody
    
    Write-Host "Server 1 Response:" -ForegroundColor Yellow
    $rescueResponse1 | ConvertTo-Json -Depth 10 | Write-Host
    
    if ($rescueResponse1.action.root_password) {
        Write-Host "Server 1 Rescue Password: $($rescueResponse1.action.root_password)" -ForegroundColor Green
        $SERVER1_PASSWORD = $rescueResponse1.action.root_password
    }
} catch {
    Write-Host "❌ Failed to enable rescue mode on Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

# Server 2
Write-Host "Enabling rescue mode on Server 2..." -ForegroundColor Green
try {
    $rescueResponse2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody
    
    Write-Host "Server 2 Response:" -ForegroundColor Yellow
    $rescueResponse2 | ConvertTo-Json -Depth 10 | Write-Host
    
    if ($rescueResponse2.action.root_password) {
        Write-Host "Server 2 Rescue Password: $($rescueResponse2.action.root_password)" -ForegroundColor Green
        $SERVER2_PASSWORD = $rescueResponse2.action.root_password
    }
} catch {
    Write-Host "❌ Failed to enable rescue mode on Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Testing SSH with Passwords ===" -ForegroundColor Cyan

if ($SERVER1_PASSWORD) {
    Write-Host "Testing Server 1 SSH with password: $SERVER1_PASSWORD" -ForegroundColor Green
    try {
        $securePassword1 = ConvertTo-SecureString $SERVER1_PASSWORD -AsPlainText -Force
        $credential1 = New-Object System.Management.Automation.PSCredential("root", $securePassword1)
        
        $session1 = New-SSHSession -ComputerName "188.245.97.41" -Credential $credential1 -AcceptKey -ConnectionTimeout 15
        
        if ($session1) {
            Write-Host "✅ Connected to Server 1!" -ForegroundColor Green
            
            $testResult = Invoke-SSHCommand -SessionId $session1.SessionId -Command "echo 'SSH test successful'; whoami; pwd" -Timeout 30
            Write-Host "Test result: $($testResult.Output)" -ForegroundColor Gray
            
            Remove-SSHSession -SessionId $session1.SessionId
        }
    } catch {
        Write-Host "❌ Failed to connect to Server 1: $($_.Exception.Message)" -ForegroundColor Red
    }
}

if ($SERVER2_PASSWORD) {
    Write-Host "Testing Server 2 SSH with password: $SERVER2_PASSWORD" -ForegroundColor Green
    try {
        $securePassword2 = ConvertTo-SecureString $SERVER2_PASSWORD -AsPlainText -Force
        $credential2 = New-Object System.Management.Automation.PSCredential("root", $securePassword2)
        
        $session2 = New-SSHSession -ComputerName "135.181.145.174" -Credential $credential2 -AcceptKey -ConnectionTimeout 15
        
        if ($session2) {
            Write-Host "✅ Connected to Server 2!" -ForegroundColor Green
            
            $testResult = Invoke-SSHCommand -SessionId $session2.SessionId -Command "echo 'SSH test successful'; whoami; pwd" -Timeout 30
            Write-Host "Test result: $($testResult.Output)" -ForegroundColor Gray
            
            Remove-SSHSession -SessionId $session2.SessionId
        }
    } catch {
        Write-Host "❌ Failed to connect to Server 2: $($_.Exception.Message)" -ForegroundColor Red
    }
}
