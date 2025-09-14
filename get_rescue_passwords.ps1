# Get Rescue Passwords and Deploy
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "=== Getting Rescue Passwords ===" -ForegroundColor Cyan

# Get server actions to find rescue passwords
try {
    $actions1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions" -Headers $headers -Method GET
    $rescueAction1 = $actions1.actions | Where-Object { $_.command -eq "enable_rescue" } | Sort-Object -Property id -Descending | Select-Object -First 1
    
    if ($rescueAction1) {
        Write-Host "Server 1 Rescue Password: $($rescueAction1.root_password)" -ForegroundColor Green
        $SERVER1_PASSWORD = $rescueAction1.root_password
    }
    
    $actions2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions" -Headers $headers -Method GET
    $rescueAction2 = $actions2.actions | Where-Object { $_.command -eq "enable_rescue" } | Sort-Object -Property id -Descending | Select-Object -First 1
    
    if ($rescueAction2) {
        Write-Host "Server 2 Rescue Password: $($rescueAction2.root_password)" -ForegroundColor Green
        $SERVER2_PASSWORD = $rescueAction2.root_password
    }
} catch {
    Write-Host "❌ Failed to get rescue passwords: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Testing Direct SSH Connection ===" -ForegroundColor Cyan

# Test Server 1 with password
if ($SERVER1_PASSWORD) {
    Write-Host "Testing Server 1 connection..." -ForegroundColor Green
    try {
        $securePassword1 = ConvertTo-SecureString $SERVER1_PASSWORD -AsPlainText -Force
        $credential1 = New-Object System.Management.Automation.PSCredential("root", $securePassword1)
        
        $session1 = New-SSHSession -ComputerName $SERVER1_IP -Credential $credential1 -AcceptKey -ConnectionTimeout 15
        
        if ($session1) {
            Write-Host "✅ Connected to Server 1!" -ForegroundColor Green
            
            # Quick deployment test
            $testResult = Invoke-SSHCommand -SessionId $session1.SessionId -Command "echo 'SSH connection successful'; whoami; pwd" -Timeout 30
            
            if ($testResult.ExitStatus -eq 0) {
                Write-Host "✅ SSH test successful on Server 1" -ForegroundColor Green
                Write-Host "Output: $($testResult.Output)" -ForegroundColor Gray
            }
            
            Remove-SSHSession -SessionId $session1.SessionId
        }
    } catch {
        Write-Host "❌ Failed to connect to Server 1: $($_.Exception.Message)" -ForegroundColor Red
    }
}

# Test Server 2 with password
if ($SERVER2_PASSWORD) {
    Write-Host "Testing Server 2 connection..." -ForegroundColor Green
    try {
        $securePassword2 = ConvertTo-SecureString $SERVER2_PASSWORD -AsPlainText -Force
        $credential2 = New-Object System.Management.Automation.PSCredential("root", $securePassword2)
        
        $session2 = New-SSHSession -ComputerName $SERVER2_IP -Credential $credential2 -AcceptKey -ConnectionTimeout 15
        
        if ($session2) {
            Write-Host "✅ Connected to Server 2!" -ForegroundColor Green
            
            # Quick deployment test
            $testResult = Invoke-SSHCommand -SessionId $session2.SessionId -Command "echo 'SSH connection successful'; whoami; pwd" -Timeout 30
            
            if ($testResult.ExitStatus -eq 0) {
                Write-Host "✅ SSH test successful on Server 2" -ForegroundColor Green
                Write-Host "Output: $($testResult.Output)" -ForegroundColor Gray
            }
            
            Remove-SSHSession -SessionId $session2.SessionId
        }
    } catch {
        Write-Host "❌ Failed to connect to Server 2: $($_.Exception.Message)" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "=== Next Steps ===" -ForegroundColor Cyan
Write-Host "If SSH connection works, I can deploy IPPAN on both servers" -ForegroundColor Yellow
Write-Host "If not, we'll need to use the Hetzner console manually" -ForegroundColor Yellow
