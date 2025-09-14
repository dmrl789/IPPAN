# Check and Fix Rescue Mode
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "=== Checking and Fixing Rescue Mode ===" -ForegroundColor Cyan
Write-Host ""

# Check current server status
Write-Host "Checking server status..." -ForegroundColor Green
try {
    $server1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID" -Headers $headers -Method GET
    $server2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID" -Headers $headers -Method GET
    
    Write-Host "Server 1 Status: $($server1.server.status)" -ForegroundColor White
    Write-Host "Server 1 Rescue: $($server1.server.rescue_enabled)" -ForegroundColor White
    Write-Host "Server 2 Status: $($server2.server.status)" -ForegroundColor White
    Write-Host "Server 2 Rescue: $($server2.server.rescue_enabled)" -ForegroundColor White
} catch {
    Write-Host "❌ Failed to get server status: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Disabling and Re-enabling Rescue Mode ===" -ForegroundColor Green

# Disable rescue mode first
Write-Host "Disabling rescue mode on both servers..." -ForegroundColor Yellow
try {
    $disable1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/disable_rescue" -Headers $headers -Method POST
    Write-Host "✅ Rescue mode disabled on Server 1" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to disable rescue mode on Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

try {
    $disable2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/disable_rescue" -Headers $headers -Method POST
    Write-Host "✅ Rescue mode disabled on Server 2" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to disable rescue mode on Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "Waiting for servers to restart..." -ForegroundColor Yellow
Start-Sleep -Seconds 30

# Re-enable rescue mode with password authentication
Write-Host "Re-enabling rescue mode with password authentication..." -ForegroundColor Yellow

$rescueBody = @{
    rescue = "linux64"
} | ConvertTo-Json

# Server 1
try {
    $rescueResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody
    Write-Host "✅ Rescue mode enabled on Server 1" -ForegroundColor Green
    Write-Host "Server 1 Rescue Password: $($rescueResponse1.action.root_password)" -ForegroundColor Yellow
    $SERVER1_PASSWORD = $rescueResponse1.action.root_password
} catch {
    Write-Host "❌ Failed to enable rescue mode on Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

# Server 2
try {
    $rescueResponse2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody
    Write-Host "✅ Rescue mode enabled on Server 2" -ForegroundColor Green
    Write-Host "Server 2 Rescue Password: $($rescueResponse2.action.root_password)" -ForegroundColor Yellow
    $SERVER2_PASSWORD = $rescueResponse2.action.root_password
} catch {
    Write-Host "❌ Failed to enable rescue mode on Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "Waiting for rescue mode to be ready..." -ForegroundColor Yellow
Start-Sleep -Seconds 30

Write-Host ""
Write-Host "=== Testing SSH Connection ===" -ForegroundColor Cyan

# Test Server 1
if ($SERVER1_PASSWORD) {
    Write-Host "Testing Server 1 SSH connection..." -ForegroundColor Green
    try {
        $result1 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -pw $SERVER1_PASSWORD root@188.245.97.41 "echo 'SSH test successful'; whoami; pwd"
        Write-Host "✅ Server 1 SSH test successful" -ForegroundColor Green
        Write-Host "Output: $result1" -ForegroundColor Gray
    } catch {
        Write-Host "❌ Server 1 SSH test failed: $($_.Exception.Message)" -ForegroundColor Red
    }
}

# Test Server 2
if ($SERVER2_PASSWORD) {
    Write-Host "Testing Server 2 SSH connection..." -ForegroundColor Green
    try {
        $result2 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -pw $SERVER2_PASSWORD root@135.181.145.174 "echo 'SSH test successful'; whoami; pwd"
        Write-Host "✅ Server 2 SSH test successful" -ForegroundColor Green
        Write-Host "Output: $result2" -ForegroundColor Gray
    } catch {
        Write-Host "❌ Server 2 SSH test failed: $($_.Exception.Message)" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "=== Next Steps ===" -ForegroundColor Cyan
Write-Host "If SSH connection works, we can proceed with IPPAN deployment" -ForegroundColor Yellow
Write-Host "If not, we may need to use the Hetzner console manually" -ForegroundColor Yellow
