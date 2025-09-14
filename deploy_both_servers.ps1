# Deploy IPPAN on Both Servers
Import-Module Posh-SSH -ErrorAction SilentlyContinue

$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$RESCUE_PASSWORD = "7LuR4nUCfTiv"

Write-Host "=== IPPAN Deployment on Both Servers ===" -ForegroundColor Cyan
Write-Host ""

# Function to create secure credential
function New-SecureCredential {
    param([string]$Username, [string]$Password)
    $securePassword = ConvertTo-SecureString $Password -AsPlainText -Force
    return New-Object System.Management.Automation.PSCredential($Username, $securePassword)
}

# Function to deploy IPPAN on a server
function Deploy-IPPANOnServer {
    param([string]$ServerIP, [string]$ServerName, [string]$Username, [string]$Password, [string]$NodeID)
    
    Write-Host "=== Deploying IPPAN on $ServerName ($ServerIP) ===" -ForegroundColor Yellow
    
    try {
        # Create credential
        $credential = New-SecureCredential -Username $Username -Password $Password
        
        # Connect to server
        Write-Host "Connecting to $ServerName..." -ForegroundColor Green
        $session = New-SSHSession -ComputerName $ServerIP -Credential $credential -AcceptKey -ConnectionTimeout 30
        
        if ($session) {
            Write-Host "✅ Connected to $ServerName" -ForegroundColor Green
            
            # Create deployment commands
            $deployCommands = @"
# Update system
apt update && apt upgrade -y

# Install essential packages
apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release

# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh

# Create ippan user if it doesn't exist
useradd -m -s /bin/bash -G sudo,docker ippan 2>/dev/null || true

# Create SSH directory for ippan user
mkdir -p /home/ippan/.ssh

# Add SSH public key
echo 'ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQDv7mfvbhDRfBUaCknJ2BP9JEUeU3RT5RKxTsMaAhtdJqvRyz6UchEgkvzMzXbt/w1kvfEcK4ev8ial+y5StTPME6SF95syjvtsNTDhr/cHSUZsV3Nbb6w7wTQGn42O1HV7o4L8Q2Fu+zbNLiHp3PXi8dONlOHptKQif/bNCxRf2uanNwPGVwGHslvEYVNaB++OcVCICmjev/rI8Bx5NZMAz4uUOP7gRwRbTK5YZE8z7X/JFNZleRhaFse8xq2WYTa9rzarkrMagH+b0l/6yLP2qbth71GBMcIY/Az3WJfumyhD5w/EkyzpqREs0kI3LbxxduuHUqDm7tK5FPIRZpSYwNJJ5adq3sx30XD7PbO3k+sh8/UtEtQwyodB9P2hhdpzszE1+TIVBauRohaQnQpwb0NBiE62qdVlN5RUXCV4j5LvFuJCeib/8m8d4H9HdHtA5H/2ZAya//1r5wwcNtgsRx/fagaLfsLMvyBhA/MKCCzrjsRl3HMj9UMOrJfKkZPIsb0W9CQkkpulsjsUlVe82ufh+sAT54niuC/HXZHeokGi51xIyq/ktfdzXoyfq+UBbSfbEIj3jOeyz75Avm7YucGoKuaI2CBghl9i4mXb+orNB1lxYzaBQc/ucilgWbMarP8bAZP9Qpy5TTyHeslodVCPEmyzCWUEts2iEkANGQ== yuyby@hugh' >> /home/ippan/.ssh/authorized_keys

# Set proper permissions
chown -R ippan:ippan /home/ippan/.ssh
chmod 700 /home/ippan/.ssh
chmod 600 /home/ippan/.ssh/authorized_keys

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

echo "Basic setup completed on $ServerName"
"@
            
            # Execute deployment commands
            Write-Host "Running basic setup on $ServerName..." -ForegroundColor Green
            $result = Invoke-SSHCommand -SessionId $session.SessionId -Command $deployCommands -Timeout 300
            
            if ($result.ExitStatus -eq 0) {
                Write-Host "✅ Basic setup completed on $ServerName" -ForegroundColor Green
            } else {
                Write-Host "⚠️ Some commands may have failed on $ServerName" -ForegroundColor Yellow
                Write-Host "Error: $($result.Error)" -ForegroundColor Red
            }
            
            # Deploy IPPAN services
            Write-Host "Deploying IPPAN services on $ServerName..." -ForegroundColor Green
            
            $ippanDeployCommands = @"
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
external_address = "$ServerIP:8080"

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
      - IPPAN_NODE_ID=$NodeID
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

echo "IPPAN deployment completed on $ServerName"
'
"@
            
            $ippanResult = Invoke-SSHCommand -SessionId $session.SessionId -Command $ippanDeployCommands -Timeout 600
            
            if ($ippanResult.ExitStatus -eq 0) {
                Write-Host "✅ IPPAN services deployed on $ServerName" -ForegroundColor Green
            } else {
                Write-Host "⚠️ IPPAN deployment may have issues on $ServerName" -ForegroundColor Yellow
                Write-Host "Error: $($ippanResult.Error)" -ForegroundColor Red
            }
            
            # Check service status
            Write-Host "Checking service status on $ServerName..." -ForegroundColor Green
            $statusResult = Invoke-SSHCommand -SessionId $session.SessionId -Command "docker ps --filter 'name=ippan' --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'" -Timeout 30
            
            if ($statusResult.ExitStatus -eq 0) {
                Write-Host "Service status on $ServerName:" -ForegroundColor Green
                Write-Host $statusResult.Output -ForegroundColor Cyan
            }
            
            # Close session
            Remove-SSHSession -SessionId $session.SessionId
            Write-Host "✅ Deployment completed on $ServerName" -ForegroundColor Green
            
        } else {
            Write-Host "❌ Failed to connect to $ServerName" -ForegroundColor Red
        }
        
    } catch {
        Write-Host "❌ Error deploying to $ServerName`: $_" -ForegroundColor Red
    }
}

# Deploy to both servers
Write-Host "Starting deployment to both servers..." -ForegroundColor Cyan
Write-Host ""

# Deploy to Server 1
Deploy-IPPANOnServer -ServerIP $SERVER1_IP -ServerName "Server 1" -Username "root" -Password $RESCUE_PASSWORD -NodeID "node1"

Write-Host ""
Write-Host "Waiting 30 seconds before deploying to Server 2..." -ForegroundColor Yellow
Start-Sleep -Seconds 30

# Deploy to Server 2
Deploy-IPPANOnServer -ServerIP $SERVER2_IP -ServerName "Server 2" -Username "root" -Password $RESCUE_PASSWORD -NodeID "node2"

Write-Host ""
Write-Host "=== Deployment Complete ===" -ForegroundColor Cyan
Write-Host "Waiting 60 seconds for services to start..." -ForegroundColor Yellow
Start-Sleep -Seconds 60

# Test the deployment
Write-Host "Testing deployment..." -ForegroundColor Green

# Test Server 1 API
try {
    $api1 = Invoke-WebRequest -Uri "http://$SERVER1_IP`:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    if ($api1.StatusCode -eq 200) {
        Write-Host "✅ Server 1 API is responding" -ForegroundColor Green
    } else {
        Write-Host "⚠️ Server 1 API returned status: $($api1.StatusCode)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "❌ Server 1 API is not responding yet" -ForegroundColor Red
}

# Test Server 2 API
try {
    $api2 = Invoke-WebRequest -Uri "http://$SERVER2_IP`:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    if ($api2.StatusCode -eq 200) {
        Write-Host "✅ Server 2 API is responding" -ForegroundColor Green
    } else {
        Write-Host "⚠️ Server 2 API returned status: $($api2.StatusCode)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "❌ Server 2 API is not responding yet" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Access URLs ===" -ForegroundColor Cyan
Write-Host "Server 1 API: http://$SERVER1_IP`:3000" -ForegroundColor White
Write-Host "Server 1 Grafana: http://$SERVER1_IP`:3001" -ForegroundColor White
Write-Host "Server 1 Prometheus: http://$SERVER1_IP`:9090" -ForegroundColor White
Write-Host "Server 2 API: http://$SERVER2_IP`:3000" -ForegroundColor White
Write-Host "Server 2 Grafana: http://$SERVER2_IP`:3001" -ForegroundColor White
Write-Host "Server 2 Prometheus: http://$SERVER2_IP`:9090" -ForegroundColor White

Write-Host ""
Write-Host "🎉 IPPAN deployment completed!" -ForegroundColor Green
Write-Host "The servers should now be connected and running IPPAN services." -ForegroundColor Green
