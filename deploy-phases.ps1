# IPPAN Devnet-1 Deployment - Execute phases manually
# Copy and run each phase section, entering passwords when prompted

$NODE1_PUB = "188.245.97.41"
$NODE1_PRIV = "10.0.0.2"
$NODE2_PUB = "135.181.145.174"
$NODE2_PRIV = "10.0.0.3"
$NODE3_PUB = "5.223.51.238"
$NODE4_PUB = "178.156.219.107"

$SERVERS = @($NODE1_PUB, $NODE2_PUB, $NODE3_PUB, $NODE4_PUB)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "IPPAN Devnet-1 Deployment" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Server variables set. Run each phase below." -ForegroundColor Yellow
Write-Host ""

# PHASE 2: Copy repo to each server
Write-Host "PHASE 2: Copying repository to servers..." -ForegroundColor Green
Write-Host "Run these commands:" -ForegroundColor Yellow

Write-Host "`n# Setup /opt/ippan on all servers" -ForegroundColor Cyan
foreach ($server in $SERVERS) {
    Write-Host "ssh ippan@$server 'sudo mkdir -p /opt/ippan && sudo chown -R ippan:ippan /opt/ippan'" -ForegroundColor White
}

Write-Host "`n# Clone/pull repo on all servers" -ForegroundColor Cyan
foreach ($server in $SERVERS) {
    Write-Host "ssh ippan@$server 'cd /opt/ippan && (test -d .git && git fetch origin && git reset --hard origin/master || git clone https://github.com/dmrl789/IPPAN .) && git checkout master && git pull --ff-only'" -ForegroundColor White
}

Write-Host "`n# Make setup script executable" -ForegroundColor Cyan
foreach ($server in $SERVERS) {
    Write-Host "ssh ippan@$server 'cd /opt/ippan && chmod +x deploy/hetzner/scripts/setup-node.sh || true'" -ForegroundColor White
}

Write-Host "`n`nPHASE 3: Running setup scripts..." -ForegroundColor Green
Write-Host "# node1 (bootstrap)" -ForegroundColor Cyan
Write-Host "ssh ippan@$NODE1_PUB 'cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh node1'" -ForegroundColor White

Write-Host "`n# node2 (bootstrap via private IP)" -ForegroundColor Cyan
Write-Host "ssh ippan@$NODE2_PUB 'cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh node2 $NODE1_PRIV'" -ForegroundColor White

Write-Host "`n# node3 (bootstrap via public IP)" -ForegroundColor Cyan
Write-Host "ssh ippan@$NODE3_PUB 'cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh node3 $NODE1_PUB'" -ForegroundColor White

Write-Host "`n# node4 (observer/RPC, bootstrap via public IP)" -ForegroundColor Cyan
Write-Host "ssh ippan@$NODE4_PUB 'cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh node4 $NODE1_PUB'" -ForegroundColor White

Write-Host "`n`nPHASE 4: Starting services..." -ForegroundColor Green
Write-Host "# Start services on all nodes" -ForegroundColor Cyan
foreach ($server in $SERVERS) {
    Write-Host "ssh ippan@$server 'sudo systemctl daemon-reload && sudo systemctl enable ippan-node && sudo systemctl restart ippan-node && sleep 2 && sudo systemctl --no-pager --full status ippan-node | head -n 30'" -ForegroundColor White
}

Write-Host "`n# Check ports on node4" -ForegroundColor Cyan
Write-Host "ssh ippan@$NODE4_PUB 'ss -lntup | egrep `":(8080|9000)\b`" || true'" -ForegroundColor White

Write-Host "`n`nPHASE 5: Checking logs..." -ForegroundColor Green
foreach ($server in $SERVERS) {
    Write-Host "ssh ippan@$server 'sudo tail -n 80 /var/log/ippan/node.log || sudo journalctl -u ippan-node -n 80 --no-pager'" -ForegroundColor White
}

Write-Host "`n`nPHASE 6: Fixing firewall on node1..." -ForegroundColor Green
Write-Host "ssh ippan@$NODE1_PUB 'sudo ufw allow 9000/tcp; sudo ufw allow 9000/udp; sudo ufw allow 8080/tcp; sudo ufw status numbered'" -ForegroundColor White
Write-Host "ssh ippan@$NODE1_PUB 'sudo systemctl restart ippan-node'" -ForegroundColor White

Write-Host "`n`nPHASE 7: Final validation..." -ForegroundColor Green
Write-Host "# Check status endpoint" -ForegroundColor Cyan
Write-Host "curl http://178.156.219.107:8080/status" -ForegroundColor White

Write-Host "`n# Verify all services are active" -ForegroundColor Cyan
foreach ($server in $SERVERS) {
    Write-Host "ssh ippan@$server 'sudo systemctl is-active ippan-node && echo OK || (echo FAIL; sudo journalctl -u ippan-node -n 120 --no-pager)'" -ForegroundColor White
}

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "Copy and run the commands above" -ForegroundColor Yellow
Write-Host "========================================" -ForegroundColor Cyan

