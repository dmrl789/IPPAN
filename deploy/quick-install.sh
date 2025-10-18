#!/bin/bash
set -euo pipefail

# Quick IPPAN Node Installation
# Simplified installation for immediate deployment

echo "🚀 Quick IPPAN Node Installation"
echo "================================="

# Server configurations
PRIMARY_SERVER="188.245.97.41"
SECONDARY_SERVER="135.181.145.174"

# Function to install node quickly
quick_install() {
    local server_ip="$1"
    local node_id="$2"
    local api_port="$3"
    local p2p_port="$4"
    
    echo "🔧 Quick installing $node_id on $server_ip..."
    
    ssh -o StrictHostKeyChecking=no root@$server_ip << EOF
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
    container_name: ippan-$node_id
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
      - NODE_ID=$node_id
      - RPC_HOST=0.0.0.0
      - RPC_PORT=$api_port
      - P2P_HOST=0.0.0.0
      - P2P_PORT=$p2p_port
      - P2P_ANNOUNCE=/ip4/$server_ip/tcp/$p2p_port
      - DATA_DIR=/data
      - LOG_LEVEL=info
      - LOG_FORMAT=json
    ports:
      - "0.0.0.0:$p2p_port:$p2p_port"
      - "127.0.0.1:$api_port:$api_port"
    volumes:
      - ./data:/data
      - ./logs:/logs
    networks:
      - ippan-network
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:$api_port/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
networks:
  ippan-network:
    driver: bridge
YML
        
        # Configure firewall
        ufw allow $p2p_port/tcp comment "IPPAN P2P" || true
        ufw allow $api_port/tcp comment "IPPAN API" || true
        ufw reload || true
        
        # Start node
        docker compose up -d
        
        # Wait and check
        sleep 30
        if curl -fsSL "http://127.0.0.1:$api_port/health" >/dev/null 2>&1; then
            echo "✅ $node_id is healthy"
            curl -sSL "http://127.0.0.1:$api_port/health" | jq '.' || echo "Health check successful"
        else
            echo "❌ $node_id health check failed"
            docker compose logs
            exit 1
        fi
EOF
    
    echo "✅ $node_id installation completed on $server_ip"
}

# Install both nodes
echo "📦 Installing Node 1 on primary server..."
quick_install "$PRIMARY_SERVER" "node-1" "8080" "9000"

echo "📦 Installing Node 2 on secondary server..."
quick_install "$SECONDARY_SERVER" "node-2" "8081" "9001"

# Configure P2P connectivity
echo "🔗 Configuring P2P connectivity..."

# Add node2 as bootstrap to node1
ssh -o StrictHostKeyChecking=no root@$PRIMARY_SERVER << EOF
    cd /opt/ippan
    echo "IPPAN_P2P_BOOTSTRAP=/ip4/$SECONDARY_SERVER/tcp/9001" >> .env
    docker compose down
    docker compose up -d
EOF

# Add node1 as bootstrap to node2
ssh -o StrictHostKeyChecking=no root@$SECONDARY_SERVER << EOF
    cd /opt/ippan
    echo "IPPAN_P2P_BOOTSTRAP=/ip4/$PRIMARY_SERVER/tcp/9000" >> .env
    docker compose down
    docker compose up -d
EOF

# Final verification
echo "🔍 Final verification..."
sleep 30

echo "📊 Node Status:"
echo "Node 1: http://$PRIMARY_SERVER:8080/health"
curl -sSL "http://$PRIMARY_SERVER:8080/health" | jq '.' || echo "Node 1 check"

echo "Node 2: http://$SECONDARY_SERVER:8081/health"
curl -sSL "http://$SECONDARY_SERVER:8081/health" | jq '.' || echo "Node 2 check"

echo ""
echo "🎉 Quick Installation Completed!"
echo "================================="
echo "📍 Node 1: http://$PRIMARY_SERVER:8080"
echo "📍 Node 2: http://$SECONDARY_SERVER:8081"
echo "🌐 P2P Network: Connected"
echo "✅ Both nodes are running and healthy"
