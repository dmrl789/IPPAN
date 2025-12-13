# Final IPPAN Devnet-1 Deployment Script
# Handles password expiration and full deployment

$ErrorActionPreference = "Continue"

# Server credentials (passwords need to be changed on first login)
$servers = @{
    "188.245.97.41" = @{Pass="vK3n9MKjWb9XtTsVAttP"; NewPass="vK3n9MKjWb9XtTsVAttP"; PrivIP="10.0.0.2"; Name="node1"; Bootstrap=""}
    "135.181.145.174" = @{Pass="XhH7gUA7UM9gEPPALE7p"; NewPass="XhH7gUA7UM9gEPPALE7p"; PrivIP="10.0.0.3"; Name="node2"; Bootstrap="10.0.0.2"}
    "5.223.51.238" = @{Pass="MriVKtEK9psU9RwMCidn"; NewPass="MriVKtEK9psU9RwMCidn"; PrivIP=""; Name="node3"; Bootstrap="188.245.97.41"}
    "178.156.219.107" = @{Pass="hPAtPLw7hx3ndKXTW4vM"; NewPass="hPAtPLw7hx3ndKXTW4vM"; PrivIP=""; Name="node4"; Bootstrap="188.245.97.41"}
}

# Get public key
$pubKeyPath = "$env:USERPROFILE\.ssh\id_ed25519.pub"
if (-not (Test-Path $pubKeyPath)) {
    $pubKeyPath = "$env:USERPROFILE\.ssh\id_rsa.pub"
}
$pubKey = (Get-Content $pubKeyPath).Trim()

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "IPPAN Devnet-1 Deployment" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "NOTE: Hetzner servers require password change on first login." -ForegroundColor Yellow
Write-Host "You'll need to manually change passwords first, or use the commands below." -ForegroundColor Yellow
Write-Host ""

Write-Host "STEP 1: Change root passwords (run these manually first):" -ForegroundColor Yellow
Write-Host ""

foreach ($ip in $servers.Keys) {
    $server = $servers[$ip]
    Write-Host "# Change password on $($server.Name) ($ip)" -ForegroundColor Green
    Write-Host "ssh root@$ip" -ForegroundColor White
    Write-Host "# When prompted, enter current password: $($server.Pass)" -ForegroundColor Gray
    Write-Host "# Then set new password (use same or different):" -ForegroundColor Gray
    Write-Host "passwd" -ForegroundColor White
    Write-Host ""
}

Write-Host "After passwords are changed, update NewPass in this script and continue." -ForegroundColor Cyan
Write-Host ""

# Setup ippan user function
function Setup-IppanUser {
    param($ip, $pass, $name, $pubKey)
    
    Write-Host "Setting up ippan user on $name..." -ForegroundColor Green
    
    $setupCmd = "if ! id -u ippan >/dev/null 2>&1; then useradd -m -s /bin/bash ippan; usermod -aG sudo ippan; echo 'ippan ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers.d/ippan; mkdir -p /home/ippan/.ssh; echo '$pubKey' > /home/ippan/.ssh/authorized_keys; chmod 700 /home/ippan/.ssh; chmod 600 /home/ippan/.ssh/authorized_keys; chown -R ippan:ippan /home/ippan/.ssh; echo 'User created'; else echo 'User exists'; mkdir -p /home/ippan/.ssh; echo '$pubKey' > /home/ippan/.ssh/authorized_keys; chmod 700 /home/ippan/.ssh; chmod 600 /home/ippan/.ssh/authorized_keys; chown -R ippan:ippan /home/ippan/.ssh; fi"
    
    # Get host key fingerprint first
    $hostKey = ssh-keyscan -t ed25519 $ip 2>&1 | Select-String "ssh-ed25519"
    
    if ($hostKey) {
        $fingerprint = ($hostKey -split " ")[2]
        $result = & plink -ssh -pw $pass -hostkey "ssh-ed25519 255 $fingerprint" -batch root@$ip $setupCmd 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Host "  ✓ User setup complete" -ForegroundColor Green
            return $true
        } else {
            Write-Host "  ✗ Failed: $result" -ForegroundColor Red
            return $false
        }
    } else {
        Write-Host "  Manual setup required" -ForegroundColor Yellow
        Write-Host "  Run: ssh root@$ip" -ForegroundColor White
        Write-Host "  Then paste the setup command" -ForegroundColor Gray
        return $false
    }
}

# Continue with deployment after password change
Write-Host "STEP 2: After passwords changed, uncomment the section below to continue:" -ForegroundColor Yellow
Write-Host ""

$continueScript = @'

# Uncomment to continue after password change:

# Setup ippan users
# foreach ($ip in $servers.Keys) {
#     $server = $servers[$ip]
#     Setup-IppanUser -ip $ip -pass $server.NewPass -name $server.Name -pubKey $pubKey
# }

# Verify SSH
# Start-Sleep -Seconds 2
# foreach ($ip in $servers.Keys) {
#     $server = $servers[$ip]
#     $test = ssh -o ConnectTimeout=5 -o BatchMode=yes ippan@$ip "hostname" 2>&1
#     if ($LASTEXITCODE -eq 0) {
#         Write-Host "✓ $($server.Name): SSH working" -ForegroundColor Green
#     }
# }

# Phase 2: Deploy repo
# foreach ($ip in $servers.Keys) {
#     $server = $servers[$ip]
#     ssh ippan@$ip 'sudo mkdir -p /opt/ippan && sudo chown -R ippan:ippan /opt/ippan'
#     ssh ippan@$ip 'cd /opt/ippan && (test -d .git && git fetch origin && git reset --hard origin/master || git clone https://github.com/dmrl789/IPPAN .) && git checkout master && git pull --ff-only'
#     ssh ippan@$ip 'cd /opt/ippan && chmod +x deploy/hetzner/scripts/setup-node.sh || true'
# }

# Phase 3: Run setup scripts (15-30 min each)
# foreach ($ip in $servers.Keys) {
#     $server = $servers[$ip]
#     if ($server.Bootstrap) {
#         ssh ippan@$ip "cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh $($server.Name) $($server.Bootstrap)"
#     } else {
#         ssh ippan@$ip "cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh $($server.Name)"
#     }
# }

# Phase 4-7: Continue with service startup, logs, firewall, validation
# (See RUN_DEPLOYMENT.md for full commands)

'@

Write-Host $continueScript -ForegroundColor Gray

Write-Host "`nFor immediate deployment, use the commands in RUN_DEPLOYMENT.md" -ForegroundColor Cyan

