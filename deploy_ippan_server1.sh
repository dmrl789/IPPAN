#!/bin/bash

# IPPAN Server 1 Deployment Script
# Run this on the server at 188.245.97.41

set -e

echo "=== IPPAN Server 1 Deployment ==="
echo "Server IP: 188.245.97.41"
echo "Starting deployment..."

# Update system
echo "Updating system packages..."
apt update && apt upgrade -y

# Install essential packages
echo "Installing essential packages..."
apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release

# Install Docker
echo "Installing Docker..."
curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh

# Install Docker Compose
echo "Installing Docker Compose..."
curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
chmod +x /usr/local/bin/docker-compose

# Create ippan user
echo "Creating ippan user..."
useradd -m -s /bin/bash -G sudo,docker ippan 2>/dev/null || true

# Create IPPAN directories
echo "Creating IPPAN directories..."
mkdir -p /opt/ippan/mainnet/{data,keys,logs,monitor,grafana,alertmanager,postgres,redis,nginx/logs,fluentd,backups,ssl}
chown -R ippan:ippan /opt/ippan
chmod -R 755 /opt/ippan

# Configure firewall
echo "Configuring firewall..."
ufw --force reset
ufw default deny incoming
ufw default allow outgoing
ufw allow 22/tcp    # SSH
ufw allow 80/tcp    # HTTP
ufw allow 443/tcp   # HTTPS
ufw allow 3000/tcp  # API
ufw allow 8080/tcp  # P2P
ufw allow 9090/tcp  # Prometheus
ufw allow 3001/tcp  # Grafana
ufw --force enable

# Deploy IPPAN as ippan user
echo "Deploying IPPAN blockchain system..."
su - ippan -c '
cd /opt/ippan

# Clone IPPAN repository
echo "Cloning IPPAN repository..."
git clone https://github.com/dmrl789/IPPAN.git ippan-repo
cp -r ippan-repo/* mainnet/
rm -rf ippan-repo

# Create configuration for Server 1
echo "Creating configuration..."
cat > mainnet/config.toml << "EOF"
[network]
bootstrap_nodes = [
    "188.245.97.41:8080",
    "135.181.145.174:8080"
]
listen_address = "0.0.0.0:8080"
external_address = "188.245.97.41:8080"

[api]
listen_address = "0.0.0.0:3000"
cors_origins = ["*"]

[metrics]
listen_address = "0.0.0.0:9090"

[logging]
level = "info"
format = "json"

[consensus]
algorithm = "proof_of_stake"
block_time = 10
max_transactions_per_block = 1000

[storage]
data_dir = "/opt/ippan/data"
wal_dir = "/opt/ippan/wal"
EOF

# Create docker-compose for Server 1
echo "Creating docker-compose configuration..."
cat > mainnet/docker-compose.yml << "EOF"
version: "3.8"
services:
  ippan-node:
    build: .
    container_name: ippan-node
    restart: unless-stopped
    ports:
      - "8080:8080"
      - "3000:3000"
      - "80:80"
      - "443:443"
    volumes:
      - ./config.toml:/config/config.toml:ro
      - ippan_data:/data
      - ippan_keys:/keys
      - ippan_logs:/logs
    environment:
      - RUST_LOG=info
      - IPPAN_NETWORK_PORT=8080
      - IPPAN_API_PORT=3000
      - IPPAN_STORAGE_DIR=/data
      - IPPAN_KEYS_DIR=/keys
      - IPPAN_LOG_DIR=/logs
      - NODE_ENV=production
      - RUST_BACKTRACE=1
      - IPPAN_NODE_ID=node1
      - IPPAN_BOOTSTRAP_NODES=188.245.97.41:8080,135.181.145.174:8080
    networks:
      - ippan_network

volumes:
  ippan_data:
  ippan_keys:
  ippan_logs:

networks:
  ippan_network:
    driver: bridge
EOF

# Start services
echo "Starting IPPAN services..."
cd mainnet
docker-compose up -d

echo "IPPAN deployment completed on Server 1"
'

# Configure SSH security
echo "Configuring SSH security..."
sed -i 's/#PermitRootLogin yes/PermitRootLogin no/' /etc/ssh/sshd_config
sed -i 's/#PasswordAuthentication yes/PasswordAuthentication no/' /etc/ssh/sshd_config
systemctl restart ssh

# Start and enable services
echo "Starting services..."
systemctl restart fail2ban
systemctl enable fail2ban
systemctl restart docker
systemctl enable docker

# Create setup completion marker
echo "Creating setup completion marker..."
echo "IPPAN server #1 setup completed at $(date)" > /opt/ippan/mainnet/logs/setup-complete.log
chown ippan:ippan /opt/ippan/mainnet/logs/setup-complete.log

echo ""
echo "=== Deployment Complete ==="
echo "✅ IPPAN Server 1 deployment completed successfully!"
echo ""
echo "Server Details:"
echo "- Hostname: $(hostname)"
echo "- IP: 188.245.97.41"
echo "- User: ippan (with sudo access)"
echo "- Docker: Installed and configured"
echo "- Firewall: UFW configured with IPPAN rules"
echo "- Security: Fail2ban enabled, SSH hardened"
echo ""
echo "Access URLs:"
echo "- API: http://188.245.97.41:3000"
echo "- P2P: 188.245.97.41:8080"
echo "- Metrics: http://188.245.97.41:9090"
echo ""
echo "Next Steps:"
echo "1. Deploy Server 2 (135.181.145.174)"
echo "2. Test API endpoints"
echo "3. Monitor blockchain network"
echo ""
echo "Setup log: /opt/ippan/mainnet/logs/setup-complete.log"
