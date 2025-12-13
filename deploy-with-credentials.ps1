# IPPAN Devnet-1 Deployment with Root Credentials
# This script sets up ippan user and deploys the network

$ErrorActionPreference = "Continue"

# Server credentials
$servers = @{
    "188.245.97.41" = @{Pass="vK3n9MKjWb9XtTsVAttP"; PrivIP="10.0.0.2"; Name="node1"; Bootstrap=""}
    "135.181.145.174" = @{Pass="XhH7gUA7UM9gEPPALE7p"; PrivIP="10.0.0.3"; Name="node2"; Bootstrap="10.0.0.2"}
    "5.223.51.238" = @{Pass="MriVKtEK9psU9RwMCidn"; PrivIP=""; Name="node3"; Bootstrap="188.245.97.41"}
    "178.156.219.107" = @{Pass="hPAtPLw7hx3ndKXTW4vM"; PrivIP=""; Name="node4"; Bootstrap="188.245.97.41"}
}

# Get public key
$pubKeyPath = "$env:USERPROFILE\.ssh\id_ed25519.pub"
if (-not (Test-Path $pubKeyPath)) {
    $pubKeyPath = "$env:USERPROFILE\.ssh\id_rsa.pub"
}
$pubKey = Get-Content $pubKeyPath

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "IPPAN Devnet-1 Deployment" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Function to execute SSH command with password
function Invoke-SSHWithPassword {
    param(
        [string]$Host,
        [string]$Password,
        [string]$Command
    )
    
    # Create a here-string script for expect-like behavior
    # For Windows, we'll use plink if available, or provide manual steps
    Write-Host "Executing on $Host : $Command" -ForegroundColor Gray
    
    # Try using ssh with password via stdin (limited on Windows)
    # For now, return the command to execute
    return "ssh root@$Host `"$Command`""
}

Write-Host "STEP 1: Setting up ippan user on all servers" -ForegroundColor Yellow
Write-Host "Run these commands (enter password when prompted):" -ForegroundColor Cyan
Write-Host ""

foreach ($ip in $servers.Keys) {
    $server = $servers[$ip]
    $setupScript = @"
if ! id -u ippan >/dev/null 2>&1; then
    useradd -m -s /bin/bash ippan
    usermod -aG sudo ippan
    echo 'ippan ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers.d/ippan
    mkdir -p /home/ippan/.ssh
    echo '$pubKey' > /home/ippan/.ssh/authorized_keys
    chmod 700 /home/ippan/.ssh
    chmod 600 /home/ippan/.ssh/authorized_keys
    chown -R ippan:ippan /home/ippan/.ssh
    echo 'User ippan created'
else
    echo 'User ippan exists, updating SSH key'
    mkdir -p /home/ippan/.ssh
    echo '$pubKey' > /home/ippan/.ssh/authorized_keys
    chmod 700 /home/ippan/.ssh
    chmod 600 /home/ippan/.ssh/authorized_keys
    chown -R ippan:ippan /home/ippan/.ssh
fi
"@
    
    Write-Host "# Setup $($server.Name) ($ip)" -ForegroundColor Green
    Write-Host "ssh root@$ip" -ForegroundColor White
    Write-Host "Then paste and run:" -ForegroundColor Gray
    Write-Host $setupScript -ForegroundColor Cyan
    Write-Host ""
}

Write-Host "After setting up users, verify:" -ForegroundColor Yellow
foreach ($ip in $servers.Keys) {
    Write-Host "ssh ippan@$ip 'hostname'" -ForegroundColor White
}
Write-Host ""

Write-Host "STEP 2: Deploy repository and setup" -ForegroundColor Yellow
Write-Host "After ippan users are set up, run:" -ForegroundColor Cyan
Write-Host ""

# Phase 2: Copy repo
Write-Host "# Phase 2: Setup directories and clone repo" -ForegroundColor Green
foreach ($ip in $servers.Keys) {
    $server = $servers[$ip]
    Write-Host "ssh ippan@$ip 'sudo mkdir -p /opt/ippan && sudo chown -R ippan:ippan /opt/ippan'" -ForegroundColor White
    Write-Host "ssh ippan@$ip 'cd /opt/ippan && (test -d .git && git fetch origin && git reset --hard origin/master || git clone https://github.com/dmrl789/IPPAN .) && git checkout master && git pull --ff-only'" -ForegroundColor White
    Write-Host "ssh ippan@$ip 'cd /opt/ippan && chmod +x deploy/hetzner/scripts/setup-node.sh || true'" -ForegroundColor White
    Write-Host ""
}

# Phase 3: Run setup scripts
Write-Host "# Phase 3: Run setup scripts" -ForegroundColor Green
foreach ($ip in $servers.Keys) {
    $server = $servers[$ip]
    if ($server.Bootstrap) {
        Write-Host "ssh ippan@$ip 'cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh $($server.Name) $($server.Bootstrap)'" -ForegroundColor White
    } else {
        Write-Host "ssh ippan@$ip 'cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh $($server.Name)'" -ForegroundColor White
    }
}
Write-Host ""

# Phase 4: Start services
Write-Host "# Phase 4: Start services" -ForegroundColor Green
foreach ($ip in $servers.Keys) {
    Write-Host "ssh ippan@$ip 'sudo systemctl daemon-reload && sudo systemctl enable ippan-node && sudo systemctl restart ippan-node && sleep 2 && sudo systemctl --no-pager --full status ippan-node | head -n 30'" -ForegroundColor White
}
Write-Host ""

# Phase 5: Check logs
Write-Host "# Phase 5: Check logs" -ForegroundColor Green
foreach ($ip in $servers.Keys) {
    Write-Host "ssh ippan@$ip 'sudo tail -n 80 /var/log/ippan/node.log || sudo journalctl -u ippan-node -n 80 --no-pager'" -ForegroundColor White
}
Write-Host ""

# Phase 6: Firewall
Write-Host "# Phase 6: Configure firewall on node1" -ForegroundColor Green
Write-Host "ssh ippan@188.245.97.41 'sudo ufw allow 9000/tcp; sudo ufw allow 9000/udp; sudo ufw allow 8080/tcp; sudo ufw status numbered'" -ForegroundColor White
Write-Host "ssh ippan@188.245.97.41 'sudo systemctl restart ippan-node'" -ForegroundColor White
Write-Host ""

# Phase 7: Validation
Write-Host "# Phase 7: Final validation" -ForegroundColor Green
Write-Host "curl http://178.156.219.107:8080/status" -ForegroundColor White
foreach ($ip in $servers.Keys) {
    Write-Host "ssh ippan@$ip 'sudo systemctl is-active ippan-node && echo OK || (echo FAIL; sudo journalctl -u ippan-node -n 120 --no-pager)'" -ForegroundColor White
}

