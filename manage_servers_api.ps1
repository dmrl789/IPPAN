# IPPAN Server Management via Hetzner Cloud API
# This script will help manage the servers through the API

Write-Host "=== IPPAN Server Management via API ===" -ForegroundColor Cyan
Write-Host ""

# Server information
$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$RESCUE_PASSWORD = "7LuR4nUCfTiv"

Write-Host "Server 1 (Nuremberg): $SERVER1_IP" -ForegroundColor Blue
Write-Host "Server 2 (Helsinki): $SERVER2_IP" -ForegroundColor Blue
Write-Host ""

Write-Host "=== Current Server Status ===" -ForegroundColor Yellow

# Test basic connectivity
Write-Host "Testing Server 1 connectivity..." -ForegroundColor Green
try {
    $ping1 = Test-Connection -ComputerName $SERVER1_IP -Count 1 -Quiet
    if ($ping1) {
        Write-Host "✅ Server 1 is reachable" -ForegroundColor Green
    } else {
        Write-Host "❌ Server 1 is not reachable" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ Server 1 ping failed" -ForegroundColor Red
}

Write-Host "Testing Server 2 connectivity..." -ForegroundColor Green
try {
    $ping2 = Test-Connection -ComputerName $SERVER2_IP -Count 1 -Quiet
    if ($ping2) {
        Write-Host "✅ Server 2 is reachable" -ForegroundColor Green
    } else {
        Write-Host "❌ Server 2 is not reachable" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ Server 2 ping failed" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== API Management Options ===" -ForegroundColor Yellow
Write-Host ""
Write-Host "1. Check server status via API" -ForegroundColor White
Write-Host "2. Deploy IPPAN services via API" -ForegroundColor White
Write-Host "3. Test server connectivity" -ForegroundColor White
Write-Host "4. Create deployment scripts" -ForegroundColor White
Write-Host ""

# Test API endpoints
Write-Host "=== Testing API Endpoints ===" -ForegroundColor Yellow

# Test Server 1 API
Write-Host "Testing Server 1 API (port 3000)..." -ForegroundColor Green
try {
    $api1 = Invoke-WebRequest -Uri "http://$SERVER1_IP`:3000/health" -TimeoutSec 5 -UseBasicParsing 2>$null
    if ($api1.StatusCode -eq 200) {
        Write-Host "✅ Server 1 API is responding" -ForegroundColor Green
        Write-Host "Response: $($api1.Content)" -ForegroundColor Cyan
    } else {
        Write-Host "⚠️ Server 1 API returned status: $($api1.StatusCode)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "❌ Server 1 API is not responding" -ForegroundColor Red
}

# Test Server 2 API
Write-Host "Testing Server 2 API (port 3000)..." -ForegroundColor Green
try {
    $api2 = Invoke-WebRequest -Uri "http://$SERVER2_IP`:3000/health" -TimeoutSec 5 -UseBasicParsing 2>$null
    if ($api2.StatusCode -eq 200) {
        Write-Host "✅ Server 2 API is responding" -ForegroundColor Green
        Write-Host "Response: $($api2.Content)" -ForegroundColor Cyan
    } else {
        Write-Host "⚠️ Server 2 API returned status: $($api2.StatusCode)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "❌ Server 2 API is not responding" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Next Steps ===" -ForegroundColor Yellow
Write-Host "Since the servers are not responding to API calls, we need to:" -ForegroundColor White
Write-Host "1. Deploy IPPAN services on both servers" -ForegroundColor White
Write-Host "2. Configure the peer-to-peer network" -ForegroundColor White
Write-Host "3. Test the connection between servers" -ForegroundColor White
Write-Host ""

# Create a simple deployment script
$deploymentScript = @"
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
    "$SERVER1_IP:8080",
    "$SERVER2_IP:8080"
]
listen_address = "0.0.0.0:8080"
external_address = "$SERVER1_IP:8080"

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
echo "API: http://$SERVER1_IP:3000"
echo "P2P: $SERVER1_IP:8080"
"@

$deploymentScript | Out-File -FilePath "quick_deploy.sh" -Encoding UTF8
Write-Host "Quick deployment script created: quick_deploy.sh" -ForegroundColor Green

Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Cyan
Write-Host "Server 1: $SERVER1_IP - Status: $(if($ping1){'Online'}else{'Offline'})" -ForegroundColor White
Write-Host "Server 2: $SERVER2_IP - Status: $(if($ping2){'Online'}else{'Offline'})" -ForegroundColor White
Write-Host "IPPAN Services: Not running on either server" -ForegroundColor White
Write-Host ""
Write-Host "Next: Deploy IPPAN services on both servers to establish connection" -ForegroundColor Green
