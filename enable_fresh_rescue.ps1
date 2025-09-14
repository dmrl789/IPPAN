# Enable Fresh Rescue Mode and Get Passwords
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "=== Enabling Fresh Rescue Mode ===" -ForegroundColor Cyan

# Get SSH key
try {
    $sshKeysResponse = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/ssh_keys" -Headers $headers -Method GET
    $sshKeys = $sshKeysResponse.ssh_keys
    
    if ($sshKeys.Count -gt 0) {
        $sshKeyId = $sshKeys[0].id
        Write-Host "Using SSH key: $($sshKeys[0].name) (ID: $sshKeyId)" -ForegroundColor Green
    }
} catch {
    Write-Host "❌ Failed to get SSH keys: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# Enable rescue mode on Server 1
Write-Host "Enabling rescue mode on Server 1..." -ForegroundColor Green
$rescueBody1 = @{
    rescue = "linux64"
    ssh_keys = @($sshKeyId)
} | ConvertTo-Json

try {
    $rescueResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody1
    Write-Host "✅ Server 1 rescue mode enabled" -ForegroundColor Green
    Write-Host "Server 1 Password: $($rescueResponse1.action.root_password)" -ForegroundColor Yellow
    $SERVER1_PASSWORD = $rescueResponse1.action.root_password
} catch {
    Write-Host "❌ Failed to enable rescue mode on Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

# Enable rescue mode on Server 2
Write-Host "Enabling rescue mode on Server 2..." -ForegroundColor Green
$rescueBody2 = @{
    rescue = "linux64"
    ssh_keys = @($sshKeyId)
} | ConvertTo-Json

try {
    $rescueResponse2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody2
    Write-Host "✅ Server 2 rescue mode enabled" -ForegroundColor Green
    Write-Host "Server 2 Password: $($rescueResponse2.action.root_password)" -ForegroundColor Yellow
    $SERVER2_PASSWORD = $rescueResponse2.action.root_password
} catch {
    Write-Host "❌ Failed to enable rescue mode on Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Manual Deployment Instructions ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Now you can access both servers via Hetzner Console:" -ForegroundColor Yellow
Write-Host ""
Write-Host "Server 1 (188.245.97.41):" -ForegroundColor Green
Write-Host "  Username: root" -ForegroundColor White
Write-Host "  Password: $SERVER1_PASSWORD" -ForegroundColor White
Write-Host ""
Write-Host "Server 2 (135.181.145.174):" -ForegroundColor Green
Write-Host "  Username: root" -ForegroundColor White
Write-Host "  Password: $SERVER2_PASSWORD" -ForegroundColor White
Write-Host ""
Write-Host "Run the deployment script on both servers to complete the setup!" -ForegroundColor Green
