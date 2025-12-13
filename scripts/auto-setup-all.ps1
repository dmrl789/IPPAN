# Automatic SSH key setup for all servers with new passwords
$ErrorActionPreference = "Stop"

$pubKey = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEzegKgadJCRptuIM1aEVk/EaobuPAoMcssObcEO1uF+ ippan"
$userName = "ippan-devnet"

$servers = @(
    @{IP="188.245.97.41"; Pass="LXhK3ReRVNgP"; Name="node1"},
    @{IP="135.181.145.174"; Pass="tw4qTjLpWKW3"; Name="node2"},
    @{IP="5.223.51.238"; Pass="XdXgeLisVasq"; Name="node3"},
    @{IP="178.156.219.107"; Pass="hcHdgaCrxVJe"; Name="node4"}
)

$setupCmd = @"
set -euo pipefail
PUBKEY="$pubKey"
USERNAME="$userName"

# 1) Ensure root can use SSH keys
install -d -m 700 /root/.ssh
grep -qF "`$PUBKEY" /root/.ssh/authorized_keys 2>/dev/null || echo "`$PUBKEY" >> /root/.ssh/authorized_keys
chmod 600 /root/.ssh/authorized_keys

# 2) Create ippan-devnet user + sudo + key auth
id -u `$USERNAME >/dev/null 2>&1 || useradd -m -s /bin/bash `$USERNAME
usermod -aG sudo `$USERNAME

install -d -m 700 /home/`$USERNAME/.ssh
echo "`$PUBKEY" > /home/`$USERNAME/.ssh/authorized_keys
chmod 600 /home/`$USERNAME/.ssh/authorized_keys
chown -R `$USERNAME:`$USERNAME /home/`$USERNAME/.ssh

# 3) Passwordless sudo for automation
echo "`$USERNAME ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/`$USERNAME
chmod 440 /etc/sudoers.d/`$USERNAME

echo "BOOTSTRAP_OK"
"@

Write-Host "Setting up each server..." -ForegroundColor Yellow
Write-Host ""

foreach ($s in $servers) {
    Write-Host "$($s.Name) ($($s.IP))..." -ForegroundColor Green
    
    # Save to temp file
    $tempFile = [System.IO.Path]::GetTempFileName() + ".sh"
    $setupCmd | Out-File -FilePath $tempFile -Encoding ASCII
    
    # Accept host key first
    $hostIP = $s.IP
    ssh -o StrictHostKeyChecking=accept-new "root@$hostIP" "echo ok" 2>&1 | Out-Null
    
    # Use plink with -m flag
    $result = & plink -ssh -pw $s.Pass -m $tempFile -batch "root@$hostIP" 2>&1
    
    if ($LASTEXITCODE -eq 0 -and $result -match "BOOTSTRAP_OK") {
        Write-Host "  ✓ $($s.Name) configured" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $($s.Name) - Output: $($result -join ', ')" -ForegroundColor Red
    }
    
    Remove-Item $tempFile -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "Verifying SSH access..." -ForegroundColor Cyan
Start-Sleep -Seconds 3

$allReady = $true
foreach ($s in $servers) {
    $testCmd = 'whoami && sudo -n true && echo SUDO_OK'
    $test = ssh -o BatchMode=yes -o ConnectTimeout=5 "$userName@$($s.IP)" $testCmd 2>&1
    if ($LASTEXITCODE -eq 0 -and $test -match "SUDO_OK") {
        Write-Host "  ✓ $($s.Name) : Ready" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $($s.Name) : Not ready" -ForegroundColor Red
        $allReady = $false
    }
}

if ($allReady) {
    Write-Host ""
    Write-Host "✅ All servers ready! Starting deployment...`n" -ForegroundColor Green
    & "$PSScriptRoot\devnet1_autodeploy.ps1"
} else {
    Write-Host ""
    Write-Host "⚠ Some servers not ready. Please check manually." -ForegroundColor Yellow
}

