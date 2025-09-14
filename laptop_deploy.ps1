# Deploy IPPAN from Laptop Terminal
$SERVER1_IP = "188.245.97.41"
$SSH_KEY_PATH = "$env:USERPROFILE\.ssh\id_rsa_ippan"

Write-Host "=== DEPLOYING IPPAN FROM LAPTOP TERMINAL ===" -ForegroundColor Cyan

# Function to execute SSH command
function Invoke-SSHCommand {
    param($Command, $Description)
    
    Write-Host "=== $Description ===" -ForegroundColor Yellow
    Write-Host "Executing: $Command" -ForegroundColor Gray
    
    try {
        $result = ssh -o IdentitiesOnly=yes -i $SSH_KEY_PATH -o StrictHostKeyChecking=no -o ConnectTimeout=30 root@$SERVER1_IP $Command
        Write-Host "✅ Success: $Description" -ForegroundColor Green
        if ($result) {
            Write-Host $result -ForegroundColor Cyan
        }
        return $true
    } catch {
        Write-Host "❌ Error in $Description`: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

# Step 1: Update system
$step1 = Invoke-SSHCommand "apt update && apt upgrade -y" "Updating system packages"

# Step 2: Install essential packages
$step2 = Invoke-SSHCommand "apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release" "Installing essential packages"

# Step 3: Install Docker
$step3 = Invoke-SSHCommand "curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh" "Installing Docker"

# Step 4: Install Docker Compose
$step4 = Invoke-SSHCommand "curl -L 'https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)' -o /usr/local/bin/docker-compose && chmod +x /usr/local/bin/docker-compose" "Installing Docker Compose"

# Step 5: Create ippan user
$step5 = Invoke-SSHCommand "useradd -m -s /bin/bash -G sudo,docker ippan 2>/dev/null || true" "Creating ippan user"

# Step 6: Create directories
$step6 = Invoke-SSHCommand "mkdir -p /opt/ippan/mainnet/{data,keys,logs,monitor,grafana,alertmanager,postgres,redis,nginx/logs,fluentd,backups,ssl} && chown -R ippan:ippan /opt/ippan && chmod -R 755 /opt/ippan" "Creating IPPAN directories"

# Step 7: Configure firewall
$step7 = Invoke-SSHCommand "ufw --force reset && ufw default deny incoming && ufw default allow outgoing && ufw allow 22/tcp && ufw allow 80/tcp && ufw allow 443/tcp && ufw allow 3000/tcp && ufw allow 8080/tcp && ufw allow 9090/tcp && ufw allow 3001/tcp && ufw --force enable" "Configuring firewall"

# Step 8: Clone repository
$step8 = Invoke-SSHCommand "su - ippan -c 'cd /opt/ippan && git clone https://github.com/dmrl789/IPPAN.git ippan-repo && cp -r ippan-repo/* mainnet/ && rm -rf ippan-repo'" "Cloning IPPAN repository"

# Step 9: Create configuration file
$configContent = @"
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
"@

$step9 = Invoke-SSHCommand "su - ippan -c 'cd /opt/ippan && cat > mainnet/config.toml << `"EOF`"
$configContent
EOF'" "Creating configuration file"

# Step 10: Create docker-compose file
$dockerComposeContent = @"
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
"@

$step10 = Invoke-SSHCommand "su - ippan -c 'cd /opt/ippan && cat > mainnet/docker-compose.yml << `"EOF`"
$dockerComposeContent
EOF'" "Creating docker-compose file"

# Step 11: Start services
$step11 = Invoke-SSHCommand "su - ippan -c 'cd /opt/ippan/mainnet && docker-compose up -d'" "Starting IPPAN services"

# Step 12: Configure SSH security
$step12 = Invoke-SSHCommand "sed -i 's/#PermitRootLogin yes/PermitRootLogin no/' /etc/ssh/sshd_config && sed -i 's/#PasswordAuthentication yes/PasswordAuthentication no/' /etc/ssh/sshd_config && systemctl restart ssh" "Configuring SSH security"

# Step 13: Start and enable services
$step13 = Invoke-SSHCommand "systemctl restart fail2ban && systemctl enable fail2ban && systemctl restart docker && systemctl enable docker" "Starting and enabling services"

# Step 14: Create completion marker
$step14 = Invoke-SSHCommand "echo 'IPPAN server #1 setup completed at $(date)' > /opt/ippan/mainnet/logs/setup-complete.log && chown ippan:ippan /opt/ippan/mainnet/logs/setup-complete.log" "Creating completion marker"

# Summary
Write-Host ""
Write-Host "=== DEPLOYMENT SUMMARY ===" -ForegroundColor Cyan
$steps = @($step1, $step2, $step3, $step4, $step5, $step6, $step7, $step8, $step9, $step10, $step11, $step12, $step13, $step14)
$successCount = ($steps | Where-Object { $_ -eq $true }).Count
$totalSteps = $steps.Count

Write-Host "Completed: $successCount/$totalSteps steps" -ForegroundColor $(if($successCount -eq $totalSteps){'Green'}else{'Yellow'})

if ($successCount -eq $totalSteps) {
    Write-Host ""
    Write-Host "🎉 IPPAN SERVER 1 DEPLOYMENT COMPLETED SUCCESSFULLY!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Access URLs:" -ForegroundColor Cyan
    Write-Host "- API: http://188.245.97.41:3000" -ForegroundColor White
    Write-Host "- P2P: 188.245.97.41:8080" -ForegroundColor White
    Write-Host "- Metrics: http://188.245.97.41:9090" -ForegroundColor White
    Write-Host ""
    Write-Host "Next: Deploy Server 2 (135.181.145.174)" -ForegroundColor Yellow
} else {
    Write-Host ""
    Write-Host "⚠️ Deployment completed with some issues." -ForegroundColor Yellow
    Write-Host "Check the errors above and retry failed steps." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "=== DEPLOYMENT COMPLETE ===" -ForegroundColor Green
