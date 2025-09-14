# Enable Rescue Mode and Deploy IPPAN via Hetzner API
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"

$SSH_PUBLIC_KEY = "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQDv7mfvbhDRfBUaCknJ2BP9JEUeU3RT5RKxTsMaAhtdJqvRyz6UchEgkvzMzXbt/w1kvfEcK4ev8ial+y5StTPME6SF95syjvtsNTDhr/cHSUZsV3Nbb6w7wTQGn42O1HV7o4L8Q2Fu+zbNLiHp3PXi8dONlOHptKQif/bNCxRf2uanNwPGVwGHslvEYVNaB++OcVCICmjev/rI8Bx5NZMAz4uUOP7gRwRbTK5YZE8z7X/JFNZleRhaFse8xq2WYTa9rzarkrMagH+b0l/6yLP2qbth71GBMcIY/Az3WJfumyhD5w/EkyzpqREs0kI3LbxxduuHUqDm7tK5FPIRZpSYwNJJ5adq3sx30XD7PbO3k+sh8/UtEtQwyodB9P2hhdpzszE1+TIVBauRohaQnQpwb0NBiE62qdVlN5RUXCV4j5LvFuJCeib/8m8d4H9HdHtA5H/2ZAya//1r5wwcNtgsRx/fagaLfsLMvyBhA/MKCCzrjsRl3HMj9UMOrJfKkZPIsb0W9CQkkpulsjsUlVe82ufh+sAT54niuC/HXZHeokGi51xIyq/ktfdzXoyfq+UBbSfbEIj3jOeyz75Avm7YucGoKuaI2CBghl9i4mXb+orNB1lxYzaBQc/ucilgWbMarP8bAZP9Qpy5TTyHeslodVCPEmyzCWUEts2iEkANGQ== yuyby@hugh"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "=== Enabling Rescue Mode and Deploying IPPAN ===" -ForegroundColor Cyan
Write-Host ""

# Enable rescue mode on Server 1
Write-Host "Enabling rescue mode on Server 1..." -ForegroundColor Green
$rescueBody1 = @{
    rescue = "linux64"
    ssh_keys = @($SSH_PUBLIC_KEY)
} | ConvertTo-Json

try {
    $rescueResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody1
    Write-Host "✅ Rescue mode enabled on Server 1" -ForegroundColor Green
    Write-Host "Rescue password: $($rescueResponse1.action.root_password)" -ForegroundColor Yellow
    $SERVER1_RESCUE_PASSWORD = $rescueResponse1.action.root_password
} catch {
    Write-Host "❌ Failed to enable rescue mode on Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

# Enable rescue mode on Server 2
Write-Host "Enabling rescue mode on Server 2..." -ForegroundColor Green
$rescueBody2 = @{
    rescue = "linux64"
    ssh_keys = @($SSH_PUBLIC_KEY)
} | ConvertTo-Json

try {
    $rescueResponse2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/enable_rescue" -Headers $headers -Method POST -Body $rescueBody2
    Write-Host "✅ Rescue mode enabled on Server 2" -ForegroundColor Green
    Write-Host "Rescue password: $($rescueResponse2.action.root_password)" -ForegroundColor Yellow
    $SERVER2_RESCUE_PASSWORD = $rescueResponse2.action.root_password
} catch {
    Write-Host "❌ Failed to enable rescue mode on Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "Waiting for rescue mode to be ready..." -ForegroundColor Yellow
Start-Sleep -Seconds 30

# Now deploy IPPAN using SSH with rescue passwords
Write-Host "=== Deploying IPPAN via SSH ===" -ForegroundColor Cyan

# Deploy to Server 1
Write-Host "Deploying IPPAN on Server 1..." -ForegroundColor Green
try {
    $securePassword1 = ConvertTo-SecureString $SERVER1_RESCUE_PASSWORD -AsPlainText -Force
    $credential1 = New-Object System.Management.Automation.PSCredential("root", $securePassword1)
    
    $session1 = New-SSHSession -ComputerName $SERVER1_IP -Credential $credential1 -AcceptKey -ConnectionTimeout 15
    
    if ($session1) {
        Write-Host "✅ Connected to Server 1 in rescue mode" -ForegroundColor Green
        
        $deployCommands1 = @"
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
        
        $deployResult1 = Invoke-SSHCommand -SessionId $session1.SessionId -Command $deployCommands1 -Timeout 300
        
        if ($deployResult1.ExitStatus -eq 0) {
            Write-Host "✅ Basic setup completed on Server 1" -ForegroundColor Green
            
            # Deploy IPPAN services
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
            
            $ippanResult1 = Invoke-SSHCommand -SessionId $session1.SessionId -Command $ippanDeploy1 -Timeout 600
            
            if ($ippanResult1.ExitStatus -eq 0) {
                Write-Host "✅ IPPAN services deployed on Server 1" -ForegroundColor Green
            } else {
                Write-Host "⚠️ IPPAN deployment had issues on Server 1" -ForegroundColor Yellow
                Write-Host "Error: $($ippanResult1.Error)" -ForegroundColor Red
            }
        }
        
        Remove-SSHSession -SessionId $session1.SessionId
    }
} catch {
    Write-Host "❌ Failed to deploy on Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

# Deploy to Server 2
Write-Host "Deploying IPPAN on Server 2..." -ForegroundColor Green
try {
    $securePassword2 = ConvertTo-SecureString $SERVER2_RESCUE_PASSWORD -AsPlainText -Force
    $credential2 = New-Object System.Management.Automation.PSCredential("root", $securePassword2)
    
    $session2 = New-SSHSession -ComputerName $SERVER2_IP -Credential $credential2 -AcceptKey -ConnectionTimeout 15
    
    if ($session2) {
        Write-Host "✅ Connected to Server 2 in rescue mode" -ForegroundColor Green
        
        $deployCommands2 = @"
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
        
        $deployResult2 = Invoke-SSHCommand -SessionId $session2.SessionId -Command $deployCommands2 -Timeout 300
        
        if ($deployResult2.ExitStatus -eq 0) {
            Write-Host "✅ Basic setup completed on Server 2" -ForegroundColor Green
            
            # Deploy IPPAN services
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
            
            $ippanResult2 = Invoke-SSHCommand -SessionId $session2.SessionId -Command $ippanDeploy2 -Timeout 600
            
            if ($ippanResult2.ExitStatus -eq 0) {
                Write-Host "✅ IPPAN services deployed on Server 2" -ForegroundColor Green
            } else {
                Write-Host "⚠️ IPPAN deployment had issues on Server 2" -ForegroundColor Yellow
                Write-Host "Error: $($ippanResult2.Error)" -ForegroundColor Red
            }
        }
        
        Remove-SSHSession -SessionId $session2.SessionId
    }
} catch {
    Write-Host "❌ Failed to deploy on Server 2: $($_.Exception.Message)" -ForegroundColor Red
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
