# IPPAN Production Server Deployment Script
# This script deploys IPPAN nodes to the production servers

param(
    [string]$Server1 = "188.245.97.41",
    [string]$Server2 = "135.181.145.174",
    [string]$SshUser = "root"
)

Write-Host "🚀 IPPAN Production Deployment" -ForegroundColor Green
Write-Host "=============================" -ForegroundColor Green
Write-Host "📍 Server 1: $Server1" -ForegroundColor Yellow
Write-Host "📍 Server 2: $Server2" -ForegroundColor Yellow
Write-Host ""

# Function to create deployment script for remote execution
function Create-DeploymentScript {
    param([string]$NodeId, [string]$ApiPort, [string]$P2pPort, [string]$BootstrapPeers)
    
    $script = @"
#!/bin/bash
set -euo pipefail

echo "🔧 Deploying $NodeId on $(hostname)..."

# Update system and install dependencies
apt-get update -y
DEBIAN_FRONTEND=noninteractive apt-get install -y docker.io docker-compose-plugin jq curl ufw

# Start Docker
systemctl start docker
systemctl enable docker

# Create IPPAN directory
mkdir -p /opt/ippan
cd /opt/ippan

# Create docker-compose.yml for production
cat > docker-compose.yml << 'EOF'
version: '3.8'

services:
  ippan-node:
    image: ghcr.io/dmrl789/ippan:latest
    container_name: ippan-$NodeId
    restart: unless-stopped
    user: "0:0"
    command:
      - sh
      - -lc
      - |
        set -e
        echo 'deb http://deb.debian.org/debian bookworm main' > /etc/apt/sources.list.d/bookworm.list
        apt-get update -y
        apt-get install -y --no-install-recommends -t bookworm libssl3 ca-certificates
        exec ippan-node
    environment:
      - NODE_ID=$NodeId
      - RPC_HOST=0.0.0.0
      - RPC_PORT=$ApiPort
      - P2P_HOST=0.0.0.0
      - P2P_PORT=$P2pPort
      - P2P_ANNOUNCE=/ip4/$(curl -s ifconfig.me)/tcp/$P2pPort
      - P2P_BOOTSTRAP=$BootstrapPeers
      - DATA_DIR=/data
      - LOG_LEVEL=info
      - LOG_FORMAT=json
    ports:
      - "0.0.0.0:${P2pPort}:${P2pPort}"
      - "127.0.0.1:${ApiPort}:${ApiPort}"
    volumes:
      - ./data:/data
      - ./logs:/logs
    networks:
      - ippan-network
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:$ApiPort/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

networks:
  ippan-network:
    driver: bridge
EOF

# Configure firewall
ufw allow $P2pPort/tcp comment "IPPAN P2P"
ufw allow $ApiPort/tcp comment "IPPAN API"
ufw --force enable

# Start the node
docker compose up -d

# Wait for node to start
echo "⏳ Waiting for node to initialize..."
sleep 60

# Verify deployment
echo "🏥 Verifying deployment..."
if curl -f http://localhost:$ApiPort/health >/dev/null 2>&1; then
    echo "✅ $NodeId is healthy and running!"
    echo "📊 Node Status:"
    curl -s http://localhost:$ApiPort/health | jq '.'
else
    echo "❌ $NodeId health check failed"
    docker compose logs ippan-node
    exit 1
fi

echo "🎉 $NodeId deployment complete!"
"@
    
    return $script
}

# Create deployment scripts
Write-Host "📝 Creating deployment scripts..." -ForegroundColor Cyan

$server1Script = Create-DeploymentScript -NodeId "node-1" -ApiPort "8080" -P2pPort "9000" -BootstrapPeers "/ip4/$Server2/tcp/9001"
$server2Script = Create-DeploymentScript -NodeId "node-2" -ApiPort "8081" -P2pPort "9001" -BootstrapPeers "/ip4/$Server1/tcp/9000"

# Save scripts to files
$server1Script | Out-File -FilePath "deploy-server1.sh" -Encoding UTF8
$server2Script | Out-File -FilePath "deploy-server2.sh" -Encoding UTF8

Write-Host "✅ Deployment scripts created:" -ForegroundColor Green
Write-Host "   📄 deploy-server1.sh - For Server 1 ($Server1)" -ForegroundColor Yellow
Write-Host "   📄 deploy-server2.sh - For Server 2 ($Server2)" -ForegroundColor Yellow
Write-Host ""

Write-Host "🚀 Next Steps:" -ForegroundColor Cyan
Write-Host "1. Copy deploy-server1.sh to Server 1 and run it" -ForegroundColor White
Write-Host "2. Copy deploy-server2.sh to Server 2 and run it" -ForegroundColor White
Write-Host "3. Verify both nodes are running and connected" -ForegroundColor White
Write-Host ""

Write-Host "📋 Manual Deployment Commands:" -ForegroundColor Magenta
Write-Host "# On Server 1 ($Server1):" -ForegroundColor Yellow
Write-Host "scp deploy-server1.sh root@${Server1}:/root/" -ForegroundColor White
Write-Host "ssh root@${Server1} 'chmod +x /root/deploy-server1.sh && /root/deploy-server1.sh'" -ForegroundColor White
Write-Host ""
Write-Host "# On Server 2 (${Server2}):" -ForegroundColor Yellow
Write-Host "scp deploy-server2.sh root@${Server2}:/root/" -ForegroundColor White
Write-Host "ssh root@${Server2} 'chmod +x /root/deploy-server2.sh && /root/deploy-server2.sh'" -ForegroundColor White
Write-Host ""

Write-Host "🔍 Verification Commands:" -ForegroundColor Cyan
Write-Host "curl http://${Server1}:8080/health" -ForegroundColor White
Write-Host "curl http://${Server2}:8081/health" -ForegroundColor White
Write-Host "curl http://`${Server1}:8080/p2p/peers" -ForegroundColor White
Write-Host "curl http://`${Server2}:8081/p2p/peers" -ForegroundColor White
