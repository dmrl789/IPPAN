# IPPAN Devnet-1 Hetzner Deployment Script
# Run this script from the repo root directory

$ErrorActionPreference = "Stop"

# Server configuration
$NODE1_PUB = "188.245.97.41"
$NODE1_PRIV = "10.0.0.2"
$NODE2_PUB = "135.181.145.174"
$NODE2_PRIV = "10.0.0.3"
$NODE3_PUB = "5.223.51.238"
$NODE4_PUB = "178.156.219.107"

$SERVERS = @($NODE1_PUB, $NODE2_PUB, $NODE3_PUB, $NODE4_PUB)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "IPPAN Devnet-1 Hetzner Deployment" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Phase 0: Verify local repo state
Write-Host "[Phase 0] Verifying local repository..." -ForegroundColor Yellow
if (-not (Test-Path "deploy\hetzner\scripts\setup-node.sh")) {
    Write-Host "ERROR: setup-node.sh not found!" -ForegroundColor Red
    exit 1
}
if (-not (Test-Path "deploy\hetzner\systemd\ippan-node.service")) {
    Write-Host "ERROR: ippan-node.service not found!" -ForegroundColor Red
    exit 1
}
Write-Host "✓ Local files verified" -ForegroundColor Green
Write-Host ""

# Phase 1: Verify SSH access
Write-Host "[Phase 1] Verifying SSH access to all servers..." -ForegroundColor Yellow
foreach ($server in $SERVERS) {
    Write-Host "Testing SSH to $server..." -ForegroundColor Gray
    $result = ssh -o ConnectTimeout=5 -o StrictHostKeyChecking=accept-new ippan@$server "hostname && uptime" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ $server : $result" -ForegroundColor Green
    } else {
        Write-Host "✗ $server : SSH failed (password may be required)" -ForegroundColor Red
        Write-Host "  Please ensure SSH key authentication is set up, or run commands manually" -ForegroundColor Yellow
    }
}
Write-Host ""

# Phase 2: Copy repo to each server
Write-Host "[Phase 2] Copying repository to each server..." -ForegroundColor Yellow
foreach ($server in $SERVERS) {
    Write-Host "Setting up /opt/ippan on $server..." -ForegroundColor Gray
    ssh ippan@$server "sudo mkdir -p /opt/ippan && sudo chown -R ippan:ippan /opt/ippan"
    if ($LASTEXITCODE -ne 0) {
        Write-Host "✗ Failed to create /opt/ippan on $server" -ForegroundColor Red
        continue
    }
    
    Write-Host "Cloning/pulling repo on $server..." -ForegroundColor Gray
    ssh ippan@$server "cd /opt/ippan && (test -d .git && git fetch origin && git reset --hard origin/master || git clone https://github.com/dmrl789/IPPAN .) && git checkout master && git pull --ff-only"
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ Repository synced on $server" -ForegroundColor Green
    } else {
        Write-Host "✗ Failed to sync repo on $server" -ForegroundColor Red
    }
    
    ssh ippan@$server "cd /opt/ippan && chmod +x deploy/hetzner/scripts/setup-node.sh || true"
}
Write-Host ""

# Phase 3: Run setup script on each node
Write-Host "[Phase 3] Running setup-node.sh on each node..." -ForegroundColor Yellow

Write-Host "Setting up node1 (bootstrap)..." -ForegroundColor Gray
ssh ippan@$NODE1_PUB "cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh node1"
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ node1 setup complete" -ForegroundColor Green
} else {
    Write-Host "✗ node1 setup failed" -ForegroundColor Red
}

Write-Host "Setting up node2 (bootstrap via private IP)..." -ForegroundColor Gray
ssh ippan@$NODE2_PUB "cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh node2 $NODE1_PRIV"
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ node2 setup complete" -ForegroundColor Green
} else {
    Write-Host "✗ node2 setup failed" -ForegroundColor Red
}

Write-Host "Setting up node3 (bootstrap via public IP)..." -ForegroundColor Gray
ssh ippan@$NODE3_PUB "cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh node3 $NODE1_PUB"
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ node3 setup complete" -ForegroundColor Green
} else {
    Write-Host "✗ node3 setup failed" -ForegroundColor Red
}

Write-Host "Setting up node4 (observer/RPC, bootstrap via public IP)..." -ForegroundColor Gray
ssh ippan@$NODE4_PUB "cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh node4 $NODE1_PUB"
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ node4 setup complete" -ForegroundColor Green
} else {
    Write-Host "✗ node4 setup failed" -ForegroundColor Red
}
Write-Host ""

# Phase 4: Start services
Write-Host "[Phase 4] Starting services and verifying ports..." -ForegroundColor Yellow
foreach ($server in $SERVERS) {
    Write-Host "Starting ippan-node on $server..." -ForegroundColor Gray
    ssh ippan@$server "sudo systemctl daemon-reload && sudo systemctl enable ippan-node && sudo systemctl restart ippan-node && sleep 2 && sudo systemctl --no-pager --full status ippan-node | head -n 30"
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ Service started on $server" -ForegroundColor Green
    } else {
        Write-Host "✗ Service failed to start on $server" -ForegroundColor Red
    }
}

Write-Host "Checking ports on node4..." -ForegroundColor Gray
ssh ippan@$NODE4_PUB "ss -lntup | egrep ':(8080|9000)\b' || true"
Write-Host ""

# Phase 5: Check logs
Write-Host "[Phase 5] Checking bootstrap connectivity and logs..." -ForegroundColor Yellow
foreach ($server in $SERVERS) {
    Write-Host "Logs from $server:" -ForegroundColor Gray
    ssh ippan@$server "sudo tail -n 80 /var/log/ippan/node.log 2>/dev/null || sudo journalctl -u ippan-node -n 80 --no-pager"
    Write-Host ""
}

# Phase 6: Check firewall on node1
Write-Host "[Phase 6] Ensuring firewall allows P2P on node1..." -ForegroundColor Yellow
ssh ippan@$NODE1_PUB "sudo ufw allow 9000/tcp; sudo ufw allow 9000/udp; sudo ufw allow 8080/tcp; sudo ufw status numbered"
Write-Host ""

# Phase 7: Final validation
Write-Host "[Phase 7] Final validation..." -ForegroundColor Yellow

Write-Host "Checking service status on all nodes:" -ForegroundColor Gray
foreach ($server in $SERVERS) {
    $status = ssh ippan@$server "sudo systemctl is-active ippan-node 2>&1"
    if ($status -eq "active") {
        Write-Host "✓ $server : active" -ForegroundColor Green
    } else {
        Write-Host "✗ $server : $status" -ForegroundColor Red
        Write-Host "  Recent logs:" -ForegroundColor Yellow
        ssh ippan@$server "sudo journalctl -u ippan-node -n 120 --no-pager | tail -n 20"
    }
}

Write-Host ""
Write-Host "Checking /status endpoint on node4:" -ForegroundColor Gray
try {
    $statusResponse = Invoke-WebRequest -Uri "http://$NODE4_PUB:8080/status" -TimeoutSec 10 -UseBasicParsing
    Write-Host "✓ Status endpoint responded:" -ForegroundColor Green
    Write-Host $statusResponse.Content
} catch {
    Write-Host "✗ Status endpoint failed: $_" -ForegroundColor Red
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Deployment script completed" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "1. Verify all services are active" -ForegroundColor White
Write-Host "2. Check curl http://$NODE4_PUB:8080/status" -ForegroundColor White
Write-Host "3. Monitor logs: ssh ippan@<server> 'sudo journalctl -u ippan-node -f'" -ForegroundColor White
Write-Host "4. Lock down firewall to known IPs (optional)" -ForegroundColor White

