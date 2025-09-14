# Check SSH Keys and Deploy IPPAN
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "=== Checking SSH Keys and Deploying IPPAN ===" -ForegroundColor Cyan
Write-Host ""

# Get SSH keys
Write-Host "Fetching SSH keys..." -ForegroundColor Green
try {
    $sshKeysResponse = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/ssh_keys" -Headers $headers -Method GET
    $sshKeys = $sshKeysResponse.ssh_keys
    
    Write-Host "Found $($sshKeys.Count) SSH keys:" -ForegroundColor Yellow
    foreach ($key in $sshKeys) {
        Write-Host "Key: $($key.name) (ID: $($key.id))" -ForegroundColor White
        Write-Host "  Fingerprint: $($key.fingerprint)" -ForegroundColor Gray
        Write-Host ""
    }
    
    # Use the first available SSH key
    if ($sshKeys.Count -gt 0) {
        $sshKeyId = $sshKeys[0].id
        Write-Host "Using SSH key: $($sshKeys[0].name) (ID: $sshKeyId)" -ForegroundColor Green
    } else {
        Write-Host "No SSH keys found. Creating one..." -ForegroundColor Yellow
        
        # Get the public key content
        $publicKeyContent = Get-Content $env:USERPROFILE\.ssh\id_rsa.pub -Raw
        
        # Create SSH key
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

# Try to enable rescue mode with the SSH key
Write-Host ""
Write-Host "Enabling rescue mode on Server 1 with SSH key..." -ForegroundColor Green
$rescueBody1 = @{
    rescue = "linux64"
    ssh_keys = @($sshKeyId)
} | ConvertTo-Json

try {
    $rescueResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody1
    Write-Host "✅ Rescue mode enabled on Server 1" -ForegroundColor Green
    Write-Host "Rescue password: $($rescueResponse1.action.root_password)" -ForegroundColor Yellow
    $SERVER1_RESCUE_PASSWORD = $rescueResponse1.action.root_password
} catch {
    Write-Host "❌ Failed to enable rescue mode on Server 1: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Response: $($_.Exception.Response)" -ForegroundColor Red
}

Write-Host ""
Write-Host "Enabling rescue mode on Server 2 with SSH key..." -ForegroundColor Green
$rescueBody2 = @{
    rescue = "linux64"
    ssh_keys = @($sshKeyId)
} | ConvertTo-Json

try {
    $rescueResponse2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody2
    Write-Host "✅ Rescue mode enabled on Server 2" -ForegroundColor Green
    Write-Host "Rescue password: $($rescueResponse2.action.root_password)" -ForegroundColor Yellow
    $SERVER2_RESCUE_PASSWORD = $rescueResponse2.action.root_password
} catch {
    Write-Host "❌ Failed to enable rescue mode on Server 2: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Response: $($_.Exception.Response)" -ForegroundColor Red
}

Write-Host ""
Write-Host "Waiting for rescue mode to be ready..." -ForegroundColor Yellow
Start-Sleep -Seconds 30

# Try direct SSH connection with the rescue passwords
Write-Host "=== Attempting Direct SSH Deployment ===" -ForegroundColor Cyan

# Deploy to Server 1
if ($SERVER1_RESCUE_PASSWORD) {
    Write-Host "Deploying IPPAN on Server 1..." -ForegroundColor Green
    
    $deployScript1 = @"
#!/bin/bash
set -e

echo "Starting deployment on Server 1..."

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

    # Save script to temporary file
    $deployScript1 | Out-File -FilePath "deploy_server1.sh" -Encoding UTF8
    
    # Execute via SSH
    try {
        $result1 = ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no root@$SERVER1_IP "bash -s" < deploy_server1.sh
        Write-Host "✅ Server 1 deployment completed" -ForegroundColor Green
    } catch {
        Write-Host "❌ Server 1 deployment failed: $($_.Exception.Message)" -ForegroundColor Red
    }
}

# Deploy to Server 2
if ($SERVER2_RESCUE_PASSWORD) {
    Write-Host "Deploying IPPAN on Server 2..." -ForegroundColor Green
    
    $deployScript2 = @"
#!/bin/bash
set -e

echo "Starting deployment on Server 2..."

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

    # Save script to temporary file
    $deployScript2 | Out-File -FilePath "deploy_server2.sh" -Encoding UTF8
    
    # Execute via SSH
    try {
        $result2 = ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no root@$SERVER2_IP "bash -s" < deploy_server2.sh
        Write-Host "✅ Server 2 deployment completed" -ForegroundColor Green
    } catch {
        Write-Host "❌ Server 2 deployment failed: $($_.Exception.Message)" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "=== Exiting Rescue Mode ===" -ForegroundColor Cyan

# Exit rescue mode on both servers
Write-Host "Exiting rescue mode on Server 1..." -ForegroundColor Green
try {
    $exitResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/disable_rescue" -Headers $headers -Method POST
    Write-Host "✅ Rescue mode disabled on Server 1" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to exit rescue mode on Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host "Exiting rescue mode on Server 2..." -ForegroundColor Green
try {
    $exitResponse2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/disable_rescue" -Headers $headers -Method POST
    Write-Host "✅ Rescue mode disabled on Server 2" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to exit rescue mode on Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "Waiting for servers to restart..." -ForegroundColor Yellow
Start-Sleep -Seconds 60

Write-Host ""
Write-Host "=== Testing Final Deployment ===" -ForegroundColor Cyan

# Test APIs
try {
    $api1 = Invoke-WebRequest -Uri "http://$SERVER1_IP`:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    Write-Host "✅ Server 1 API responding: $($api1.StatusCode)" -ForegroundColor Green
} catch {
    Write-Host "❌ Server 1 API not responding" -ForegroundColor Red
}

try {
    $api2 = Invoke-WebRequest -Uri "http://$SERVER2_IP`:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    Write-Host "✅ Server 2 API responding: $($api2.StatusCode)" -ForegroundColor Green
} catch {
    Write-Host "❌ Server 2 API not responding" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Deployment Complete ===" -ForegroundColor Green
Write-Host "Server 1 API: http://$SERVER1_IP`:3000" -ForegroundColor White
Write-Host "Server 2 API: http://$SERVER2_IP`:3000" -ForegroundColor White
Write-Host ""
Write-Host "Both servers should now be connected as peers in the IPPAN network!" -ForegroundColor Green

# Clean up temporary files
Remove-Item -Path "deploy_server1.sh" -ErrorAction SilentlyContinue
Remove-Item -Path "deploy_server2.sh" -ErrorAction SilentlyContinue
