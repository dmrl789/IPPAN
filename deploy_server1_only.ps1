# Deploy IPPAN on Server 1 Only
Import-Module Posh-SSH -ErrorAction SilentlyContinue

$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$SERVER1_PASSWORD = "7LuR4nUCfTiv"

Write-Host "=== Deploying IPPAN on Server 1 ===" -ForegroundColor Cyan
Write-Host ""

try {
    # Create credential
    $securePassword = ConvertTo-SecureString $SERVER1_PASSWORD -AsPlainText -Force
    $credential = New-Object System.Management.Automation.PSCredential("root", $securePassword)
    
    # Connect to Server 1
    Write-Host "Connecting to Server 1..." -ForegroundColor Green
    $session = New-SSHSession -ComputerName $SERVER1_IP -Credential $credential -AcceptKey -ConnectionTimeout 30
    
    if ($session) {
        Write-Host "✅ Connected to Server 1" -ForegroundColor Green
        
        # Check current status
        Write-Host "Checking current status..." -ForegroundColor Yellow
        $statusCheck = Invoke-SSHCommand -SessionId $session.SessionId -Command "docker ps --filter 'name=ippan' --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'" -Timeout 30
        
        if ($statusCheck.ExitStatus -eq 0) {
            Write-Host "Current Docker status:" -ForegroundColor Cyan
            Write-Host $statusCheck.Output -ForegroundColor White
        }
        
        # Check if IPPAN is already deployed
        $ippanCheck = Invoke-SSHCommand -SessionId $session.SessionId -Command "ls -la /opt/ippan/mainnet/ 2>/dev/null || echo 'IPPAN not found'" -Timeout 30
        
        if ($ippanCheck.Output -like "*IPPAN not found*") {
            Write-Host "IPPAN not found. Deploying..." -ForegroundColor Yellow
            
            # Deploy IPPAN
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

echo "Basic setup completed on Server 1"
"@
            
            Write-Host "Running basic setup..." -ForegroundColor Green
            $deployResult = Invoke-SSHCommand -SessionId $session.SessionId -Command $deployCommands -Timeout 300
            
            if ($deployResult.ExitStatus -eq 0) {
                Write-Host "✅ Basic setup completed" -ForegroundColor Green
                
                # Deploy IPPAN services
                Write-Host "Deploying IPPAN services..." -ForegroundColor Green
                
                $ippanDeploy = @"
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
external_address = "$SERVER1_IP:8080"

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

echo "IPPAN deployment completed on Server 1"
'
"@
                
                $ippanResult = Invoke-SSHCommand -SessionId $session.SessionId -Command $ippanDeploy -Timeout 600
                
                if ($ippanResult.ExitStatus -eq 0) {
                    Write-Host "✅ IPPAN services deployed on Server 1" -ForegroundColor Green
                } else {
                    Write-Host "⚠️ IPPAN deployment had issues" -ForegroundColor Yellow
                    Write-Host "Error: $($ippanResult.Error)" -ForegroundColor Red
                }
            }
        } else {
            Write-Host "✅ IPPAN already exists on Server 1" -ForegroundColor Green
            
            # Try to start services
            Write-Host "Starting IPPAN services..." -ForegroundColor Green
            $startResult = Invoke-SSHCommand -SessionId $session.SessionId -Command "su - ippan -c 'cd /opt/ippan/mainnet && docker-compose up -d'" -Timeout 300
            
            if ($startResult.ExitStatus -eq 0) {
                Write-Host "✅ IPPAN services started" -ForegroundColor Green
            } else {
                Write-Host "⚠️ Failed to start services" -ForegroundColor Yellow
            }
        }
        
        # Check final status
        Write-Host "Checking final status..." -ForegroundColor Green
        $finalStatus = Invoke-SSHCommand -SessionId $session.SessionId -Command "docker ps --filter 'name=ippan' --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'" -Timeout 30
        
        if ($finalStatus.ExitStatus -eq 0) {
            Write-Host "Final Docker status:" -ForegroundColor Cyan
            Write-Host $finalStatus.Output -ForegroundColor White
        }
        
        # Close session
        Remove-SSHSession -SessionId $session.SessionId
        Write-Host "✅ Server 1 deployment completed" -ForegroundColor Green
        
    } else {
        Write-Host "❌ Failed to connect to Server 1" -ForegroundColor Red
    }
    
} catch {
    Write-Host "❌ Error: $_" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Testing Server 1 API ===" -ForegroundColor Cyan
Start-Sleep -Seconds 30

try {
    $response = Invoke-WebRequest -Uri "http://$SERVER1_IP`:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    if ($response.StatusCode -eq 200) {
        Write-Host "✅ Server 1 API is responding: $($response.StatusCode)" -ForegroundColor Green
        Write-Host "Response: $($response.Content)" -ForegroundColor Cyan
    } else {
        Write-Host "⚠️ Server 1 API returned status: $($response.StatusCode)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "❌ Server 1 API is not responding yet: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Next Steps ===" -ForegroundColor Cyan
Write-Host "1. Server 1 deployment completed" -ForegroundColor White
Write-Host "2. Need to deploy Server 2 (SSH key exchange issue)" -ForegroundColor White
Write-Host "3. Once both are running, they will connect automatically" -ForegroundColor White
Write-Host ""
Write-Host "Access URLs:" -ForegroundColor Cyan
Write-Host "Server 1 API: http://$SERVER1_IP`:3000" -ForegroundColor White
Write-Host "Server 1 Grafana: http://$SERVER1_IP`:3001" -ForegroundColor White
Write-Host "Server 1 Prometheus: http://$SERVER1_IP`:9090" -ForegroundColor White
