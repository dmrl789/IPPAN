# Simple Server 2 Deployment Script
$SERVER1_IP = "188.245.97.41"    # Nuremberg
$SERVER2_IP = "135.181.145.174"  # Helsinki
$IPPAN_USER = "ippan"

function Write-Status {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Green
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

function Write-Header {
    param([string]$Message)
    Write-Host "[HEADER] $Message" -ForegroundColor Blue
}

Write-Header "🚀 Deploying IPPAN on Server 2 and connecting to Server 1"

# Deploy IPPAN on Server 2
$deployScript = @"
set -e
echo "=== Installing dependencies ==="
sudo apt update && sudo apt install -y curl git ufw fail2ban ca-certificates gnupg lsb-release

echo "=== Installing Docker ==="
if ! command -v docker >/dev/null; then
  curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh
  sudo usermod -aG docker `$USER
fi

echo "=== Installing Docker Compose ==="
if ! command -v docker-compose >/dev/null; then
  sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-`$(uname -s)-`$(uname -m)" -o /usr/local/bin/docker-compose && sudo chmod +x /usr/local/bin/docker-compose
fi

echo "=== Configuring firewall ==="
sudo ufw --force reset
sudo ufw default deny incoming && sudo ufw default allow outgoing
sudo ufw allow 22,80,443,3000,8080,9090,3001/tcp
sudo ufw --force enable

echo "=== Setting up IPPAN directory ==="
sudo mkdir -p /opt/ippan && cd /opt/ippan
sudo chown -R ippan:ippan /opt/ippan

echo "=== Cloning IPPAN repository ==="
if [ ! -d mainnet/.git ]; then
  git clone https://github.com/dmrl789/IPPAN.git mainnet
fi
cd mainnet

echo "=== Creating node2 configuration ==="
cat > config.toml << 'EOF'
[network]
bootstrap_nodes = [
    "$SERVER1_IP:8080",  # Node 1 (Nuremberg)
    "$SERVER2_IP:8080"   # Node 2 (Helsinki)
]
listen_address = "0.0.0.0:8080"
external_address = "$SERVER2_IP:8080"

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

echo "=== Creating environment file ==="
cat > .env << 'EOF'
RUST_LOG=info
IPPAN_NETWORK_PORT=8080
IPPAN_API_PORT=3000
IPPAN_STORAGE_DIR=/opt/ippan/data
IPPAN_KEYS_DIR=/opt/ippan/keys
IPPAN_LOG_DIR=/opt/ippan/logs
NODE_ENV=production
RUST_BACKTRACE=1
IPPAN_NODE_ID=node2
IPPAN_BOOTSTRAP_NODES=$SERVER1_IP:8080,$SERVER2_IP:8080
EOF

echo "=== Starting IPPAN services ==="
docker-compose -f docker-compose.production.yml up -d

echo "=== Waiting for services to start ==="
sleep 30

echo "=== Checking service status ==="
docker ps --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'

echo "=== Testing API endpoint ==="
curl -sf http://127.0.0.1:3000/health && echo "API is responding" || echo "API not ready yet"

echo "=== Testing connectivity to Server 1 ==="
timeout 10 bash -c "</dev/tcp/$SERVER1_IP/8080" && echo "Server 1 P2P port is reachable" || echo "Server 1 P2P port not reachable"

echo "=== Deployment complete ==="
"@

Write-Status "Executing deployment script on Server 2..."
$result = ssh -o ConnectTimeout=30 -o StrictHostKeyChecking=no $IPPAN_USER@$SERVER2_IP "bash -lc `"$deployScript`""

if ($LASTEXITCODE -eq 0) {
    Write-Status "✅ Server 2 deployment completed successfully"
} else {
    Write-Error "❌ Server 2 deployment failed"
    Write-Error "Exit code: $LASTEXITCODE"
}

Write-Header "🔍 Testing connectivity after deployment"
Write-Status "Testing Server 2 API..."
try {
    $response = Invoke-WebRequest -Uri "http://$SERVER2_IP:3000/health" -TimeoutSec 10 -UseBasicParsing
    if ($response.StatusCode -eq 200) {
        Write-Status "✅ Server 2 API is responding"
    }
} catch {
    Write-Error "❌ Server 2 API is not responding yet"
}

Write-Status "Testing Server 2 P2P port..."
try {
    $tcpClient = New-Object System.Net.Sockets.TcpClient
    $connect = $tcpClient.BeginConnect($SERVER2_IP, 8080, $null, $null)
    $wait = $connect.AsyncWaitHandle.WaitOne(10000, $false)
    
    if ($wait) {
        $tcpClient.EndConnect($connect)
        $tcpClient.Close()
        Write-Status "✅ Server 2 P2P port is open"
    } else {
        $tcpClient.Close()
        Write-Error "❌ Server 2 P2P port is not accessible"
    }
} catch {
    Write-Error "❌ Server 2 P2P port is not accessible"
}

Write-Header "🎉 Server 2 deployment complete!"
Write-Status "Server 2 URLs:"
Write-Host "  API: http://$SERVER2_IP:3000" -ForegroundColor Cyan
Write-Host "  Grafana: http://$SERVER2_IP:3001" -ForegroundColor Cyan
Write-Host "  Prometheus: http://$SERVER2_IP:9090" -ForegroundColor Cyan
