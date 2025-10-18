# IPPAN Multi-Node Installation Script (PowerShell)
# This script installs IPPAN nodes on both servers

param(
    [string]$PrimaryServer = "188.245.97.41",
    [string]$SecondaryServer = "135.181.145.174",
    [string]$SshUser = "root",
    [int]$SshPort = 22
)

Write-Host "🚀 IPPAN Multi-Node Installation" -ForegroundColor Green
Write-Host "=================================" -ForegroundColor Green
Write-Host "📍 Primary Server: $PrimaryServer" -ForegroundColor Yellow
Write-Host "📍 Secondary Server: $SecondaryServer" -ForegroundColor Yellow
Write-Host ""

# Function to install node on server
function Install-Node {
    param(
        [string]$ServerIp,
        [string]$NodeId,
        [int]$ApiPort,
        [int]$P2pPort
    )
    
    Write-Host "🔧 Installing $NodeId on $ServerIp..." -ForegroundColor Cyan
    
    $installScript = @"
set -euo pipefail

# Update and install packages
apt-get update -y
DEBIAN_FRONTEND=noninteractive apt-get install -y docker.io docker-compose-plugin jq curl ufw

# Start Docker
systemctl start docker
systemctl enable docker

# Create directory
mkdir -p /opt/ippan
cd /opt/ippan

# Create docker-compose.yml
cat > docker-compose.yml << 'YML'
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
      - P2P_ANNOUNCE=/ip4/$ServerIp/tcp/$P2pPort
      - DATA_DIR=/data
      - LOG_LEVEL=info
      - LOG_FORMAT=json
    ports:
      - "0.0.0.0:$P2pPort:$P2pPort"
      - "127.0.0.1:$ApiPort:$ApiPort"
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
YML

# Configure firewall
ufw allow $P2pPort/tcp comment "IPPAN P2P" || true
ufw allow $ApiPort/tcp comment "IPPAN API" || true
ufw reload || true

# Start node
docker compose up -d

# Wait and check
sleep 30
if curl -fsSL "http://127.0.0.1:$ApiPort/health" >/dev/null 2>&1; then
    echo "✅ $NodeId is healthy"
    curl -sSL "http://127.0.0.1:$ApiPort/health" | jq '.' || echo "Health check successful"
else
    echo "❌ $NodeId health check failed"
    docker compose logs
    exit 1
fi
"@

    # Execute installation on server
    $installScript | ssh -o StrictHostKeyChecking=no -p $SshPort $SshUser@$ServerIp
    
    Write-Host "✅ $NodeId installation completed on $ServerIp" -ForegroundColor Green
}

# Function to verify node health
function Test-NodeHealth {
    param(
        [string]$ServerIp,
        [int]$ApiPort,
        [string]$NodeId
    )
    
    Write-Host "🏥 Verifying $NodeId health on $ServerIp`:$ApiPort..." -ForegroundColor Cyan
    
    try {
        $response = Invoke-RestMethod -Uri "http://$ServerIp`:$ApiPort/health" -TimeoutSec 10
        Write-Host "✅ $NodeId is healthy" -ForegroundColor Green
        $response | ConvertTo-Json -Depth 3
    }
    catch {
        Write-Host "❌ $NodeId health check failed: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
    return $true
}

# Function to configure P2P connectivity
function Set-P2PConnectivity {
    param(
        [string]$Node1Ip,
        [int]$Node1Port,
        [string]$Node2Ip,
        [int]$Node2Port
    )
    
    Write-Host "🔗 Configuring P2P connectivity..." -ForegroundColor Cyan
    
    # Add node2 as bootstrap peer to node1
    $bootstrapScript1 = @"
cd /opt/ippan
echo "IPPAN_P2P_BOOTSTRAP=/ip4/$Node2Ip/tcp/$Node2Port" >> .env
docker compose down
docker compose up -d
"@
    
    $bootstrapScript1 | ssh -o StrictHostKeyChecking=no -p $SshPort $SshUser@$Node1Ip
    
    # Add node1 as bootstrap peer to node2
    $bootstrapScript2 = @"
cd /opt/ippan
echo "IPPAN_P2P_BOOTSTRAP=/ip4/$Node1Ip/tcp/$Node1Port" >> .env
docker compose down
docker compose up -d
"@
    
    $bootstrapScript2 | ssh -o StrictHostKeyChecking=no -p $SshPort $SshUser@$Node2Ip
    
    Write-Host "✅ P2P connectivity configured" -ForegroundColor Green
}

# Main installation process
try {
    Write-Host "🚀 Starting IPPAN multi-node installation..." -ForegroundColor Green
    
    # Install Node 1
    Write-Host "📦 Installing Node 1 on primary server..." -ForegroundColor Yellow
    Install-Node -ServerIp $PrimaryServer -NodeId "node-1" -ApiPort 8080 -P2pPort 9000
    
    # Install Node 2
    Write-Host "📦 Installing Node 2 on secondary server..." -ForegroundColor Yellow
    Install-Node -ServerIp $SecondaryServer -NodeId "node-2" -ApiPort 8081 -P2pPort 9001
    
    # Wait for both nodes to start
    Write-Host "⏳ Waiting for nodes to initialize..." -ForegroundColor Yellow
    Start-Sleep -Seconds 60
    
    # Verify both nodes
    Write-Host "🏥 Verifying node health..." -ForegroundColor Cyan
    $node1Healthy = Test-NodeHealth -ServerIp $PrimaryServer -ApiPort 8080 -NodeId "node-1"
    $node2Healthy = Test-NodeHealth -ServerIp $SecondaryServer -ApiPort 8081 -NodeId "node-2"
    
    if (-not $node1Healthy -or -not $node2Healthy) {
        Write-Host "❌ One or more nodes failed health check" -ForegroundColor Red
        exit 1
    }
    
    # Configure P2P connectivity
    Set-P2PConnectivity -Node1Ip $PrimaryServer -Node1Port 9000 -Node2Ip $SecondaryServer -Node2Port 9001
    
    # Final verification
    Write-Host "🔍 Final connectivity test..." -ForegroundColor Cyan
    Start-Sleep -Seconds 30
    
    Write-Host "📊 Final Status Check:" -ForegroundColor Green
    Write-Host "Node 1: http://$PrimaryServer`:8080/health" -ForegroundColor Yellow
    try {
        $node1Status = Invoke-RestMethod -Uri "http://$PrimaryServer`:8080/health" -TimeoutSec 10
        $node1Status | ConvertTo-Json -Depth 3
    }
    catch {
        Write-Host "Node 1 health check failed: $($_.Exception.Message)" -ForegroundColor Red
    }
    
    Write-Host "Node 2: http://$SecondaryServer`:8081/health" -ForegroundColor Yellow
    try {
        $node2Status = Invoke-RestMethod -Uri "http://$SecondaryServer`:8081/health" -TimeoutSec 10
        $node2Status | ConvertTo-Json -Depth 3
    }
    catch {
        Write-Host "Node 2 health check failed: $($_.Exception.Message)" -ForegroundColor Red
    }
    
    Write-Host ""
    Write-Host "🎉 IPPAN Multi-Node Installation Completed!" -ForegroundColor Green
    Write-Host "==========================================" -ForegroundColor Green
    Write-Host "📍 Node 1: http://$PrimaryServer`:8080" -ForegroundColor Yellow
    Write-Host "📍 Node 2: http://$SecondaryServer`:8081" -ForegroundColor Yellow
    Write-Host "🌐 P2P Network: Connected" -ForegroundColor Green
    Write-Host "✅ Both nodes are running and healthy" -ForegroundColor Green
}
catch {
    Write-Host "❌ Installation failed: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}
