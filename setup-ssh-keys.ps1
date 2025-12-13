# Helper script to add SSH public key to all Hetzner servers
# Run this AFTER you've manually SSH'd to each server at least once

$NODE1_PUB = "188.245.97.41"
$NODE2_PUB = "135.181.145.174"
$NODE3_PUB = "5.223.51.238"
$NODE4_PUB = "178.156.219.107"

$SERVERS = @($NODE1_PUB, $NODE2_PUB, $NODE3_PUB, $NODE4_PUB)

# Get public key
$pubKeyPath = "$env:USERPROFILE\.ssh\id_ed25519.pub"
if (-not (Test-Path $pubKeyPath)) {
    $pubKeyPath = "$env:USERPROFILE\.ssh\id_rsa.pub"
}

if (-not (Test-Path $pubKeyPath)) {
    Write-Host "ERROR: No SSH public key found!" -ForegroundColor Red
    exit 1
}

$pubKey = Get-Content $pubKeyPath

Write-Host "Your public key:" -ForegroundColor Cyan
Write-Host $pubKey
Write-Host ""
Write-Host "To add this key to each server, run:" -ForegroundColor Yellow
Write-Host ""

foreach ($server in $SERVERS) {
    Write-Host "ssh ippan@$server 'mkdir -p ~/.ssh && echo ""$pubKey"" >> ~/.ssh/authorized_keys && chmod 700 ~/.ssh && chmod 600 ~/.ssh/authorized_keys'" -ForegroundColor White
}

Write-Host ""
Write-Host "Or manually add the key above to ~/.ssh/authorized_keys on each server" -ForegroundColor Yellow

