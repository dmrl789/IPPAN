# Full deployment script that sets up SSH keys then deploys
# This script will prompt for root passwords once per server

$ErrorActionPreference = "Stop"

$pubKey = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEzegKgadJCRptuIM1aEVk/EaobuPAoMcssObcEO1uF+ ippan"

$servers = @(
    @{IP="188.245.97.41"; Pass="vK3n9MKjWb9XtTsVAttP"; Name="node1"},
    @{IP="135.181.145.174"; Pass="XhH7gUA7UM9gEPPALE7p"; Name="node2"},
    @{IP="5.223.51.238"; Pass="MriVKtEK9psU9RwMCidn"; Name="node3"},
    @{IP="178.156.219.107"; Pass="hPAtPLw7hx3ndKXTW4vM"; Name="node4"}
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "IPPAN Devnet-1 Full Deployment" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Step 1: Setup SSH keys
Write-Host "STEP 1: Setting up SSH keys on all servers" -ForegroundColor Yellow
Write-Host "You will be prompted for root password for each server" -ForegroundColor Gray
Write-Host ""

$setupCmd = "if ! id -u ippan >/dev/null 2>&1; then useradd -m -s /bin/bash ippan; usermod -aG sudo ippan; fi; mkdir -p /home/ippan/.ssh; echo '$pubKey' > /home/ippan/.ssh/authorized_keys; chmod 700 /home/ippan/.ssh; chmod 600 /home/ippan/.ssh/authorized_keys; chown -R ippan:ippan /home/ippan/.ssh; echo 'ippan ALL=(ALL) NOPASSWD: ALL' > /etc/sudoers.d/ippan; chmod 440 /etc/sudoers.d/ippan"

foreach ($s in $servers) {
    Write-Host "Setting up $($s.Name) ($($s.IP))..." -ForegroundColor Green
    Write-Host "Password: $($s.Pass)" -ForegroundColor Gray
    Write-Host "Running: ssh root@$($s.IP) `"$setupCmd`"" -ForegroundColor Gray
    
    $result = ssh root@$s.IP $setupCmd 2>&1
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✓ $($s.Name) configured" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $($s.Name) failed" -ForegroundColor Red
        Write-Host "  Error: $result" -ForegroundColor Red
    }
    Write-Host ""
}

# Step 2: Verify
Write-Host "STEP 2: Verifying SSH key setup..." -ForegroundColor Yellow
Start-Sleep -Seconds 3

$allReady = $true
foreach ($s in $servers) {
    $test = ssh -o BatchMode=yes -o ConnectTimeout=5 ippan@$s.IP "whoami && sudo -n true && echo OK" 2>&1
    if ($LASTEXITCODE -eq 0 -and $test -match "OK") {
        Write-Host "  ✓ $($s.Name) : Ready" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $($s.Name) : Not ready" -ForegroundColor Red
        $allReady = $false
    }
}

if (-not $allReady) {
    Write-Host "`n⚠ Some servers are not ready. Please check SSH key setup." -ForegroundColor Yellow
    Write-Host "You can run the setup commands manually from SSH_KEY_SETUP.md" -ForegroundColor White
    exit 1
}

# Step 3: Run deployment
Write-Host "`nSTEP 3: Running automated deployment..." -ForegroundColor Yellow
Write-Host ""

& "$PSScriptRoot\devnet1_hetzner_autodeploy.ps1"

