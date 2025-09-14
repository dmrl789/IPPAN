# Fresh Deployment - Format and Start Afresh
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "=== Fresh Deployment - Format and Start Afresh ===" -ForegroundColor Cyan
Write-Host ""

# Get current server details
Write-Host "Getting current server details..." -ForegroundColor Green
try {
    $server1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID" -Headers $headers -Method GET
    $server2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID" -Headers $headers -Method GET
    
    Write-Host "Server 1: $($server1.server.name) - $($server1.server.server_type.name)" -ForegroundColor White
    Write-Host "Server 2: $($server2.server.name) - $($server2.server.server_type.name)" -ForegroundColor White
} catch {
    Write-Host "❌ Failed to get server details: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Step 1: Shutdown Servers ===" -ForegroundColor Yellow

# Shutdown Server 1
Write-Host "Shutting down Server 1..." -ForegroundColor Green
try {
    $shutdown1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/shutdown" -Headers $headers -Method POST
    Write-Host "✅ Server 1 shutdown initiated" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to shutdown Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

# Shutdown Server 2
Write-Host "Shutting down Server 2..." -ForegroundColor Green
try {
    $shutdown2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/shutdown" -Headers $headers -Method POST
    Write-Host "✅ Server 2 shutdown initiated" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to shutdown Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "Waiting for servers to shutdown..." -ForegroundColor Yellow
Start-Sleep -Seconds 30

Write-Host ""
Write-Host "=== Step 2: Reset Servers (Format Disks) ===" -ForegroundColor Yellow

# Reset Server 1
Write-Host "Resetting Server 1 (formatting disk)..." -ForegroundColor Green
try {
    $reset1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/reset" -Headers $headers -Method POST
    Write-Host "✅ Server 1 reset initiated" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to reset Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

# Reset Server 2
Write-Host "Resetting Server 2 (formatting disk)..." -ForegroundColor Green
try {
    $reset2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/reset" -Headers $headers -Method POST
    Write-Host "✅ Server 2 reset initiated" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to reset Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "Waiting for servers to reset..." -ForegroundColor Yellow
Start-Sleep -Seconds 60

Write-Host ""
Write-Host "=== Step 3: Start Servers ===" -ForegroundColor Yellow

# Start Server 1
Write-Host "Starting Server 1..." -ForegroundColor Green
try {
    $start1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/poweron" -Headers $headers -Method POST
    Write-Host "✅ Server 1 started" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to start Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

# Start Server 2
Write-Host "Starting Server 2..." -ForegroundColor Green
try {
    $start2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/poweron" -Headers $headers -Method POST
    Write-Host "✅ Server 2 started" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to start Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "Waiting for servers to boot..." -ForegroundColor Yellow
Start-Sleep -Seconds 60

Write-Host ""
Write-Host "=== Step 4: Enable Rescue Mode with SSH Key ===" -ForegroundColor Yellow

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

# Enable rescue mode with SSH key on Server 1
Write-Host "Enabling rescue mode on Server 1 with SSH key..." -ForegroundColor Green
$rescueBody1 = @{
    rescue = "linux64"
    ssh_keys = @($sshKeyId)
} | ConvertTo-Json

try {
    $rescueResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody1
    Write-Host "✅ Rescue mode enabled on Server 1" -ForegroundColor Green
    Write-Host "Server 1 Rescue Password: $($rescueResponse1.action.root_password)" -ForegroundColor Yellow
    $SERVER1_PASSWORD = $rescueResponse1.action.root_password
} catch {
    Write-Host "❌ Failed to enable rescue mode on Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

# Enable rescue mode with SSH key on Server 2
Write-Host "Enabling rescue mode on Server 2 with SSH key..." -ForegroundColor Green
$rescueBody2 = @{
    rescue = "linux64"
    ssh_keys = @($sshKeyId)
} | ConvertTo-Json

try {
    $rescueResponse2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody2
    Write-Host "✅ Rescue mode enabled on Server 2" -ForegroundColor Green
    Write-Host "Server 2 Rescue Password: $($rescueResponse2.action.root_password)" -ForegroundColor Yellow
    $SERVER2_PASSWORD = $rescueResponse2.action.root_password
} catch {
    Write-Host "❌ Failed to enable rescue mode on Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "Waiting for rescue mode to be ready..." -ForegroundColor Yellow
Start-Sleep -Seconds 30

Write-Host ""
Write-Host "=== Step 5: Deploy IPPAN via SSH ===" -ForegroundColor Yellow

# Test SSH connection and deploy
Write-Host "Testing SSH connection to Server 1..." -ForegroundColor Green
try {
    $test1 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -i $env:USERPROFILE\.ssh\id_rsa root@188.245.97.41 "echo 'SSH test successful'; whoami; pwd"
    Write-Host "✅ Server 1 SSH test successful" -ForegroundColor Green
    Write-Host "Output: $test1" -ForegroundColor Gray
    
    # Deploy IPPAN on Server 1
    Write-Host "Deploying IPPAN on Server 1..." -ForegroundColor Yellow
    $deploy1 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -i $env:USERPROFILE\.ssh\id_rsa root@188.245.97.41 "apt update && apt upgrade -y && apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release && curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh && useradd -m -s /bin/bash -G sudo,docker ippan 2>/dev/null || true && mkdir -p /opt/ippan/mainnet /opt/ippan/data /opt/ippan/keys /opt/ippan/logs && chown -R ippan:ippan /opt/ippan && ufw allow 22/tcp && ufw allow 3000/tcp && ufw allow 8080/tcp && ufw allow 9090/tcp && ufw allow 3001/tcp && ufw --force enable"
    Write-Host "✅ Server 1 basic setup completed" -ForegroundColor Green
    
} catch {
    Write-Host "❌ Server 1 SSH failed: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host "Testing SSH connection to Server 2..." -ForegroundColor Green
try {
    $test2 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -i $env:USERPROFILE\.ssh\id_rsa root@135.181.145.174 "echo 'SSH test successful'; whoami; pwd"
    Write-Host "✅ Server 2 SSH test successful" -ForegroundColor Green
    Write-Host "Output: $test2" -ForegroundColor Gray
    
    # Deploy IPPAN on Server 2
    Write-Host "Deploying IPPAN on Server 2..." -ForegroundColor Yellow
    $deploy2 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -i $env:USERPROFILE\.ssh\id_rsa root@135.181.145.174 "apt update && apt upgrade -y && apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release && curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh && useradd -m -s /bin/bash -G sudo,docker ippan 2>/dev/null || true && mkdir -p /opt/ippan/mainnet /opt/ippan/data /opt/ippan/keys /opt/ippan/logs && chown -R ippan:ippan /opt/ippan && ufw allow 22/tcp && ufw allow 3000/tcp && ufw allow 8080/tcp && ufw allow 9090/tcp && ufw allow 3001/tcp && ufw --force enable"
    Write-Host "✅ Server 2 basic setup completed" -ForegroundColor Green
    
} catch {
    Write-Host "❌ Server 2 SSH failed: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Step 6: Deploy IPPAN Services ===" -ForegroundColor Yellow

# Deploy IPPAN services on both servers
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
"@

# Deploy on Server 1
Write-Host "Deploying IPPAN services on Server 1..." -ForegroundColor Yellow
try {
    $ippanResult1 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -i $env:USERPROFILE\.ssh\id_rsa root@188.245.97.41 $ippanDeploy
    Write-Host "✅ IPPAN services deployed on Server 1" -ForegroundColor Green
} catch {
    Write-Host "❌ IPPAN deployment failed on Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

# Deploy on Server 2 (with different node ID and external address)
$ippanDeploy2 = $ippanDeploy -replace "external_address = \"188.245.97.41:8080\"", "external_address = \"135.181.145.174:8080\"" -replace "IPPAN_NODE_ID=node1", "IPPAN_NODE_ID=node2"

Write-Host "Deploying IPPAN services on Server 2..." -ForegroundColor Yellow
try {
    $ippanResult2 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -i $env:USERPROFILE\.ssh\id_rsa root@135.181.145.174 $ippanDeploy2
    Write-Host "✅ IPPAN services deployed on Server 2" -ForegroundColor Green
} catch {
    Write-Host "❌ IPPAN deployment failed on Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Step 7: Exit Rescue Mode ===" -ForegroundColor Yellow

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
Write-Host "=== Step 8: Test Final Deployment ===" -ForegroundColor Yellow

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
Write-Host "=== Fresh Deployment Complete ===" -ForegroundColor Green
Write-Host "Server 1 API: http://188.245.97.41:3000" -ForegroundColor White
Write-Host "Server 2 API: http://135.181.145.174:3000" -ForegroundColor White
Write-Host ""
Write-Host "Both servers have been formatted and freshly deployed with IPPAN!" -ForegroundColor Green
Write-Host "They should now be connected as peers in the IPPAN network!" -ForegroundColor Green
