# Get Passwords Simple
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "Getting fresh rescue passwords..." -ForegroundColor Green

# Server 1
$actions1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/108447288/actions" -Headers $headers -Method GET
$rescueAction1 = $actions1.actions | Where-Object { $_.command -eq "enable_rescue" } | Sort-Object -Property id -Descending | Select-Object -First 1
Write-Host "Server 1 Password: $($rescueAction1.root_password)" -ForegroundColor Yellow

# Server 2
$actions2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/108535607/actions" -Headers $headers -Method GET
$rescueAction2 = $actions2.actions | Where-Object { $_.command -eq "enable_rescue" } | Sort-Object -Property id -Descending | Select-Object -First 1
Write-Host "Server 2 Password: $($rescueAction2.root_password)" -ForegroundColor Yellow

Write-Host ""
Write-Host "Use these passwords to access the servers via Hetzner Console" -ForegroundColor Green
