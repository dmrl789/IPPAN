# Final Deployment via Hetzner API and SSH
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "=== Final IPPAN Deployment via Hetzner API ===" -ForegroundColor Cyan
Write-Host ""

# Get or create SSH key
Write-Host "Managing SSH keys..." -ForegroundColor Green
try {
    $sshKeysResponse = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/ssh_keys" -Headers $headers -Method GET
    $sshKeys = $sshKeysResponse.ssh_keys
    
    if ($sshKeys.Count -gt 0) {
        $sshKeyId = $sshKeys[0].id
        Write-Host "✅ Using existing SSH key: $($sshKeys[0].name) (ID: $sshKeyId)" -ForegroundColor Green
    } else {
        Write-Host "Creating new SSH key..." -ForegroundColor Yellow
        $publicKeyContent = Get-Content $env:USERPROFILE\.ssh\id_rsa.pub -Raw
        
        $sshKeyBody = @{
            name = "laptop-key"
            public_key = $publicKeyContent.Trim()
        } | ConvertTo-Json
        
        $newSshKeyResponse = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/ssh_keys" -Headers $headers -Method POST -Body $sshKeyBody
        $sshKeyId = $newSshKeyResponse.ssh_key.id
        Write-Host "✅ Created SSH key with ID: $sshKeyId" -ForegroundColor Green
    }
} catch {
    Write-Host "❌ Failed to manage SSH keys: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# Enable rescue mode on both servers
Write-Host ""
Write-Host "Enabling rescue mode on both servers..." -ForegroundColor Green

$rescueBody = @{
    rescue = "linux64"
    ssh_keys = @($sshKeyId)
} | ConvertTo-Json

# Server 1
try {
    $rescueResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody
    Write-Host "✅ Rescue mode enabled on Server 1" -ForegroundColor Green
    Write-Host "Rescue password: $($rescueResponse1.action.root_password)" -ForegroundColor Yellow
    $SERVER1_RESCUE_PASSWORD = $rescueResponse1.action.root_password
} catch {
    Write-Host "❌ Failed to enable rescue mode on Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

# Server 2
try {
    $rescueResponse2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody
    Write-Host "✅ Rescue mode enabled on Server 2" -ForegroundColor Green
    Write-Host "Rescue password: $($rescueResponse2.action.root_password)" -ForegroundColor Yellow
    $SERVER2_RESCUE_PASSWORD = $rescueResponse2.action.root_password
} catch {
    Write-Host "❌ Failed to enable rescue mode on Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "Waiting for rescue mode to be ready..." -ForegroundColor Yellow
Start-Sleep -Seconds 30

# Create deployment commands for manual execution
Write-Host ""
Write-Host "=== Manual Deployment Commands ===" -ForegroundColor Cyan
Write-Host "Since SSH automation is having issues, here are the commands to run manually:" -ForegroundColor Yellow
Write-Host ""

Write-Host "For Server 1 ($SERVER1_IP):" -ForegroundColor Green
Write-Host "Password: $SERVER1_RESCUE_PASSWORD" -ForegroundColor Yellow
Write-Host ""
Write-Host "Run these commands on Server 1:" -ForegroundColor White
$server1Commands = @"
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

echo "Deployment completed successfully on Server 1"
"@

Write-Host $server1Commands -ForegroundColor Gray

Write-Host ""
Write-Host "For Server 2 ($SERVER2_IP):" -ForegroundColor Green
Write-Host "Password: $SERVER2_RESCUE_PASSWORD" -ForegroundColor Yellow
Write-Host ""
Write-Host "Run these commands on Server 2:" -ForegroundColor White
$server2Commands = @"
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

# Deploy IPPAN as ippan user
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

echo "Deployment completed successfully on Server 2"
"@

Write-Host $server2Commands -ForegroundColor Gray

Write-Host ""
Write-Host "=== After Deployment ===" -ForegroundColor Cyan
Write-Host "1. Run the commands above on both servers" -ForegroundColor Yellow
Write-Host "2. Wait for deployment to complete" -ForegroundColor Yellow
Write-Host "3. Exit rescue mode using the API" -ForegroundColor Yellow
Write-Host "4. Test the APIs" -ForegroundColor Yellow
Write-Host ""

Write-Host "To exit rescue mode after deployment, run:" -ForegroundColor White
Write-Host "powershell -ExecutionPolicy Bypass -File exit_rescue_mode.ps1" -ForegroundColor Gray

# Create exit rescue mode script
$exitRescueScript = @"
# Exit Rescue Mode
`$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
`$SERVER1_ID = "108447288"
`$SERVER2_ID = "108535607"

`$headers = @{
    "Authorization" = "Bearer `$HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "Exiting rescue mode on both servers..." -ForegroundColor Green

# Exit rescue mode on Server 1
try {
    `$exitResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/`$SERVER1_ID/actions/disable_rescue" -Headers `$headers -Method POST
    Write-Host "✅ Rescue mode disabled on Server 1" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to exit rescue mode on Server 1: `$(`$_.Exception.Message)" -ForegroundColor Red
}

# Exit rescue mode on Server 2
try {
    `$exitResponse2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/`$SERVER2_ID/actions/disable_rescue" -Headers `$headers -Method POST
    Write-Host "✅ Rescue mode disabled on Server 2" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to exit rescue mode on Server 2: `$(`$_.Exception.Message)" -ForegroundColor Red
}

Write-Host "Waiting for servers to restart..." -ForegroundColor Yellow
Start-Sleep -Seconds 60

Write-Host "Testing APIs..." -ForegroundColor Green
try {
    `$api1 = Invoke-WebRequest -Uri "http://188.245.97.41:3000/health" -TimeoutSec 10 -UseBasicParsing 2>`$null
    Write-Host "✅ Server 1 API responding: `$(`$api1.StatusCode)" -ForegroundColor Green
} catch {
    Write-Host "❌ Server 1 API not responding" -ForegroundColor Red
}

try {
    `$api2 = Invoke-WebRequest -Uri "http://135.181.145.174:3000/health" -TimeoutSec 10 -UseBasicParsing 2>`$null
    Write-Host "✅ Server 2 API responding: `$(`$api2.StatusCode)" -ForegroundColor Green
} catch {
    Write-Host "❌ Server 2 API not responding" -ForegroundColor Red
}
"@

$exitRescueScript | Out-File -FilePath "exit_rescue_mode.ps1" -Encoding UTF8

Write-Host "✅ Created exit_rescue_mode.ps1 script" -ForegroundColor Green
Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Cyan
Write-Host "Both servers are now in rescue mode with SSH keys injected" -ForegroundColor Green
Write-Host "Use the commands above to deploy IPPAN manually" -ForegroundColor Yellow
Write-Host "Then run exit_rescue_mode.ps1 to exit rescue mode" -ForegroundColor Yellow
