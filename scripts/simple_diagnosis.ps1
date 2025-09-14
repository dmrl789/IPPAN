# Simple IPPAN Diagnosis Script

$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$IPPAN_USER = "ippan"

Write-Host "=== IPPAN Multi-Node Diagnosis ===" -ForegroundColor Blue
Write-Host "Server 1 (Nuremberg): $SERVER1_IP" -ForegroundColor White
Write-Host "Server 2 (Helsinki): $SERVER2_IP" -ForegroundColor White
Write-Host ""

# Test basic connectivity
Write-Host "Testing basic connectivity..." -ForegroundColor Green

Write-Host "Testing Server 1..." -ForegroundColor Yellow
try {
    $ping1 = Test-Connection -ComputerName $SERVER1_IP -Count 2 -Quiet
    if ($ping1) {
        Write-Host "✅ Server 1 is reachable" -ForegroundColor Green
    } else {
        Write-Host "❌ Server 1 is not reachable" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ Server 1 is not reachable" -ForegroundColor Red
}

Write-Host "Testing Server 2..." -ForegroundColor Yellow
try {
    $ping2 = Test-Connection -ComputerName $SERVER2_IP -Count 2 -Quiet
    if ($ping2) {
        Write-Host "✅ Server 2 is reachable" -ForegroundColor Green
    } else {
        Write-Host "❌ Server 2 is not reachable" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ Server 2 is not reachable" -ForegroundColor Red
}

# Test SSH connectivity
Write-Host ""
Write-Host "Testing SSH connectivity..." -ForegroundColor Green

Write-Host "Testing SSH to Server 1..." -ForegroundColor Yellow
try {
    $ssh1 = ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no $IPPAN_USER@$SERVER1_IP "echo 'SSH test successful'" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ SSH to Server 1 is working" -ForegroundColor Green
    } else {
        Write-Host "⚠️  SSH to Server 1 failed: $ssh1" -ForegroundColor Yellow
    }
} catch {
    Write-Host "⚠️  SSH to Server 1 failed" -ForegroundColor Yellow
}

Write-Host "Testing SSH to Server 2..." -ForegroundColor Yellow
try {
    $ssh2 = ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no $IPPAN_USER@$SERVER2_IP "echo 'SSH test successful'" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ SSH to Server 2 is working" -ForegroundColor Green
    } else {
        Write-Host "⚠️  SSH to Server 2 failed: $ssh2" -ForegroundColor Yellow
    }
} catch {
    Write-Host "⚠️  SSH to Server 2 failed" -ForegroundColor Yellow
}

# Check SSH key
Write-Host ""
Write-Host "Checking SSH key..." -ForegroundColor Green

$sshKeyPath = "$env:USERPROFILE\.ssh\id_rsa"
if (Test-Path $sshKeyPath) {
    Write-Host "✅ SSH key exists" -ForegroundColor Green
    $publicKey = Get-Content "$sshKeyPath.pub" -ErrorAction SilentlyContinue
    if ($publicKey) {
        Write-Host ""
        Write-Host "Your SSH Public Key:" -ForegroundColor Cyan
        Write-Host $publicKey -ForegroundColor White
        Write-Host ""
        Write-Host "IMPORTANT: Add this key to both servers!" -ForegroundColor Yellow
    }
} else {
    Write-Host "⚠️  SSH key does not exist" -ForegroundColor Yellow
    Write-Host "Run: ssh-keygen -t rsa -b 4096 -f $sshKeyPath" -ForegroundColor Cyan
}

Write-Host ""
Write-Host "=== Next Steps ===" -ForegroundColor Blue
Write-Host "1. Set up SSH access using the public key above" -ForegroundColor White
Write-Host "2. Once SSH is working, deploy IPPAN services" -ForegroundColor White
Write-Host "3. Run verification script to test the setup" -ForegroundColor White
Write-Host ""
Write-Host "Manual deployment commands:" -ForegroundColor Cyan
Write-Host "ssh $IPPAN_USER@$SERVER1_IP" -ForegroundColor White
Write-Host "ssh $IPPAN_USER@$SERVER2_IP" -ForegroundColor White
