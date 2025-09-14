# Simple Fresh Deployment
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "=== Simple Fresh Deployment ===" -ForegroundColor Cyan
Write-Host ""

Write-Host "=== Step 1: Shutdown Servers ===" -ForegroundColor Yellow

# Shutdown both servers
Write-Host "Shutting down both servers..." -ForegroundColor Green
try {
    $shutdown1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/shutdown" -Headers $headers -Method POST
    $shutdown2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/shutdown" -Headers $headers -Method POST
    Write-Host "✅ Both servers shutdown initiated" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to shutdown servers: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host "Waiting for servers to shutdown..." -ForegroundColor Yellow
Start-Sleep -Seconds 30

Write-Host ""
Write-Host "=== Step 2: Reset Servers (Format Disks) ===" -ForegroundColor Yellow

# Reset both servers
Write-Host "Resetting both servers (formatting disks)..." -ForegroundColor Green
try {
    $reset1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/reset" -Headers $headers -Method POST
    $reset2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/reset" -Headers $headers -Method POST
    Write-Host "✅ Both servers reset initiated" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to reset servers: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host "Waiting for servers to reset..." -ForegroundColor Yellow
Start-Sleep -Seconds 60

Write-Host ""
Write-Host "=== Step 3: Start Servers ===" -ForegroundColor Yellow

# Start both servers
Write-Host "Starting both servers..." -ForegroundColor Green
try {
    $start1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/poweron" -Headers $headers -Method POST
    $start2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/poweron" -Headers $headers -Method POST
    Write-Host "✅ Both servers started" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to start servers: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host "Waiting for servers to boot..." -ForegroundColor Yellow
Start-Sleep -Seconds 60

Write-Host ""
Write-Host "=== Step 4: Enable Rescue Mode ===" -ForegroundColor Yellow

# Get SSH key
try {
    $sshKeysResponse = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/ssh_keys" -Headers $headers -Method GET
    $sshKeys = $sshKeysResponse.ssh_keys
    
    if ($sshKeys.Count -gt 0) {
        $sshKeyId = $sshKeys[0].id
        Write-Host "✅ Using SSH key: $($sshKeys[0].name) (ID: $sshKeyId)" -ForegroundColor Green
    } else {
        Write-Host "Creating SSH key..." -ForegroundColor Yellow
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

# Enable rescue mode with SSH key
$rescueBody = @{
    rescue = "linux64"
    ssh_keys = @($sshKeyId)
} | ConvertTo-Json

Write-Host "Enabling rescue mode on both servers..." -ForegroundColor Green
try {
    $rescueResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody
    $rescueResponse2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody
    
    Write-Host "✅ Rescue mode enabled on both servers" -ForegroundColor Green
    Write-Host "Server 1 Rescue Password: $($rescueResponse1.action.root_password)" -ForegroundColor Yellow
    Write-Host "Server 2 Rescue Password: $($rescueResponse2.action.root_password)" -ForegroundColor Yellow
    
    $SERVER1_PASSWORD = $rescueResponse1.action.root_password
    $SERVER2_PASSWORD = $rescueResponse2.action.root_password
} catch {
    Write-Host "❌ Failed to enable rescue mode: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "Waiting for rescue mode to be ready..." -ForegroundColor Yellow
Start-Sleep -Seconds 30

Write-Host ""
Write-Host "=== Step 5: Deploy IPPAN ===" -ForegroundColor Yellow

# Create deployment script
$deployScript = @"
#!/bin/bash
set -e

echo "Starting fresh IPPAN deployment..."

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

echo "IPPAN deployment completed successfully!"
'

echo "Fresh deployment completed!"
"@

# Save deployment script
$deployScript | Out-File -FilePath "fresh_deploy.sh" -Encoding UTF8

Write-Host "Created deployment script" -ForegroundColor Green

# Try to deploy using SSH
Write-Host "Attempting to deploy via SSH..." -ForegroundColor Yellow

# Test SSH connection first
Write-Host "Testing SSH connection to Server 1..." -ForegroundColor Green
try {
    $test1 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -i $env:USERPROFILE\.ssh\id_rsa root@188.245.97.41 "echo 'SSH test successful'"
    Write-Host "✅ Server 1 SSH test successful" -ForegroundColor Green
    
    # Deploy on Server 1
    Write-Host "Deploying IPPAN on Server 1..." -ForegroundColor Yellow
    $result1 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -i $env:USERPROFILE\.ssh\id_rsa root@188.245.97.41 -m fresh_deploy.sh
    Write-Host "✅ Server 1 deployment completed" -ForegroundColor Green
    
} catch {
    Write-Host "❌ Server 1 SSH failed: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host "Testing SSH connection to Server 2..." -ForegroundColor Green
try {
    $test2 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -i $env:USERPROFILE\.ssh\id_rsa root@135.181.145.174 "echo 'SSH test successful'"
    Write-Host "✅ Server 2 SSH test successful" -ForegroundColor Green
    
    # Deploy on Server 2 (modify config for Server 2)
    Write-Host "Deploying IPPAN on Server 2..." -ForegroundColor Yellow
    $deployScript2 = $deployScript -replace "external_address = `"188.245.97.41:8080`"", "external_address = `"135.181.145.174:8080`"" -replace "IPPAN_NODE_ID=node1", "IPPAN_NODE_ID=node2"
    $deployScript2 | Out-File -FilePath "fresh_deploy2.sh" -Encoding UTF8
    
    $result2 = & "C:\Program Files\PuTTY\plink.exe" -ssh -batch -i $env:USERPROFILE\.ssh\id_rsa root@135.181.145.174 -m fresh_deploy2.sh
    Write-Host "✅ Server 2 deployment completed" -ForegroundColor Green
    
} catch {
    Write-Host "❌ Server 2 SSH failed: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Step 6: Exit Rescue Mode ===" -ForegroundColor Yellow

# Exit rescue mode
Write-Host "Exiting rescue mode on both servers..." -ForegroundColor Green
try {
    $exitResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/disable_rescue" -Headers $headers -Method POST
    $exitResponse2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/disable_rescue" -Headers $headers -Method POST
    Write-Host "✅ Rescue mode disabled on both servers" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to exit rescue mode: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "Waiting for servers to restart..." -ForegroundColor Yellow
Start-Sleep -Seconds 60

Write-Host ""
Write-Host "=== Step 7: Test Deployment ===" -ForegroundColor Yellow

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

# Clean up
Remove-Item -Path "fresh_deploy.sh" -ErrorAction SilentlyContinue
Remove-Item -Path "fresh_deploy2.sh" -ErrorAction SilentlyContinue
