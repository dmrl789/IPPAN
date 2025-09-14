#!/bin/bash
# IPPAN Quick Deployment Script

# Server 1 (Nuremberg)
SERVER1_IP="188.245.97.41"
SERVER2_IP="135.181.145.174"

echo "=== IPPAN Quick Deployment ==="

# Install Docker if not present
if ! command -v docker &> /dev/null; then
    echo "Installing Docker..."
    curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh
fi

# Create IPPAN directories
mkdir -p /opt/ippan/mainnet
mkdir -p /opt/ippan/data
mkdir -p /opt/ippan/keys
mkdir -p /opt/ippan/logs

# Clone IPPAN repository
cd /opt/ippan
git clone https://github.com/dmrl789/IPPAN.git ippan-repo
cp -r ippan-repo/* mainnet/
rm -rf ippan-repo

# Create basic configuration
cat > mainnet/config.toml << 'EOF'
[network]
bootstrap_nodes = [
    "",
    ""
]
listen_address = "0.0.0.0:8080"
external_address = ""

[api]
listen_address = "0.0.0.0:3000"
cors_origins = ["*"]

[logging]
level = "info"
format = "json"

[consensus]
algorithm = "proof_of_stake"
block_time = 10

[storage]
data_dir = "/opt/ippan/data"
wal_dir = "/opt/ippan/wal"
EOF

# Create simple docker-compose
cat > mainnet/docker-compose.yml << 'EOF'
version: '3.8'
services:
  ippan-node:
    build: .
    container_name: ippan-node
    restart: unless-stopped
    ports:
      - "8080:8080"
      - "3000:3000"
    volumes:
      - ./config.toml:/config/config.toml:ro
      - ippan_data:/data
    environment:
      - RUST_LOG=info
      - IPPAN_NETWORK_PORT=8080
      - IPPAN_API_PORT=3000
    networks:
      - ippan_network

volumes:
  ippan_data:

networks:
  ippan_network:
    driver: bridge
EOF

# Start services
cd mainnet
docker-compose up -d

echo "=== Deployment Complete ==="
echo "API: http://"
echo "P2P: "
