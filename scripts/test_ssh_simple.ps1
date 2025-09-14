# Simple SSH Test Script
$ErrorActionPreference = 'Stop'

Write-Host "Testing SSH access to both servers..." -ForegroundColor Green

# Test Server 1
Write-Host "Testing Server 1 (188.245.97.41)..." -ForegroundColor Yellow
try {
    $result1 = ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no ippan@188.245.97.41 "echo 'SSH to Server 1 successful'" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ Server 1 SSH is working!" -ForegroundColor Green
        Write-Host $result1 -ForegroundColor White
    } else {
        Write-Host "❌ Server 1 SSH failed: $result1" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ Server 1 SSH failed: $_" -ForegroundColor Red
}

Write-Host ""

# Test Server 2
Write-Host "Testing Server 2 (135.181.145.174)..." -ForegroundColor Yellow
try {
    $result2 = ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no ippan@135.181.145.174 "echo 'SSH to Server 2 successful'" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ Server 2 SSH is working!" -ForegroundColor Green
        Write-Host $result2 -ForegroundColor White
    } else {
        Write-Host "❌ Server 2 SSH failed: $result2" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ Server 2 SSH failed: $_" -ForegroundColor Red
}

Write-Host ""
Write-Host "SSH test complete!" -ForegroundColor Blue
