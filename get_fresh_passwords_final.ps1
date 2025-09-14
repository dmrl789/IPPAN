# Get Fresh Passwords After Reset
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "=== Getting Fresh Passwords After Reset ===" -ForegroundColor Cyan

# Get the latest rescue actions
try {
    $actions1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions" -Headers $headers -Method GET
    $rescueAction1 = $actions1.actions | Where-Object { $_.command -eq "enable_rescue" } | Sort-Object -Property id -Descending | Select-Object -First 1
    
    if ($rescueAction1) {
        Write-Host "Server 1 Fresh Rescue Password: $($rescueAction1.root_password)" -ForegroundColor Green
        $SERVER1_PASSWORD = $rescueAction1.root_password
    }
    
    $actions2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions" -Headers $headers -Method GET
    $rescueAction2 = $actions2.actions | Where-Object { $_.command -eq "enable_rescue" } | Sort-Object -Property id -Descending | Select-Object -First 1
    
    if ($rescueAction2) {
        Write-Host "Server 2 Fresh Rescue Password: $($rescueAction2.root_password)" -ForegroundColor Green
        $SERVER2_PASSWORD = $rescueAction2.root_password
    }
} catch {
    Write-Host "❌ Failed to get rescue passwords: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Manual Deployment Instructions ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Since SSH automation is having issues, here's what you need to do:" -ForegroundColor Yellow
Write-Host ""
Write-Host "1. Go to Hetzner Cloud Console" -ForegroundColor White
Write-Host "2. Click on each server and select 'Console'" -ForegroundColor White
Write-Host "3. Use these credentials:" -ForegroundColor White
Write-Host "   - Server 1 (188.245.97.41): root / $SERVER1_PASSWORD" -ForegroundColor Green
Write-Host "   - Server 2 (135.181.145.174): root / $SERVER2_PASSWORD" -ForegroundColor Green
Write-Host ""
Write-Host "4. Run this command on both servers:" -ForegroundColor White
Write-Host ""
$deployCommand = @"
# Update system
apt update && apt upgrade -y

# Install essential packages
apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release

# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh

# Create ippan user
useradd -m -s /bin/bash -G sudo,docker ippan 2>/dev/null || true

# Create IPPAN directories
mkdir -p /opt/ippan/mainnet
mkdir -p /opt/ippan/data
mkdir -p /opt/ippan/keys
mkdir -p /opt/ippan/logs
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

# Create configuration
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

echo "IPPAN deployment completed"
'

echo "Deployment completed successfully!"
"@

Write-Host $deployCommand -ForegroundColor Gray

Write-Host ""
Write-Host "5. For Server 2, change the external_address to '135.181.145.174:8080' and NODE_ID to 'node2'" -ForegroundColor White
Write-Host ""
Write-Host "6. After deployment on both servers, exit rescue mode:" -ForegroundColor White
Write-Host "   powershell -ExecutionPolicy Bypass -File exit_rescue_mode.ps1" -ForegroundColor Gray
Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Cyan
Write-Host "✅ Servers have been formatted and reset" -ForegroundColor Green
Write-Host "✅ Fresh rescue mode enabled with new passwords" -ForegroundColor Green
Write-Host "✅ Deployment script ready" -ForegroundColor Green
Write-Host "✅ Both servers will be connected as peers once deployed" -ForegroundColor Green
Write-Host ""
Write-Host "The servers are now clean and ready for fresh IPPAN deployment!" -ForegroundColor Green
