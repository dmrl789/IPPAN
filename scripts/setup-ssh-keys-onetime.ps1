# One-time SSH key setup using root credentials
$ErrorActionPreference = "Stop"

$servers = @{
    "188.245.97.41" = @{Pass='vK3n9MKjWb9XtTsVAttP'; Name='node1'}
    "135.181.145.174" = @{Pass='XhH7gUA7UM9gEPPALE7p'; Name='node2'}
    "5.223.51.238" = @{Pass='MriVKtEK9psU9RwMCidn'; Name='node3'}
    "178.156.219.107" = @{Pass='hPAtPLw7hx3ndKXTW4vM'; Name='node4'}
}

$pubKeyPath = "$env:USERPROFILE\.ssh\id_ed25519.pub"
if (-not (Test-Path $pubKeyPath)) {
    $pubKeyPath = "$env:USERPROFILE\.ssh\id_rsa.pub"
}
if (-not (Test-Path $pubKeyPath)) {
    Write-Host "ERROR: No SSH public key found!" -ForegroundColor Red
    exit 1
}
$pubKey = (Get-Content $pubKeyPath).Trim()

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "One-time SSH Key Setup" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

$plink = Get-Command plink -ErrorAction SilentlyContinue
if (-not $plink) {
    Write-Host "ERROR: plink not found. Please install PuTTY." -ForegroundColor Red
    exit 1
}

foreach ($ip in $servers.Keys) {
    $server = $servers[$ip]
    Write-Host "Setting up $($server.Name) ($ip)..." -ForegroundColor Green
    
    # Accept host key first via SSH
    ssh -o StrictHostKeyChecking=accept-new root@$ip "echo 'host key accepted'" 2>&1 | Out-Null
    
    $setupCmd = "if ! id -u ippan >/dev/null 2>&1; then useradd -m -s /bin/bash ippan; usermod -aG sudo ippan; fi; mkdir -p /home/ippan/.ssh; echo '$pubKey' > /home/ippan/.ssh/authorized_keys; chmod 700 /home/ippan/.ssh; chmod 600 /home/ippan/.ssh/authorized_keys; chown -R ippan:ippan /home/ippan/.ssh; echo 'ippan ALL=(ALL) NOPASSWD: ALL' > /etc/sudoers.d/ippan; chmod 440 /etc/sudoers.d/ippan"
    
    $result = & plink -ssh -pw $server.Pass -batch root@$ip $setupCmd 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✓ Setup complete" -ForegroundColor Green
    } else {
        Write-Host "  ✗ Failed: $result" -ForegroundColor Red
    }
}

Write-Host "Verifying SSH key setup..." -ForegroundColor Yellow
Start-Sleep -Seconds 2

foreach ($ip in $servers.Keys) {
    $server = $servers[$ip]
    $test = ssh -o BatchMode=yes -o ConnectTimeout=5 ippan@$ip "whoami" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✓ $($server.Name): SSH key working" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $($server.Name): SSH key not working" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "If all checks pass, run:" -ForegroundColor Cyan
Write-Host "  powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\devnet1_hetzner_autodeploy.ps1" -ForegroundColor White
