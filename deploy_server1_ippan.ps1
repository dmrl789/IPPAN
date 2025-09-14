# Deploy IPPAN Services on Server 1
# Run this after SSH access is working

$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$IPPAN_USER = "ippan"

Write-Host "=== IPPAN Server 1 Deployment ===" -ForegroundColor Cyan
Write-Host "Server 1: $SERVER1_IP" -ForegroundColor Blue
Write-Host "Server 2: $SERVER2_IP" -ForegroundColor Blue
Write-Host ""

Write-Host "=== Step 1: Connect to Server 1 ===" -ForegroundColor Blue
Write-Host "First, connect to Server 1:" -ForegroundColor White
Write-Host "ssh $IPPAN_USER@$SERVER1_IP" -ForegroundColor Yellow
Write-Host ""

Write-Host "=== Step 2: Deploy IPPAN Services ===" -ForegroundColor Blue
Write-Host "Run these commands on Server 1:" -ForegroundColor White
Write-Host ""

$deployCommands = @"
# Clone IPPAN repository
cd /opt/ippan
git clone https://github.com/dmrl789/IPPAN.git ippan-repo
cp -r ippan-repo/* mainnet/
rm -rf ippan-repo

# Create configuration for Server 1
cat > mainnet/config.toml << 'EOF'
[network]
bootstrap_nodes = [
    "$SERVER1_IP:8080",  # Node 1 (Nuremberg)
    "$SERVER2_IP:8080"   # Node 2 (Helsinki)
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

# Create Docker Compose configuration
cat > mainnet/docker-compose.yml << 'EOF'
version: '3.8'

services:
  ippan-node1:
    build:
      context: .
      dockerfile: Dockerfile.optimized
    container_name: ippan-node1
    restart: unless-stopped
    ports:
      - "8080:8080"  # P2P network port
      - "3000:3000"  # API port
      - "80:80"      # HTTP frontend
      - "443:443"    # HTTPS frontend
    volumes:
      - ippan_data:/data
      - ippan_keys:/keys
      - ippan_logs:/logs
      - ./config.toml:/config/config.toml:ro
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
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:80/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 60s

  prometheus-node1:
    image: prom/prometheus:latest
    container_name: ippan-prometheus-node1
    restart: unless-stopped
    ports:
      - "9090:9090"
    volumes:
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--storage.tsdb.retention.time=30d'
      - '--web.enable-lifecycle'
    networks:
      - ippan_network
    depends_on:
      - ippan-node1

  grafana-node1:
    image: grafana/grafana:latest
    container_name: ippan-grafana-node1
    restart: unless-stopped
    ports:
      - "3001:3000"
    volumes:
      - grafana_data:/var/lib/grafana
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin123
      - GF_USERS_ALLOW_SIGN_UP=false
      - GF_SECURITY_DISABLE_GRAVATAR=true
      - GF_ANALYTICS_REPORTING_ENABLED=false
      - GF_ANALYTICS_CHECK_FOR_UPDATES=false
    networks:
      - ippan_network
    depends_on:
      - prometheus-node1

volumes:
  ippan_data:
    driver: local
  ippan_keys:
    driver: local
  ippan_logs:
    driver: local
  prometheus_data:
    driver: local
  grafana_data:
    driver: local

networks:
  ippan_network:
    driver: bridge
EOF

# Build and start services
cd mainnet
docker-compose build --no-cache
docker-compose up -d

# Check status
docker-compose ps
docker logs ippan-node1 --tail 50
"@

Write-Host $deployCommands -ForegroundColor Cyan
Write-Host ""

Write-Host "=== Step 3: Verify Deployment ===" -ForegroundColor Blue
Write-Host "After deployment, check these URLs:" -ForegroundColor White
Write-Host "API: http://$SERVER1_IP`:3000/health" -ForegroundColor Yellow
Write-Host "Grafana: http://$SERVER1_IP`:3001" -ForegroundColor Yellow
Write-Host "Prometheus: http://$SERVER1_IP`:9090" -ForegroundColor Yellow
Write-Host ""

Write-Host "=== Step 4: Next Steps ===" -ForegroundColor Blue
Write-Host "1. Verify Server 1 is running properly" -ForegroundColor White
Write-Host "2. Deploy Server 2 with similar configuration" -ForegroundColor White
Write-Host "3. Test peer-to-peer connection between servers" -ForegroundColor White
Write-Host ""

# Save commands to file
$deployCommands | Out-File -FilePath "server1_deploy_commands.txt" -Encoding UTF8
Write-Host "Deployment commands saved to: server1_deploy_commands.txt" -ForegroundColor Green
