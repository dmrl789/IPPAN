# Deploy IPPAN Step by Step
$SERVER1_IP = "188.245.97.41"
$SSH_KEY_PATH = "$env:USERPROFILE\.ssh\id_rsa_ippan"

Write-Host "=== DEPLOYING IPPAN STEP BY STEP ===" -ForegroundColor Cyan

# Function to execute SSH command
function Invoke-SSHCommand {
    param($Command)
    
    Write-Host "Executing: $Command" -ForegroundColor Yellow
    
    try {
        $result = ssh -o IdentitiesOnly=yes -i $SSH_KEY_PATH -o StrictHostKeyChecking=no -o ConnectTimeout=30 root@$SERVER1_IP $Command
        Write-Host "Result: $result" -ForegroundColor Green
        return $true
    } catch {
        Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

# Step 1: Update system
Write-Host "=== STEP 1: UPDATING SYSTEM ===" -ForegroundColor Cyan
Invoke-SSHCommand "apt update && apt upgrade -y"

# Step 2: Install packages
Write-Host "=== STEP 2: INSTALLING PACKAGES ===" -ForegroundColor Cyan
Invoke-SSHCommand "apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release"

# Step 3: Install Docker
Write-Host "=== STEP 3: INSTALLING DOCKER ===" -ForegroundColor Cyan
Invoke-SSHCommand "curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh"

# Step 4: Install Docker Compose
Write-Host "=== STEP 4: INSTALLING DOCKER COMPOSE ===" -ForegroundColor Cyan
Invoke-SSHCommand "curl -L 'https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)' -o /usr/local/bin/docker-compose && chmod +x /usr/local/bin/docker-compose"

# Step 5: Create user
Write-Host "=== STEP 5: CREATING IPPAN USER ===" -ForegroundColor Cyan
Invoke-SSHCommand "useradd -m -s /bin/bash -G sudo,docker ippan 2>/dev/null || true"

# Step 6: Create directories
Write-Host "=== STEP 6: CREATING DIRECTORIES ===" -ForegroundColor Cyan
Invoke-SSHCommand "mkdir -p /opt/ippan/mainnet/{data,keys,logs,monitor,grafana,alertmanager,postgres,redis,nginx/logs,fluentd,backups,ssl} && chown -R ippan:ippan /opt/ippan && chmod -R 755 /opt/ippan"

# Step 7: Configure firewall
Write-Host "=== STEP 7: CONFIGURING FIREWALL ===" -ForegroundColor Cyan
Invoke-SSHCommand "ufw --force reset && ufw default deny incoming && ufw default allow outgoing && ufw allow 22/tcp && ufw allow 80/tcp && ufw allow 443/tcp && ufw allow 3000/tcp && ufw allow 8080/tcp && ufw allow 9090/tcp && ufw allow 3001/tcp && ufw --force enable"

# Step 8: Clone repository
Write-Host "=== STEP 8: CLONING IPPAN REPOSITORY ===" -ForegroundColor Cyan
Invoke-SSHCommand "su - ippan -c 'cd /opt/ippan && git clone https://github.com/dmrl789/IPPAN.git ippan-repo && cp -r ippan-repo/* mainnet/ && rm -rf ippan-repo'"

# Step 9: Create configuration
Write-Host "=== STEP 9: CREATING CONFIGURATION ===" -ForegroundColor Cyan
$configCommand = @"
su - ippan -c 'cd /opt/ippan && cat > mainnet/config.toml << "EOF"
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
EOF'
"@

Invoke-SSHCommand $configCommand

# Step 10: Create docker-compose
Write-Host "=== STEP 10: CREATING DOCKER-COMPOSE ===" -ForegroundColor Cyan
$dockerComposeCommand = @"
su - ippan -c 'cd /opt/ippan && cat > mainnet/docker-compose.yml << "EOF"
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
EOF'
"@

Invoke-SSHCommand $dockerComposeCommand

# Step 11: Start services
Write-Host "=== STEP 11: STARTING SERVICES ===" -ForegroundColor Cyan
Invoke-SSHCommand "su - ippan -c 'cd /opt/ippan/mainnet && docker-compose up -d'"

# Step 12: Configure SSH security
Write-Host "=== STEP 12: CONFIGURING SSH SECURITY ===" -ForegroundColor Cyan
Invoke-SSHCommand "sed -i 's/#PermitRootLogin yes/PermitRootLogin no/' /etc/ssh/sshd_config && sed -i 's/#PasswordAuthentication yes/PasswordAuthentication no/' /etc/ssh/sshd_config && systemctl restart ssh"

# Step 13: Start and enable services
Write-Host "=== STEP 13: STARTING AND ENABLING SERVICES ===" -ForegroundColor Cyan
Invoke-SSHCommand "systemctl restart fail2ban && systemctl enable fail2ban && systemctl restart docker && systemctl enable docker"

# Step 14: Create completion marker
Write-Host "=== STEP 14: CREATING COMPLETION MARKER ===" -ForegroundColor Cyan
Invoke-SSHCommand "echo 'IPPAN server #1 setup completed at $(date)' > /opt/ippan/mainnet/logs/setup-complete.log && chown ippan:ippan /opt/ippan/mainnet/logs/setup-complete.log"

Write-Host ""
Write-Host "=== DEPLOYMENT COMPLETE ===" -ForegroundColor Green
Write-Host "✅ IPPAN Server 1 deployment completed!" -ForegroundColor Green
Write-Host ""
Write-Host "Access URLs:" -ForegroundColor Cyan
Write-Host "- API: http://188.245.97.41:3000" -ForegroundColor White
Write-Host "- P2P: 188.245.97.41:8080" -ForegroundColor White
Write-Host "- Metrics: http://188.245.97.41:9090" -ForegroundColor White
