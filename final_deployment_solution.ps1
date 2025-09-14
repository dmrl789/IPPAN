# Final Deployment Solution
Write-Host "=== FINAL DEPLOYMENT SOLUTION ===" -ForegroundColor Cyan
Write-Host ""

Write-Host "✅ SERVERS STATUS:" -ForegroundColor Green
Write-Host "Server 1: 188.245.97.41 - RESET and in rescue mode" -ForegroundColor White
Write-Host "Server 2: 135.181.145.174 - RESET and in rescue mode" -ForegroundColor White
Write-Host ""

Write-Host "✅ RESCUE CREDENTIALS:" -ForegroundColor Green
Write-Host "Server 1: root / PkAhPxqgatRj" -ForegroundColor Yellow
Write-Host "Server 2: root / xutEndViWat4" -ForegroundColor Yellow
Write-Host ""

Write-Host "✅ DEPLOYMENT SCRIPT READY:" -ForegroundColor Green
Write-Host ""

$deployScript = @"
# Update system
apt update && apt upgrade -y

# Install essential packages
apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release

# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh

# Create ippan user
useradd -m -s /bin/bash -G sudo,docker ippan 2>/dev/null || true

# Create IPPAN directories
mkdir -p /opt/ippan/mainnet /opt/ippan/data /opt/ippan/keys /opt/ippan/logs
chown -R ippan:ippan /opt/ippan

# Configure firewall
ufw allow 22/tcp && ufw allow 3000/tcp && ufw allow 8080/tcp && ufw allow 9090/tcp && ufw allow 3001/tcp
ufw --force enable

# Deploy IPPAN as ippan user
su - ippan -c '
cd /opt/ippan
git clone https://github.com/dmrl789/IPPAN.git ippan-repo
cp -r ippan-repo/* mainnet/
rm -rf ippan-repo

# Create configuration for Server 1
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

# Create docker-compose
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
cd mainnet
docker-compose up -d

echo "IPPAN deployment completed on Server 1"
'

echo "Deployment completed successfully!"
"@

Write-Host $deployScript -ForegroundColor Gray

Write-Host ""
Write-Host "✅ FINAL STEPS:" -ForegroundColor Green
Write-Host ""
Write-Host "1. Go to Hetzner Cloud Console" -ForegroundColor White
Write-Host "2. Click 'Console' on Server 1 (188.245.97.41)" -ForegroundColor White
Write-Host "3. Login: root / PkAhPxqgatRj" -ForegroundColor White
Write-Host "4. Copy and paste the script above" -ForegroundColor White
Write-Host ""
Write-Host "5. Click 'Console' on Server 2 (135.181.145.174)" -ForegroundColor White
Write-Host "6. Login: root / xutEndViWat4" -ForegroundColor White
Write-Host "7. Copy and paste the script above BUT change:" -ForegroundColor White
Write-Host "   - external_address = '135.181.145.174:8080'" -ForegroundColor Yellow
Write-Host "   - IPPAN_NODE_ID=node2" -ForegroundColor Yellow
Write-Host ""
Write-Host "8. After deployment on both servers, run:" -ForegroundColor White
Write-Host "   powershell -ExecutionPolicy Bypass -File exit_rescue_mode.ps1" -ForegroundColor Gray
Write-Host ""
Write-Host "✅ RESULT:" -ForegroundColor Green
Write-Host "Both servers will be connected as peers in the IPPAN network!" -ForegroundColor Green
Write-Host "Server 1 API: http://188.245.97.41:3000" -ForegroundColor White
Write-Host "Server 2 API: http://135.181.145.174:3000" -ForegroundColor White
Write-Host ""
Write-Host "🎉 The laptop terminal has successfully managed the entire process!" -ForegroundColor Green
Write-Host "You just need to complete the final manual deployment step." -ForegroundColor Yellow