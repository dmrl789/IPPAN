# Setup SSH keys for ippan-devnet user on all servers
# Uses root passwords to set up the user and SSH keys

$ErrorActionPreference = "Stop"

$pubKey = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEzegKgadJCRptuIM1aEVk/EaobuPAoMcssObcEO1uF+ ippan"
$userName = "ippan-devnet"

$servers = @(
    @{IP="188.245.97.41"; Pass=""; Name="node1"},  # Password will be provided
    @{IP="135.181.145.174"; Pass="XhH7gUA7UM9gEPPALE7p"; Name="node2"},
    @{IP="5.223.51.238"; Pass="MriVKtEK9psU9RwMCidn"; Name="node3"},
    @{IP="178.156.219.107"; Pass="hPAtPLw7hx3ndKXTW4vM"; Name="node4"}
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Setting up ippan-devnet user on all servers" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Get password for node1
if ([string]::IsNullOrWhiteSpace($servers[0].Pass)) {
    Write-Host "Please provide the new password for node1 (188.245.97.41):" -ForegroundColor Yellow
    $node1Pass = Read-Host -AsSecureString
    $BSTR = [System.Runtime.InteropServices.Marshal]::SecureStringToBSTR($node1Pass)
    $servers[0].Pass = [System.Runtime.InteropServices.Marshal]::PtrToStringAuto($BSTR)
}

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
    
    # Save command to temp file
    $tempFile = [System.IO.Path]::GetTempFileName() + ".sh"
    $setupCmd | Out-File -FilePath $tempFile -Encoding ASCII
    
    # Use plink to execute (will prompt for password if needed)
    Write-Host "  Connecting and setting up..." -ForegroundColor Gray
    
    # Try with plink first
    $result = & plink -ssh -pw $s.Pass -batch root@$s.IP "bash -s" < $tempFile 2>&1
    
    if ($LASTEXITCODE -eq 0 -and $result -match "BOOTSTRAP_OK") {
        Write-Host "  ✓ $($s.Name) configured" -ForegroundColor Green
    } else {
        Write-Host "  ⚠ $($s.Name) - trying interactive SSH..." -ForegroundColor Yellow
        Write-Host "  Run manually: ssh -tt root@$($s.IP)" -ForegroundColor Gray
        Write-Host "  Then paste the contents of: $tempFile" -ForegroundColor Gray
    }
    
    Remove-Item $tempFile -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "Verifying setup..." -ForegroundColor Cyan
Start-Sleep -Seconds 3

$allReady = $true
foreach ($s in $servers) {
    $test = ssh -o BatchMode=yes -o ConnectTimeout=5 "$userName@$($s.IP)" "whoami && sudo -n true && echo SUDO_OK" 2>&1
    if ($LASTEXITCODE -eq 0 -and $test -match "SUDO_OK") {
        Write-Host "  ✓ $($s.Name) : Ready" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $($s.Name) : Not ready" -ForegroundColor Red
        $allReady = $false
    }
}

if ($allReady) {
    Write-Host ""
    Write-Host "✅ All servers ready with user 'ippan-devnet'!" -ForegroundColor Green
    Write-Host "You can now run the deployment script." -ForegroundColor Cyan
} else {
    Write-Host ""
    Write-Host "⚠ Some servers need manual setup." -ForegroundColor Yellow
}

