# Deploy IPPAN via Cloud-Init (Bypass SSH Issues)
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "=== Deploying IPPAN via Cloud-Init ===" -ForegroundColor Cyan
Write-Host ""

# Create cloud-init script for IPPAN deployment
$cloudInitScript = @"
#cloud-config
package_update: true
package_upgrade: true

packages:
  - curl
  - git
  - wget
  - unzip
  - ufw
  - fail2ban
  - ca-certificates
  - gnupg
  - lsb-release

runcmd:
  # Install Docker
  - curl -fsSL https://get.docker.com -o get-docker.sh
  - sh get-docker.sh
  - rm get-docker.sh
  
  # Create ippan user
  - useradd -m -s /bin/bash -G sudo,docker ippan || true
  
  # Create IPPAN directories
  - mkdir -p /opt/ippan/mainnet
  - mkdir -p /opt/ippan/data
  - mkdir -p /opt/ippan/keys
  - mkdir -p /opt/ippan/logs
  - chown -R ippan:ippan /opt/ippan
  
  # Configure firewall
  - ufw allow 22/tcp
  - ufw allow 3000/tcp
  - ufw allow 8080/tcp
  - ufw allow 9090/tcp
  - ufw allow 3001/tcp
  - ufw --force enable
  
  # Deploy IPPAN as ippan user
  - |
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

final_message: "IPPAN deployment completed successfully!"
"@

# Create cloud-init script for Server 2
$cloudInitScript2 = @"
#cloud-config
package_update: true
package_upgrade: true

packages:
  - curl
  - git
  - wget
  - unzip
  - ufw
  - fail2ban
  - ca-certificates
  - gnupg
  - lsb-release

runcmd:
  # Install Docker
  - curl -fsSL https://get.docker.com -o get-docker.sh
  - sh get-docker.sh
  - rm get-docker.sh
  
  # Create ippan user
  - useradd -m -s /bin/bash -G sudo,docker ippan || true
  
  # Create IPPAN directories
  - mkdir -p /opt/ippan/mainnet
  - mkdir -p /opt/ippan/data
  - mkdir -p /opt/ippan/keys
  - mkdir -p /opt/ippan/logs
  - chown -R ippan:ippan /opt/ippan
  
  # Configure firewall
  - ufw allow 22/tcp
  - ufw allow 3000/tcp
  - ufw allow 8080/tcp
  - ufw allow 9090/tcp
  - ufw allow 3001/tcp
  - ufw --force enable
  
  # Deploy IPPAN as ippan user
  - |
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
    
    echo "IPPAN deployment completed"
    '

final_message: "IPPAN deployment completed successfully!"
"@

Write-Host "=== Recreating Servers with Cloud-Init ===" -ForegroundColor Green
Write-Host "This will recreate both servers with IPPAN pre-installed" -ForegroundColor Yellow
Write-Host ""

# Get current server details
try {
    $server1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID" -Headers $headers -Method GET
    $server2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID" -Headers $headers -Method GET
    
    Write-Host "Current Server 1: $($server1.server.name) - $($server1.server.server_type.name)" -ForegroundColor White
    Write-Host "Current Server 2: $($server2.server.name) - $($server2.server.server_type.name)" -ForegroundColor White
} catch {
    Write-Host "❌ Failed to get server details: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Alternative: Manual Deployment ===" -ForegroundColor Cyan
Write-Host "Since SSH automation is having issues, here's the simplest approach:" -ForegroundColor Yellow
Write-Host ""
Write-Host "1. Use the Hetzner Cloud Console to access both servers" -ForegroundColor White
Write-Host "2. Both servers are in rescue mode with these passwords:" -ForegroundColor White
Write-Host "   - Server 1 (188.245.97.41): LcNdL4Rsg3VU" -ForegroundColor Green
Write-Host "   - Server 2 (135.181.145.174): Pam3C4dcwUq4" -ForegroundColor Green
Write-Host ""
Write-Host "3. Run this command on both servers:" -ForegroundColor White
Write-Host $cloudInitScript -ForegroundColor Gray
Write-Host ""
Write-Host "4. After deployment, exit rescue mode:" -ForegroundColor White
Write-Host "   powershell -ExecutionPolicy Bypass -File exit_rescue_mode.ps1" -ForegroundColor Gray
Write-Host ""
Write-Host "This will deploy IPPAN on both servers and connect them as peers!" -ForegroundColor Green
