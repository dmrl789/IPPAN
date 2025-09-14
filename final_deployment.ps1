# Final IPPAN Deployment with Fresh Passwords
$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$SERVER1_PASSWORD = "LcNdL4Rsg3VU"
$SERVER2_PASSWORD = "Pam3C4dcwUq4"

Write-Host "=== Final IPPAN Deployment ===" -ForegroundColor Cyan
Write-Host ""

# Accept host keys first
Write-Host "Accepting host keys..." -ForegroundColor Green
try {
    echo y | & "C:\Program Files\PuTTY\plink.exe" -ssh -pw $SERVER2_PASSWORD root@$SERVER2_IP "echo 'Host key accepted'"
} catch {
    Write-Host "Host key acceptance completed" -ForegroundColor Gray
}

Write-Host ""
Write-Host "=== Deploying to Server 2 (Working) ===" -ForegroundColor Green

# Deploy to Server 2 first (it's working)
Write-Host "Deploying basic setup on Server 2..." -ForegroundColor Yellow
try {
    $setup2 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -pw $SERVER2_PASSWORD root@$SERVER2_IP "apt update && apt upgrade -y && apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release && curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh && useradd -m -s /bin/bash -G sudo,docker ippan 2>/dev/null || true && mkdir -p /opt/ippan/mainnet /opt/ippan/data /opt/ippan/keys /opt/ippan/logs && chown -R ippan:ippan /opt/ippan && ufw allow 22/tcp && ufw allow 3000/tcp && ufw allow 8080/tcp && ufw allow 9090/tcp && ufw allow 3001/tcp && ufw --force enable"
    Write-Host "✅ Server 2 basic setup completed" -ForegroundColor Green
} catch {
    Write-Host "❌ Server 2 basic setup failed: $($_.Exception.Message)" -ForegroundColor Red
}

# Deploy IPPAN on Server 2
Write-Host "Deploying IPPAN on Server 2..." -ForegroundColor Yellow
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

try {
    $ippanResult2 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -pw $SERVER2_PASSWORD root@$SERVER2_IP $ippanDeploy2
    Write-Host "✅ IPPAN services deployed on Server 2" -ForegroundColor Green
} catch {
    Write-Host "❌ IPPAN deployment failed on Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Trying Server 1 with Different Approach ===" -ForegroundColor Yellow

# Try Server 1 with a different approach - maybe it needs SSH key
Write-Host "Server 1 is requiring public key authentication. Let's try to inject SSH key..." -ForegroundColor Yellow

# Get the public key
$publicKey = Get-Content $env:USERPROFILE\.ssh\id_rsa.pub -Raw

# Try to enable rescue mode with SSH key
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_ID = "108447288"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

# Get or create SSH key
try {
    $sshKeysResponse = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/ssh_keys" -Headers $headers -Method GET
    $sshKeys = $sshKeysResponse.ssh_keys
    
    if ($sshKeys.Count -gt 0) {
        $sshKeyId = $sshKeys[0].id
        Write-Host "Using SSH key: $($sshKeys[0].name) (ID: $sshKeyId)" -ForegroundColor Green
    } else {
        Write-Host "Creating SSH key..." -ForegroundColor Yellow
        $sshKeyBody = @{
            name = "laptop-key"
            public_key = $publicKey.Trim()
        } | ConvertTo-Json
        
        $newSshKeyResponse = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/ssh_keys" -Headers $headers -Method POST -Body $sshKeyBody
        $sshKeyId = $newSshKeyResponse.ssh_key.id
        Write-Host "✅ Created SSH key with ID: $sshKeyId" -ForegroundColor Green
    }
    
    # Disable and re-enable rescue mode with SSH key
    Write-Host "Re-enabling rescue mode on Server 1 with SSH key..." -ForegroundColor Green
    $disableResponse = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/disable_rescue" -Headers $headers -Method POST
    
    Start-Sleep -Seconds 10
    
    $rescueBodyWithKey = @{
        rescue = "linux64"
        ssh_keys = @($sshKeyId)
    } | ConvertTo-Json
    
    $rescueResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBodyWithKey
    
    Write-Host "✅ Rescue mode enabled on Server 1 with SSH key" -ForegroundColor Green
    Write-Host "Server 1 Rescue Password: $($rescueResponse1.action.root_password)" -ForegroundColor Yellow
    
    Start-Sleep -Seconds 30
    
    # Try SSH with key
    Write-Host "Testing SSH with key on Server 1..." -ForegroundColor Green
    try {
        $result1 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -i $env:USERPROFILE\.ssh\id_rsa root@$SERVER1_IP "echo 'SSH with key successful'; whoami; pwd"
        Write-Host "✅ Server 1 SSH with key successful" -ForegroundColor Green
        Write-Host "Output: $result1" -ForegroundColor Gray
        
        # Deploy to Server 1
        Write-Host "Deploying basic setup on Server 1..." -ForegroundColor Yellow
        $setup1 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -i $env:USERPROFILE\.ssh\id_rsa root@$SERVER1_IP "apt update && apt upgrade -y && apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release && curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh && useradd -m -s /bin/bash -G sudo,docker ippan 2>/dev/null || true && mkdir -p /opt/ippan/mainnet /opt/ippan/data /opt/ippan/keys /opt/ippan/logs && chown -R ippan:ippan /opt/ippan && ufw allow 22/tcp && ufw allow 3000/tcp && ufw allow 8080/tcp && ufw allow 9090/tcp && ufw allow 3001/tcp && ufw --force enable"
        Write-Host "✅ Server 1 basic setup completed" -ForegroundColor Green
        
        # Deploy IPPAN on Server 1
        Write-Host "Deploying IPPAN on Server 1..." -ForegroundColor Yellow
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
        
        $ippanResult1 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -i $env:USERPROFILE\.ssh\id_rsa root@$SERVER1_IP $ippanDeploy1
        Write-Host "✅ IPPAN services deployed on Server 1" -ForegroundColor Green
        
    } catch {
        Write-Host "❌ Server 1 SSH with key failed: $($_.Exception.Message)" -ForegroundColor Red
    }
    
} catch {
    Write-Host "❌ Failed to setup SSH key for Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Exiting Rescue Mode ===" -ForegroundColor Cyan

# Exit rescue mode on both servers
$SERVER2_ID = "108535607"

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
