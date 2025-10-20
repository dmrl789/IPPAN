#!/bin/bash
set -euo pipefail

echo "🔧 Deploying Node 1 on Server 1 (188.245.97.41)..."

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
    container_name: ippan-node-1
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
      - NODE_ID=node-1
      - RPC_HOST=0.0.0.0
      - RPC_PORT=8080
      - P2P_HOST=0.0.0.0
      - P2P_PORT=9000
      - P2P_ANNOUNCE=/ip4/188.245.97.41/tcp/9000
      - P2P_BOOTSTRAP=/ip4/135.181.145.174/tcp/9001
      - DATA_DIR=/data
      - LOG_LEVEL=info
      - LOG_FORMAT=json
    ports:
      - "0.0.0.0:9000:9000"
      - "0.0.0.0:8080:8080"
    volumes:
      - ./data:/data
      - ./logs:/logs
    networks:
      - ippan-network
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

networks:
  ippan-network:
    driver: bridge
EOF

# Configure firewall
ufw allow 9000/tcp comment "IPPAN P2P"
ufw allow 8080/tcp comment "IPPAN API"
ufw --force enable

# Start the node
docker compose up -d

# Wait for node to start
echo "⏳ Waiting for node to initialize..."
sleep 60

# Verify deployment
echo "🏥 Verifying deployment..."
if curl -f http://localhost:8080/health >/dev/null 2>&1; then
    echo "✅ Node 1 is healthy and running!"
    echo "📊 Node Status:"
    curl -s http://localhost:8080/health | jq '.'
else
    echo "❌ Node 1 health check failed"
    docker compose logs ippan-node
    exit 1
fi

echo "🎉 Node 1 deployment complete!"
