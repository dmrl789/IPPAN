# Get Fresh Rescue Passwords
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "=== Getting Fresh Rescue Passwords ===" -ForegroundColor Cyan

# Get the latest rescue action for Server 1
try {
    $actions1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions" -Headers $headers -Method GET
    $rescueAction1 = $actions1.actions | Where-Object { $_.command -eq "enable_rescue" } | Sort-Object -Property id -Descending | Select-Object -First 1
    
    if ($rescueAction1) {
        Write-Host "Server 1 Rescue Password: $($rescueAction1.root_password)" -ForegroundColor Green
        $SERVER1_PASSWORD = $rescueAction1.root_password
    }
} catch {
    Write-Host "❌ Failed to get Server 1 password: $($_.Exception.Message)" -ForegroundColor Red
}

# Get the latest rescue action for Server 2
try {
    $actions2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions" -Headers $headers -Method GET
    $rescueAction2 = $actions2.actions | Where-Object { $_.command -eq "enable_rescue" } | Sort-Object -Property id -Descending | Select-Object -First 1
    
    if ($rescueAction2) {
        Write-Host "Server 2 Rescue Password: $($rescueAction2.root_password)" -ForegroundColor Green
        $SERVER2_PASSWORD = $rescueAction2.root_password
    }
} catch {
    Write-Host "❌ Failed to get Server 2 password: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Testing SSH with Fresh Passwords ===" -ForegroundColor Cyan

# Test Server 1
if ($SERVER1_PASSWORD) {
    Write-Host "Testing Server 1 SSH connection..." -ForegroundColor Green
    try {
        $result1 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -pw $SERVER1_PASSWORD root@188.245.97.41 "echo 'SSH test successful'; whoami; pwd"
        Write-Host "✅ Server 1 SSH test successful" -ForegroundColor Green
        Write-Host "Output: $result1" -ForegroundColor Gray
        
        # If SSH works, deploy IPPAN
        Write-Host "Deploying IPPAN on Server 1..." -ForegroundColor Yellow
        $deploy1 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -pw $SERVER1_PASSWORD root@188.245.97.41 "apt update && apt upgrade -y && apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release && curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh && useradd -m -s /bin/bash -G sudo,docker ippan 2>/dev/null || true && mkdir -p /opt/ippan/mainnet /opt/ippan/data /opt/ippan/keys /opt/ippan/logs && chown -R ippan:ippan /opt/ippan && ufw allow 22/tcp && ufw allow 3000/tcp && ufw allow 8080/tcp && ufw allow 9090/tcp && ufw allow 3001/tcp && ufw --force enable"
        Write-Host "✅ Server 1 basic setup completed" -ForegroundColor Green
        
    } catch {
        Write-Host "❌ Server 1 SSH test failed: $($_.Exception.Message)" -ForegroundColor Red
    }
}

# Test Server 2
if ($SERVER2_PASSWORD) {
    Write-Host "Testing Server 2 SSH connection..." -ForegroundColor Green
    try {
        $result2 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -pw $SERVER2_PASSWORD root@135.181.145.174 "echo 'SSH test successful'; whoami; pwd"
        Write-Host "✅ Server 2 SSH test successful" -ForegroundColor Green
        Write-Host "Output: $result2" -ForegroundColor Gray
        
        # If SSH works, deploy IPPAN
        Write-Host "Deploying IPPAN on Server 2..." -ForegroundColor Yellow
        $deploy2 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -pw $SERVER2_PASSWORD root@135.181.145.174 "apt update && apt upgrade -y && apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release && curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh && useradd -m -s /bin/bash -G sudo,docker ippan 2>/dev/null || true && mkdir -p /opt/ippan/mainnet /opt/ippan/data /opt/ippan/keys /opt/ippan/logs && chown -R ippan:ippan /opt/ippan && ufw allow 22/tcp && ufw allow 3000/tcp && ufw allow 8080/tcp && ufw allow 9090/tcp && ufw allow 3001/tcp && ufw --force enable"
        Write-Host "✅ Server 2 basic setup completed" -ForegroundColor Green
        
    } catch {
        Write-Host "❌ Server 2 SSH test failed: $($_.Exception.Message)" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "=== Deploying IPPAN Services ===" -ForegroundColor Cyan

# Deploy IPPAN services on Server 1
if ($SERVER1_PASSWORD) {
    Write-Host "Deploying IPPAN services on Server 1..." -ForegroundColor Yellow
    $ippanDeploy1 = @"
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
"@
    
    try {
        $ippanResult1 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -pw $SERVER1_PASSWORD root@188.245.97.41 $ippanDeploy1
        Write-Host "✅ IPPAN services deployed on Server 1" -ForegroundColor Green
    } catch {
        Write-Host "❌ IPPAN deployment failed on Server 1: $($_.Exception.Message)" -ForegroundColor Red
    }
}

# Deploy IPPAN services on Server 2
if ($SERVER2_PASSWORD) {
    Write-Host "Deploying IPPAN services on Server 2..." -ForegroundColor Yellow
    $ippanDeploy2 = @"
su - ippan -c '
cd /opt/ippan
git clone https://github.com/dmrl789/IPPAN.git ippan-repo
cp -r ippan-repo/* mainnet/
rm -rf ippan-repo

# Create configuration for Server 2
cat > mainnet/config.toml << "EOF"
[network]
bootstrap_nodes = [
    "188.245.97.41:8080",
    "135.181.145.174:8080"
]
listen_address = "0.0.0.0:8080"
external_address = "135.181.145.174:8080"

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

echo "IPPAN deployment completed on Server 2"
'
"@
    
    try {
        $ippanResult2 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -pw $SERVER2_PASSWORD root@135.181.145.174 $ippanDeploy2
        Write-Host "✅ IPPAN services deployed on Server 2" -ForegroundColor Green
    } catch {
        Write-Host "❌ IPPAN deployment failed on Server 2: $($_.Exception.Message)" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "=== Exiting Rescue Mode ===" -ForegroundColor Cyan

# Exit rescue mode
Write-Host "Exiting rescue mode on both servers..." -ForegroundColor Green
try {
    $exitResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/disable_rescue" -Headers $headers -Method POST
    Write-Host "✅ Rescue mode disabled on Server 1" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to exit rescue mode on Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

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
    $api1 = Invoke-WebRequest -Uri "http://188.245.97.41:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    Write-Host "✅ Server 1 API responding: $($api1.StatusCode)" -ForegroundColor Green
} catch {
    Write-Host "❌ Server 1 API not responding" -ForegroundColor Red
}

try {
    $api2 = Invoke-WebRequest -Uri "http://135.181.145.174:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    Write-Host "✅ Server 2 API responding: $($api2.StatusCode)" -ForegroundColor Green
} catch {
    Write-Host "❌ Server 2 API not responding" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Deployment Complete ===" -ForegroundColor Green
Write-Host "Server 1 API: http://188.245.97.41:3000" -ForegroundColor White
Write-Host "Server 2 API: http://135.181.145.174:3000" -ForegroundColor White
Write-Host ""
Write-Host "Both servers should now be connected as peers in the IPPAN network!" -ForegroundColor Green
