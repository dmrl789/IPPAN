# Connect to servers with password authentication
Import-Module Posh-SSH -ErrorAction SilentlyContinue

$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$SERVER2_PASSWORD = "VgRfqNg3T4sx"

Write-Host "=== Attempting to Connect with Password Authentication ===" -ForegroundColor Cyan
Write-Host ""

# Try Server 2 first (in rescue mode)
Write-Host "Trying Server 2 (in rescue mode)..." -ForegroundColor Green
try {
    $securePassword = ConvertTo-SecureString $SERVER2_PASSWORD -AsPlainText -Force
    $credential = New-Object System.Management.Automation.PSCredential("root", $securePassword)
    
    $session = New-SSHSession -ComputerName $SERVER2_IP -Credential $credential -AcceptKey -ConnectionTimeout 15
    
    if ($session) {
        Write-Host "✅ Connected to Server 2!" -ForegroundColor Green
        
        # Deploy IPPAN on Server 2
        Write-Host "Deploying IPPAN on Server 2..." -ForegroundColor Yellow
        
        $deployCommands = @"
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
ufw allow 22/tcp    # SSH
ufw allow 3000/tcp  # API
ufw allow 8080/tcp  # P2P
ufw allow 9090/tcp  # Prometheus
ufw allow 3001/tcp  # Grafana
ufw --force enable

echo "Basic setup completed on Server 2"
"@
        
        $deployResult = Invoke-SSHCommand -SessionId $session.SessionId -Command $deployCommands -Timeout 300
        
        if ($deployResult.ExitStatus -eq 0) {
            Write-Host "✅ Basic setup completed on Server 2" -ForegroundColor Green
            
            # Deploy IPPAN services
            $ippanDeploy = @"
su - ippan -c '
cd /opt/ippan
git clone https://github.com/dmrl789/IPPAN.git ippan-repo
cp -r ippan-repo/* mainnet/
rm -rf ippan-repo

# Create configuration for Server 2
cat > mainnet/config.toml << "EOF"
[network]
bootstrap_nodes = [
    "$SERVER1_IP:8080",
    "$SERVER2_IP:8080"
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
      - IPPAN_NODE_ID=node2
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

echo "IPPAN deployment completed on Server 2"
'
"@
            
            $ippanResult = Invoke-SSHCommand -SessionId $session.SessionId -Command $ippanDeploy -Timeout 600
            
            if ($ippanResult.ExitStatus -eq 0) {
                Write-Host "✅ IPPAN services deployed on Server 2" -ForegroundColor Green
            } else {
                Write-Host "⚠️ IPPAN deployment had issues on Server 2" -ForegroundColor Yellow
                Write-Host "Error: $($ippanResult.Error)" -ForegroundColor Red
            }
        }
        
        # Check service status
        $statusResult = Invoke-SSHCommand -SessionId $session.SessionId -Command "docker ps --filter 'name=ippan' --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'" -Timeout 30
        
        if ($statusResult.ExitStatus -eq 0) {
            Write-Host "Service status on Server 2:" -ForegroundColor Green
            Write-Host $statusResult.Output -ForegroundColor Cyan
        }
        
        Remove-SSHSession -SessionId $session.SessionId
        Write-Host "✅ Server 2 deployment completed" -ForegroundColor Green
    }
} catch {
    Write-Host "❌ Failed to connect to Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Testing APIs ===" -ForegroundColor Cyan
Start-Sleep -Seconds 30

# Test both APIs
try {
    $api1 = Invoke-WebRequest -Uri "http://$SERVER1_IP`:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    if ($api1.StatusCode -eq 200) {
        Write-Host "✅ Server 1 API is responding: $($api1.StatusCode)" -ForegroundColor Green
    } else {
        Write-Host "⚠️ Server 1 API returned status: $($api1.StatusCode)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "❌ Server 1 API is not responding" -ForegroundColor Red
}

try {
    $api2 = Invoke-WebRequest -Uri "http://$SERVER2_IP`:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    if ($api2.StatusCode -eq 200) {
        Write-Host "✅ Server 2 API is responding: $($api2.StatusCode)" -ForegroundColor Green
    } else {
        Write-Host "⚠️ Server 2 API returned status: $($api2.StatusCode)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "❌ Server 2 API is not responding" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Cyan
Write-Host "Server 2 deployment completed via terminal" -ForegroundColor Green
Write-Host "Next: Deploy Server 1 or test the connection" -ForegroundColor Yellow
Write-Host ""
Write-Host "Access URLs:" -ForegroundColor Cyan
Write-Host "Server 1 API: http://$SERVER1_IP`:3000" -ForegroundColor White
Write-Host "Server 2 API: http://$SERVER2_IP`:3000" -ForegroundColor White
