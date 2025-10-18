#!/bin/bash
set -euo pipefail

# IPPAN Node Installation Script
# This script installs and configures IPPAN blockchain nodes

# Configuration
NODE_ID=${1:-"node-1"}
SERVER_IP=${2:-"127.0.0.1"}
API_PORT=${3:-"8080"}
P2P_PORT=${4:-"9000"}
DATA_DIR=${5:-"/opt/ippan"}
DOCKER_IMAGE=${6:-"ghcr.io/dmrl789/ippan:latest"}

echo "🚀 Installing IPPAN Node: $NODE_ID"
echo "📍 Server: $SERVER_IP"
echo "🔌 API Port: $API_PORT"
echo "🌐 P2P Port: $P2P_PORT"
echo "💾 Data Directory: $DATA_DIR"

# Update system packages
echo "📦 Updating system packages..."
if command -v sudo >/dev/null 2>&1 && sudo -n true 2>/dev/null; then
    sudo apt-get update -y
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y \
        docker.io \
        docker-compose-plugin \
        jq \
        curl \
        ufw \
        htop \
        git
elif [ "$(id -u)" -eq 0 ]; then
    apt-get update -y
    DEBIAN_FRONTEND=noninteractive apt-get install -y \
        docker.io \
        docker-compose-plugin \
        jq \
        curl \
        ufw \
        htop \
        git
else
    echo "⚠️ Skipping package updates (no sudo/root access)"
fi

# Start and enable Docker
echo "🐳 Starting Docker service..."
if command -v sudo >/dev/null 2>&1; then
    sudo systemctl start docker
    sudo systemctl enable docker
    sudo usermod -aG docker $USER || true
else
    systemctl start docker
    systemctl enable docker
fi

# Create data directory
echo "📁 Creating data directory..."
mkdir -p "$DATA_DIR"
cd "$DATA_DIR"

# Create docker-compose.yml for the node
echo "📝 Creating Docker Compose configuration..."
cat > docker-compose.yml << EOF
version: '3.8'

services:
  ippan-node:
    image: $DOCKER_IMAGE
    container_name: ippan-$NODE_ID
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
      - NODE_ID=$NODE_ID
      - RPC_HOST=0.0.0.0
      - RPC_PORT=$API_PORT
      - P2P_HOST=0.0.0.0
      - P2P_PORT=$P2P_PORT
      - P2P_ANNOUNCE=/ip4/$SERVER_IP/tcp/$P2P_PORT
      - DATA_DIR=/data
      - LOG_LEVEL=info
      - LOG_FORMAT=json
    ports:
      - "0.0.0.0:$P2P_PORT:$P2P_PORT"
      - "127.0.0.1:$API_PORT:$API_PORT"
    volumes:
      - ./data:/data
      - ./logs:/logs
    networks:
      - ippan-network
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:$API_PORT/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

networks:
  ippan-network:
    driver: bridge
EOF

# Create environment file
echo "⚙️ Creating environment configuration..."
cat > .env << EOF
NODE_ID=$NODE_ID
SERVER_IP=$SERVER_IP
API_PORT=$API_PORT
P2P_PORT=$P2P_PORT
DATA_DIR=$DATA_DIR
DOCKER_IMAGE=$DOCKER_IMAGE
EOF

# Create systemd service for better management
echo "🔧 Creating systemd service..."
sudo tee /etc/systemd/system/ippan-$NODE_ID.service > /dev/null << EOF
[Unit]
Description=IPPAN Blockchain Node $NODE_ID
Requires=docker.service
After=docker.service

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=$DATA_DIR
ExecStart=/usr/bin/docker compose up -d
ExecStop=/usr/bin/docker compose down
TimeoutStartSec=0

[Install]
WantedBy=multi-user.target
EOF

# Configure firewall
echo "🔥 Configuring firewall..."
if command -v ufw >/dev/null 2>&1; then
    sudo ufw allow $P2P_PORT/tcp comment "IPPAN P2P"
    sudo ufw allow $API_PORT/tcp comment "IPPAN API"
    sudo ufw reload || true
fi

# Pull Docker image
echo "📥 Pulling Docker image..."
docker pull "$DOCKER_IMAGE"

# Start the node
echo "🚀 Starting IPPAN node..."
docker compose up -d

# Wait for node to start
echo "⏳ Waiting for node to start..."
sleep 30

# Check node health
echo "🏥 Checking node health..."
if curl -fsSL "http://127.0.0.1:$API_PORT/health" >/dev/null 2>&1; then
    echo "✅ Node $NODE_ID is healthy and running!"
    
    # Show node information
    echo "📊 Node Information:"
    curl -sSL "http://127.0.0.1:$API_PORT/health" | jq '.' || echo "Health check response received"
    
    echo "🌐 P2P Address: /ip4/$SERVER_IP/tcp/$P2P_PORT"
    echo "🔌 API Address: http://$SERVER_IP:$API_PORT"
    
    # Enable systemd service
    sudo systemctl daemon-reload
    sudo systemctl enable ippan-$NODE_ID
    
    echo "✅ IPPAN Node $NODE_ID installation completed successfully!"
else
    echo "❌ Node health check failed"
    echo "📋 Container logs:"
    docker compose logs ippan-node
    exit 1
fi

echo "🎉 Installation completed for node: $NODE_ID"
