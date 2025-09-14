# Final IPPAN Server Connection Solution
# This script provides multiple options to connect both servers

$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$RESCUE_PASSWORD = "7LuR4nUCfTiv"

Write-Host "=== IPPAN Server Connection Solution ===" -ForegroundColor Cyan
Write-Host ""

# Current status check
Write-Host "=== Current Server Status ===" -ForegroundColor Yellow

# Test Server 1
Write-Host "Testing Server 1 ($SERVER1_IP)..." -ForegroundColor Green
$server1Online = Test-Connection -ComputerName $SERVER1_IP -Count 1 -Quiet
if ($server1Online) {
    Write-Host "✅ Server 1 is online" -ForegroundColor Green
} else {
    Write-Host "❌ Server 1 is offline" -ForegroundColor Red
}

# Test Server 2
Write-Host "Testing Server 2 ($SERVER2_IP)..." -ForegroundColor Green
$server2Online = Test-Connection -ComputerName $SERVER2_IP -Count 1 -Quiet
if ($server2Online) {
    Write-Host "✅ Server 2 is online" -ForegroundColor Green
} else {
    Write-Host "❌ Server 2 is offline" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Deployment Status ===" -ForegroundColor Yellow

# Test APIs
$server1API = $false
$server2API = $false

try {
    $response1 = Invoke-WebRequest -Uri "http://$SERVER1_IP`:3000/health" -TimeoutSec 5 -UseBasicParsing 2>$null
    if ($response1.StatusCode -eq 200) {
        $server1API = $true
        Write-Host "✅ Server 1 IPPAN API is running" -ForegroundColor Green
    }
} catch {
    Write-Host "❌ Server 1 IPPAN API is not running" -ForegroundColor Red
}

try {
    $response2 = Invoke-WebRequest -Uri "http://$SERVER2_IP`:3000/health" -TimeoutSec 5 -UseBasicParsing 2>$null
    if ($response2.StatusCode -eq 200) {
        $server2API = $true
        Write-Host "✅ Server 2 IPPAN API is running" -ForegroundColor Green
    }
} catch {
    Write-Host "❌ Server 2 IPPAN API is not running" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Solution Options ===" -ForegroundColor Cyan

if ($server1API -and $server2API) {
    Write-Host "🎉 Both servers are running IPPAN services!" -ForegroundColor Green
    Write-Host "Testing peer-to-peer connection..." -ForegroundColor Yellow
    
    # Test peer connection
    try {
        $peers1 = Invoke-WebRequest -Uri "http://$SERVER1_IP`:3000/api/v1/network/peers" -TimeoutSec 10 -UseBasicParsing 2>$null
        $peers2 = Invoke-WebRequest -Uri "http://$SERVER2_IP`:3000/api/v1/network/peers" -TimeoutSec 10 -UseBasicParsing 2>$null
        
        if ($peers1.StatusCode -eq 200 -and $peers2.StatusCode -eq 200) {
            Write-Host "✅ Both servers can access peer API" -ForegroundColor Green
            Write-Host "Server 1 peers: $($peers1.Content)" -ForegroundColor Cyan
            Write-Host "Server 2 peers: $($peers2.Content)" -ForegroundColor Cyan
        }
    } catch {
        Write-Host "⚠️ Could not test peer connections" -ForegroundColor Yellow
    }
    
} else {
    Write-Host "⚠️ IPPAN services need to be deployed" -ForegroundColor Yellow
    Write-Host ""
    
    if (-not $server1API) {
        Write-Host "=== Server 1 Deployment Options ===" -ForegroundColor Blue
        Write-Host "Server 1 was in rescue mode and we attempted deployment." -ForegroundColor White
        Write-Host "If it's not working, try these options:" -ForegroundColor White
        Write-Host ""
        Write-Host "Option 1: Check if Server 1 is still in rescue mode" -ForegroundColor Yellow
        Write-Host "  - Go to Hetzner Cloud Console" -ForegroundColor White
        Write-Host "  - Check if Server 1 needs to exit rescue mode" -ForegroundColor White
        Write-Host ""
        Write-Host "Option 2: Reconnect to Server 1" -ForegroundColor Yellow
        Write-Host "  - SSH: ssh root@$SERVER1_IP" -ForegroundColor White
        Write-Host "  - Password: $RESCUE_PASSWORD" -ForegroundColor White
        Write-Host "  - Run: docker ps" to check if services are running" -ForegroundColor White
        Write-Host ""
    }
    
    if (-not $server2API) {
        Write-Host "=== Server 2 Deployment Options ===" -ForegroundColor Blue
        Write-Host "Server 2 requires SSH key authentication." -ForegroundColor White
        Write-Host "Options to deploy on Server 2:" -ForegroundColor White
        Write-Host ""
        Write-Host "Option 1: Use Hetzner Cloud Console (Recommended)" -ForegroundColor Yellow
        Write-Host "  1. Go to https://console.hetzner.cloud/" -ForegroundColor White
        Write-Host "  2. Find Server 2 ($SERVER2_IP)" -ForegroundColor White
        Write-Host "  3. Click 'Console' tab" -ForegroundColor White
        Write-Host "  4. Login and run deployment commands" -ForegroundColor White
        Write-Host ""
        Write-Host "Option 2: Put Server 2 in Rescue Mode" -ForegroundColor Yellow
        Write-Host "  1. In Hetzner Console, select Server 2" -ForegroundColor White
        Write-Host "  2. Click 'Actions' -> 'Enable Rescue Mode'" -ForegroundColor White
        Write-Host "  3. Use rescue credentials to connect" -ForegroundColor White
        Write-Host "  4. Deploy IPPAN services" -ForegroundColor White
        Write-Host "  5. Exit rescue mode" -ForegroundColor White
        Write-Host ""
        Write-Host "Option 3: Add SSH Key to Server 2" -ForegroundColor Yellow
        Write-Host "  1. Get your SSH public key:" -ForegroundColor White
        Write-Host "     cat $env:USERPROFILE\.ssh\id_rsa.pub" -ForegroundColor Cyan
        Write-Host "  2. Add it to Server 2 via Hetzner Console" -ForegroundColor White
        Write-Host "  3. Then we can deploy via SSH" -ForegroundColor White
    }
}

Write-Host ""
Write-Host "=== Quick Deployment Commands ===" -ForegroundColor Cyan
Write-Host "If you can access either server, run these commands:" -ForegroundColor White
Write-Host ""

$quickDeploy = @"
# Quick IPPAN Deployment Commands

# 1. Update system and install Docker
apt update && apt upgrade -y
apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release
curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh

# 2. Create ippan user
useradd -m -s /bin/bash -G sudo,docker ippan

# 3. Create directories
mkdir -p /opt/ippan/mainnet
mkdir -p /opt/ippan/data
mkdir -p /opt/ippan/keys
mkdir -p /opt/ippan/logs
chown -R ippan:ippan /opt/ippan

# 4. Configure firewall
ufw allow 22/tcp    # SSH
ufw allow 3000/tcp  # API
ufw allow 8080/tcp  # P2P
ufw allow 9090/tcp  # Prometheus
ufw allow 3001/tcp  # Grafana
ufw --force enable

# 5. Deploy IPPAN (run as ippan user)
su - ippan -c '
cd /opt/ippan
git clone https://github.com/dmrl789/IPPAN.git ippan-repo
cp -r ippan-repo/* mainnet/
rm -rf ippan-repo

# Create configuration
cat > mainnet/config.toml << "EOF"
[network]
bootstrap_nodes = [
    "$SERVER1_IP:8080",
    "$SERVER2_IP:8080"
]
listen_address = "0.0.0.0:8080"
external_address = "SERVER_IP:8080"

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
      - IPPAN_BOOTSTRAP_NODES=$SERVER1_IP:8080,$SERVER2_IP:8080
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

echo "IPPAN deployment completed!"
'

# 6. Check status
docker ps --filter "name=ippan"
"@

Write-Host $quickDeploy -ForegroundColor Cyan

# Save to file
$quickDeploy | Out-File -FilePath "quick_deployment_commands.txt" -Encoding UTF8
Write-Host ""
Write-Host "Commands saved to: quick_deployment_commands.txt" -ForegroundColor Green

Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Cyan
Write-Host "Server 1: $(if($server1Online){'Online'}else{'Offline'}) - IPPAN: $(if($server1API){'Running'}else{'Not Running'})" -ForegroundColor White
Write-Host "Server 2: $(if($server2Online){'Online'}else{'Offline'}) - IPPAN: $(if($server2API){'Running'}else{'Not Running'})" -ForegroundColor White
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "1. Deploy IPPAN on both servers using the options above" -ForegroundColor White
Write-Host "2. Once both APIs are running, they will automatically connect" -ForegroundColor White
Write-Host "3. Test the connection using the access URLs below" -ForegroundColor White
Write-Host ""
Write-Host "Access URLs:" -ForegroundColor Cyan
Write-Host "Server 1 API: http://$SERVER1_IP`:3000" -ForegroundColor White
Write-Host "Server 1 Grafana: http://$SERVER1_IP`:3001" -ForegroundColor White
Write-Host "Server 1 Prometheus: http://$SERVER1_IP`:9090" -ForegroundColor White
Write-Host "Server 2 API: http://$SERVER2_IP`:3000" -ForegroundColor White
Write-Host "Server 2 Grafana: http://$SERVER2_IP`:3001" -ForegroundColor White
Write-Host "Server 2 Prometheus: http://$SERVER2_IP`:9090" -ForegroundColor White
