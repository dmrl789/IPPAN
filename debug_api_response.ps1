# Debug API Response
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "=== Debug API Response ===" -ForegroundColor Cyan

# Get all actions for Server 1
try {
    $actions1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions" -Headers $headers -Method GET
    Write-Host "Server 1 Actions:" -ForegroundColor Green
    $actions1.actions | ForEach-Object {
        Write-Host "Action: $($_.command), Status: $($_.status), ID: $($_.id)" -ForegroundColor White
        if ($_.root_password) {
            Write-Host "  Password: $($_.root_password)" -ForegroundColor Yellow
        }
    }
} catch {
    Write-Host "❌ Failed to get Server 1 actions: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "Server 2 Actions:" -ForegroundColor Green
try {
    $actions2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions" -Headers $headers -Method GET
    $actions2.actions | ForEach-Object {
        Write-Host "Action: $($_.command), Status: $($_.status), ID: $($_.id)" -ForegroundColor White
        if ($_.root_password) {
            Write-Host "  Password: $($_.root_password)" -ForegroundColor Yellow
        }
    }
} catch {
    Write-Host "❌ Failed to get Server 2 actions: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Enabling Fresh Rescue Mode ===" -ForegroundColor Cyan

# Enable fresh rescue mode
$rescueBody = @{
    rescue = "linux64"
} | ConvertTo-Json

# Server 1
Write-Host "Enabling fresh rescue mode on Server 1..." -ForegroundColor Green
try {
    $rescueResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody
    Write-Host "Server 1 Response:" -ForegroundColor Yellow
    $rescueResponse1 | ConvertTo-Json -Depth 10 | Write-Host
    
    if ($rescueResponse1.root_password) {
        Write-Host "Server 1 Fresh Password: $($rescueResponse1.root_password)" -ForegroundColor Green
        $SERVER1_PASSWORD = $rescueResponse1.root_password
    }
} catch {
    Write-Host "❌ Failed to enable rescue mode on Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

# Server 2
Write-Host "Enabling fresh rescue mode on Server 2..." -ForegroundColor Green
try {
    $rescueResponse2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody
    Write-Host "Server 2 Response:" -ForegroundColor Yellow
    $rescueResponse2 | ConvertTo-Json -Depth 10 | Write-Host
    
    if ($rescueResponse2.root_password) {
        Write-Host "Server 2 Fresh Password: $($rescueResponse2.root_password)" -ForegroundColor Green
        $SERVER2_PASSWORD = $rescueResponse2.root_password
    }
} catch {
    Write-Host "❌ Failed to enable rescue mode on Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Testing SSH with Fresh Passwords ===" -ForegroundColor Cyan

# Test Server 1
if ($SERVER1_PASSWORD) {
    Write-Host "Testing Server 1 SSH with password: $SERVER1_PASSWORD" -ForegroundColor Green
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
    Write-Host "Testing Server 2 SSH with password: $SERVER2_PASSWORD" -ForegroundColor Green
    try {
        $result2 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -pw $SERVER2_PASSWORD root@135.181.145.174 "echo 'SSH test successful'; whoami; pwd"
        Write-Host "✅ Server 2 SSH test successful" -ForegroundColor Green
        Write-Host "Output: $result2" -ForegroundColor Gray
    } catch {
        Write-Host "❌ Server 2 SSH test failed: $($_.Exception.Message)" -ForegroundColor Red
    }
}
