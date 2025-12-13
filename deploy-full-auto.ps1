# Full Automated IPPAN Devnet-1 Deployment using plink
# This script uses plink (PuTTY) to automate password-based SSH

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
$pubKey = (Get-Content $pubKeyPath).Trim()

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "IPPAN Devnet-1 Full Automated Deployment" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check for plink
$plink = Get-Command plink -ErrorAction SilentlyContinue
if (-not $plink) {
    Write-Host "ERROR: plink not found. Please install PuTTY." -ForegroundColor Red
    exit 1
}

Write-Host "STEP 1: Creating ippan user on all servers..." -ForegroundColor Yellow

foreach ($ip in $servers.Keys) {
    $server = $servers[$ip]
    Write-Host "`nSetting up $($server.Name) ($ip)..." -ForegroundColor Green
    
    $setupCmd = @"
if ! id -u ippan >/dev/null 2>&1; then
    useradd -m -s /bin/bash ippan
    usermod -aG sudo ippan
    echo 'ippan ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers.d/ippan
    mkdir -p /home/ippan/.ssh
    echo '$pubKey' > /home/ippan/.ssh/authorized_keys
    chmod 700 /home/ippan/.ssh
    chmod 600 /home/ippan/.ssh/authorized_keys
    chown -R ippan:ippan /home/ippan/.ssh
    echo 'User ippan created successfully'
else
    echo 'User ippan exists, updating SSH key'
    mkdir -p /home/ippan/.ssh
    echo '$pubKey' > /home/ippan/.ssh/authorized_keys
    chmod 700 /home/ippan/.ssh
    chmod 600 /home/ippan/.ssh/authorized_keys
    chown -R ippan:ippan /home/ippan/.ssh
    echo 'SSH key updated'
fi
"@
    
    Write-Host "  Creating ippan user..." -ForegroundColor Gray
    $result = & plink -ssh -pw $server.Pass -batch root@$ip $setupCmd 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✓ User created/updated" -ForegroundColor Green
    } else {
        Write-Host "  ✗ Failed: $result" -ForegroundColor Red
    }
}

Write-Host "`nVerifying ippan user setup..." -ForegroundColor Yellow
Start-Sleep -Seconds 2

foreach ($ip in $servers.Keys) {
    $server = $servers[$ip]
    Write-Host "Testing SSH to $($server.Name)..." -ForegroundColor Gray
    $test = ssh -o ConnectTimeout=5 -o BatchMode=yes ippan@$ip "hostname" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✓ $($server.Name): SSH working" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $($server.Name): SSH not working yet" -ForegroundColor Yellow
    }
}

Write-Host "`nSTEP 2: Deploying repository..." -ForegroundColor Yellow

foreach ($ip in $servers.Keys) {
    $server = $servers[$ip]
    Write-Host "`nDeploying to $($server.Name)..." -ForegroundColor Green
    
    Write-Host "  Creating /opt/ippan..." -ForegroundColor Gray
    $mkdirCmd = 'sudo mkdir -p /opt/ippan && sudo chown -R ippan:ippan /opt/ippan'
    $result = ssh ippan@$ip $mkdirCmd 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "    ✓ Directory created" -ForegroundColor Green
    } else {
        Write-Host "    ✗ Failed" -ForegroundColor Red
        continue
    }
    
    Write-Host "  Cloning/pulling repository..." -ForegroundColor Gray
    $gitCmd = 'cd /opt/ippan && (test -d .git && git fetch origin && git reset --hard origin/master || git clone https://github.com/dmrl789/IPPAN .) && git checkout master && git pull --ff-only'
    $result = ssh ippan@$ip $gitCmd 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "    ✓ Repository synced" -ForegroundColor Green
    } else {
        Write-Host "    ✗ Failed: $result" -ForegroundColor Red
    }
    
    Write-Host "  Making setup script executable..." -ForegroundColor Gray
    $result = ssh ippan@$ip "cd /opt/ippan && chmod +x deploy/hetzner/scripts/setup-node.sh || true" 2>&1
    Write-Host "    ✓ Done" -ForegroundColor Green
}

Write-Host "`nSTEP 3: Running setup scripts..." -ForegroundColor Yellow
Write-Host "NOTE: Each setup script takes 15-30 minutes to build the binary" -ForegroundColor Yellow
Write-Host ""

foreach ($ip in $servers.Keys) {
    $server = $servers[$ip]
    Write-Host "Setting up $($server.Name)..." -ForegroundColor Green
    
    if ($server.Bootstrap) {
        $setupCmd = @"
cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh $($server.Name) $($server.Bootstrap)
"@
    } else {
        $setupCmd = @"
cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh $($server.Name)
"@
    }
    
    Write-Host "  Running: $setupCmd" -ForegroundColor Gray
    Write-Host "  This will take 15-30 minutes..." -ForegroundColor Yellow
    
    # Run setup script (this is long-running)
    $result = ssh ippan@$ip $setupCmd 2>&1
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✓ Setup complete for $($server.Name)" -ForegroundColor Green
    } else {
        Write-Host "  ✗ Setup failed for $($server.Name): $result" -ForegroundColor Red
    }
}

Write-Host "`nSTEP 4: Starting services..." -ForegroundColor Yellow

foreach ($ip in $servers.Keys) {
    $server = $servers[$ip]
    Write-Host "Starting service on $($server.Name)..." -ForegroundColor Green
    
    $systemdCmd = @'
sudo systemctl daemon-reload && sudo systemctl enable ippan-node && sudo systemctl restart ippan-node && sleep 2 && sudo systemctl --no-pager --full status ippan-node | head -n 30
'@
    $result = ssh ippan@$ip $systemdCmd 2>&1
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✓ Service started" -ForegroundColor Green
    } else {
        Write-Host "  ✗ Service failed: $result" -ForegroundColor Red
    }
}

Write-Host "`nSTEP 5: Checking logs..." -ForegroundColor Yellow

foreach ($ip in $servers.Keys) {
    $server = $servers[$ip]
    Write-Host "`nLogs from $($server.Name):" -ForegroundColor Green
    ssh ippan@$ip "sudo tail -n 80 /var/log/ippan/node.log || sudo journalctl -u ippan-node -n 80 --no-pager" 2>&1
}

Write-Host "`nSTEP 6: Configuring firewall on node1..." -ForegroundColor Yellow

$ufwCmd = @'
sudo ufw allow 9000/tcp; sudo ufw allow 9000/udp; sudo ufw allow 8080/tcp; sudo ufw status numbered
'@
$result = ssh ippan@188.245.97.41 $ufwCmd 2>&1
Write-Host $result

ssh ippan@188.245.97.41 "sudo systemctl restart ippan-node" 2>&1 | Out-Null

Write-Host "`nSTEP 7: Final validation..." -ForegroundColor Yellow

Write-Host "`nChecking status endpoint..." -ForegroundColor Green
try {
    $status = Invoke-WebRequest -Uri "http://178.156.219.107:8080/status" -TimeoutSec 10 -UseBasicParsing
    Write-Host "✓ Status endpoint responded:" -ForegroundColor Green
    Write-Host $status.Content
} catch {
    Write-Host "✗ Status endpoint failed: $_" -ForegroundColor Red
}

Write-Host "`nChecking service status on all nodes..." -ForegroundColor Green
foreach ($ip in $servers.Keys) {
    $server = $servers[$ip]
    $status = ssh ippan@$ip "sudo systemctl is-active ippan-node" 2>&1
    if ($status -eq "active") {
        Write-Host "  ✓ $($server.Name): active" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $($server.Name): $status" -ForegroundColor Red
        Write-Host "    Recent logs:" -ForegroundColor Yellow
        ssh ippan@$ip "sudo journalctl -u ippan-node -n 120 --no-pager | tail -n 20" 2>&1
    }
}

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "Deployment Complete!" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

