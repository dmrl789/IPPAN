# Test SSH Access Script

$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$IPPAN_USER = "ippan"

Write-Host "=== Testing SSH Access ===" -ForegroundColor Blue
Write-Host ""

Write-Host "Testing SSH to Server 1 (Nuremberg)..." -ForegroundColor Yellow
try {
    $ssh1 = ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no $IPPAN_USER@$SERVER1_IP "echo 'SSH to Server 1 successful'" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ SSH to Server 1 is working!" -ForegroundColor Green
        Write-Host $ssh1 -ForegroundColor White
    } else {
        Write-Host "❌ SSH to Server 1 failed: $ssh1" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ SSH to Server 1 failed" -ForegroundColor Red
}

Write-Host ""
Write-Host "Testing SSH to Server 2 (Helsinki)..." -ForegroundColor Yellow
try {
    $ssh2 = ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no $IPPAN_USER@$SERVER2_IP "echo 'SSH to Server 2 successful'" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ SSH to Server 2 is working!" -ForegroundColor Green
        Write-Host $ssh2 -ForegroundColor White
    } else {
        Write-Host "❌ SSH to Server 2 failed: $ssh2" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ SSH to Server 2 failed" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Next Steps ===" -ForegroundColor Blue
Write-Host "If both SSH tests pass, you can now run:" -ForegroundColor White
Write-Host "./scripts/deploy_server1.sh" -ForegroundColor Cyan
Write-Host "./scripts/deploy_server2_connect.sh" -ForegroundColor Cyan
Write-Host ""
Write-Host "If SSH tests fail, follow the instructions in FINAL_DEPLOYMENT_GUIDE.md" -ForegroundColor Yellow
