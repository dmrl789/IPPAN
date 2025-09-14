#!/bin/bash
set -e

# IPPAN Server 1 Deployment Script
# This script deploys IPPAN on server1 (Nuremberg)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Server configuration
SERVER1_IP="188.245.97.41"    # Nuremberg (Node 1)
SERVER2_IP="135.181.145.174"  # Helsinki (Node 2)
IPPAN_USER="ippan"
IPPAN_REPO="https://github.com/dmrl789/IPPAN.git"
IPPAN_DIR="/opt/ippan/mainnet"

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo -e "${BLUE}[HEADER]${NC} $1"
}

# Function to execute command on server1
execute_on_server1() {
    local command="$1"
    local description="$2"
    
    print_status "Executing on Server 1: $description"
    
    if ssh -o ConnectTimeout=30 -o StrictHostKeyChecking=no $IPPAN_USER@$SERVER1_IP "$command" 2>/dev/null; then
        print_status "✅ $description completed successfully"
        return 0
    else
        print_error "❌ $description failed"
        return 1
    fi
}

# Function to copy file to server1
copy_to_server1() {
    local local_file="$1"
    local remote_path="$2"
    local description="$3"
    
    print_status "Copying to Server 1: $description"
    
    if scp -o ConnectTimeout=30 -o StrictHostKeyChecking=no "$local_file" $IPPAN_USER@$SERVER1_IP:"$remote_path" 2>/dev/null; then
        print_status "✅ $description completed successfully"
        return 0
    else
        print_error "❌ $description failed"
        return 1
    fi
}

# Function to check if server1 is accessible
check_server1_access() {
    print_header "🔍 Checking Server 1 Access"
    
    if ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no $IPPAN_USER@$SERVER1_IP "echo 'Server 1 is accessible'" 2>/dev/null; then
        print_status "✅ Server 1 is accessible via SSH"
        return 0
    else
        print_error "❌ Cannot access Server 1 via SSH"
        print_error "Please ensure:"
        print_error "1. Server 1 is running and accessible"
        print_error "2. SSH key is properly configured"
        print_error "3. IPPAN user exists on Server 1"
        return 1
    fi
}

# Function to setup IPPAN repository on server1
setup_ippan_repository() {
    print_header "📦 Setting up IPPAN Repository on Server 1"
    
    # Clone repository
    execute_on_server1 "cd /opt/ippan && git clone $IPPAN_REPO ippan-repo" "Cloning IPPAN repository"
    
    # Copy repository contents to mainnet directory
    execute_on_server1 "cp -r /opt/ippan/ippan-repo/* $IPPAN_DIR/" "Copying IPPAN files to mainnet directory"
    
    # Set proper permissions
    execute_on_server1 "chown -R $IPPAN_USER:$IPPAN_USER $IPPAN_DIR" "Setting file permissions"
    execute_on_server1 "chmod -R 755 $IPPAN_DIR" "Setting directory permissions"
    
    # Clean up
    execute_on_server1 "rm -rf /opt/ippan/ippan-repo" "Cleaning up temporary files"
}

# Function to configure server1 for multi-node setup
configure_multi_node() {
    print_header "⚙️  Configuring Server 1 for Multi-Node Setup"
    
    # Create node1 configuration
    local node1_config="[network]
bootstrap_nodes = [
    \"$SERVER1_IP:8080\",  # Node 1 (Nuremberg)
    \"$SERVER2_IP:8080\"   # Node 2 (Helsinki)
]
listen_address = \"0.0.0.0:8080\"
external_address = \"$SERVER1_IP:8080\"

[api]
listen_address = \"0.0.0.0:3000\"
cors_origins = [\"*\"]

[metrics]
listen_address = \"0.0.0.0:9090\"

[logging]
level = \"info\"
format = \"json\"

[consensus]
algorithm = \"proof_of_stake\"
block_time = 10
max_transactions_per_block = 1000

[storage]
data_dir = \"/opt/ippan/data\"
wal_dir = \"/opt/ippan/wal\""

    # Write configuration to server1
    execute_on_server1 "cat > $IPPAN_DIR/config.toml << 'EOF'
$node1_config
EOF" "Creating node1 configuration file"
    
    # Create environment file
    local env_content="RUST_LOG=info
IPPAN_NETWORK_PORT=8080
IPPAN_API_PORT=3000
IPPAN_STORAGE_DIR=/opt/ippan/data
IPPAN_KEYS_DIR=/opt/ippan/keys
IPPAN_LOG_DIR=/opt/ippan/logs
NODE_ENV=production
RUST_BACKTRACE=1
IPPAN_NODE_ID=node1
IPPAN_BOOTSTRAP_NODES=$SERVER1_IP:8080,$SERVER2_IP:8080"
    
    execute_on_server1 "cat > $IPPAN_DIR/.env << 'EOF'
$env_content
EOF" "Creating environment file"
}

# Function to create Docker Compose configuration for server1
create_docker_compose() {
    print_header "🐳 Creating Docker Compose Configuration for Server 1"
    
    # Create a simplified docker-compose for server1
    local docker_compose="version: '3.8'

services:
  # IPPAN Node 1
  ippan-node1:
    build:
      context: .
      dockerfile: Dockerfile.optimized
    container_name: ippan-node1
    restart: unless-stopped
    ports:
      - \"8080:8080\"  # P2P network port
      - \"3000:3000\"  # API port
      - \"80:80\"      # HTTP frontend
      - \"443:443\"    # HTTPS frontend
    volumes:
      - ippan_data:/data
      - ippan_keys:/keys
      - ippan_logs:/logs
      - ./config.toml:/config/config.toml:ro
      - ./ssl:/ssl:ro
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
      test: [\"CMD\", \"curl\", \"-f\", \"http://localhost:80/health\"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 60s
    deploy:
      resources:
        limits:
          memory: 4G
          cpus: '2.0'
        reservations:
          memory: 2G
          cpus: '1.0'

  # Prometheus Monitoring for Node 1
  prometheus-node1:
    image: prom/prometheus:latest
    container_name: ippan-prometheus-node1
    restart: unless-stopped
    ports:
      - \"9090:9090\"
    volumes:
      - prometheus_data:/prometheus
      - ./deployments/monitoring/prometheus.yml:/etc/prometheus/prometheus.yml:ro
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--storage.tsdb.retention.time=30d'
      - '--web.enable-lifecycle'
    networks:
      - ippan_network
    depends_on:
      - ippan-node1

  # Grafana Dashboard for Node 1
  grafana-node1:
    image: grafana/grafana:latest
    container_name: ippan-grafana-node1
    restart: unless-stopped
    ports:
      - \"3001:3000\"
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
    ipam:
      config:
        - subnet: 172.20.0.0/16"

    # Write docker-compose to server1
    execute_on_server1 "cat > $IPPAN_DIR/docker-compose.yml << 'EOF'
$docker_compose
EOF" "Creating Docker Compose configuration"
}

# Function to setup monitoring
setup_monitoring() {
    print_header "📊 Setting up Monitoring Dashboard"
    
    # Create basic Prometheus configuration
    local prometheus_config="global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  # - \"first_rules.yml\"
  # - \"second_rules.yml\"

scrape_configs:
  - job_name: 'ippan-node1'
    static_configs:
      - targets: ['ippan-node1:9090']
    scrape_interval: 5s
    metrics_path: /metrics

  - job_name: 'ippan-node2'
    static_configs:
      - targets: ['$SERVER2_IP:9090']
    scrape_interval: 5s
    metrics_path: /metrics"

    # Create monitoring directory and config
    execute_on_server1 "mkdir -p $IPPAN_DIR/deployments/monitoring" "Creating monitoring directory"
    execute_on_server1 "cat > $IPPAN_DIR/deployments/monitoring/prometheus.yml << 'EOF'
$prometheus_config
EOF" "Creating Prometheus configuration"
}

# Function to deploy IPPAN services on server1
deploy_ippan_services() {
    print_header "🚀 Deploying IPPAN Services on Server 1"
    
    # Build and start services
    execute_on_server1 "cd $IPPAN_DIR && docker-compose build --no-cache" "Building IPPAN services"
    
    # Start services
    execute_on_server1 "cd $IPPAN_DIR && docker-compose up -d" "Starting IPPAN services"
    
    # Wait for services to start
    print_status "Waiting for services to start..."
    sleep 30
    
    # Check service status
    execute_on_server1 "cd $IPPAN_DIR && docker-compose ps" "Checking service status"
}

# Function to verify server1 deployment
verify_deployment() {
    print_header "✅ Verifying Server 1 Deployment"
    
    # Check if services are running
    execute_on_server1 "docker ps --filter 'name=ippan' --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'" "Checking IPPAN containers"
    
    # Test API endpoint
    print_status "Testing API endpoint..."
    if curl -s --connect-timeout 10 "http://$SERVER1_IP:3000/health" >/dev/null 2>&1; then
        print_status "✅ Server 1 API is responding"
    else
        print_warning "⚠️  Server 1 API is not responding yet (may need more time to start)"
    fi
    
    # Test P2P port
    print_status "Testing P2P connectivity..."
    if timeout 10 bash -c "</dev/tcp/$SERVER1_IP/8080" 2>/dev/null; then
        print_status "✅ Server 1 P2P port is open"
    else
        print_warning "⚠️  Server 1 P2P port is not accessible"
    fi
}

# Main deployment function
main() {
    print_header "🚀 IPPAN Server 1 Deployment Script"
    echo "Server 1 (Nuremberg): $SERVER1_IP"
    echo "Server 2 (Helsinki): $SERVER2_IP"
    echo "================================================"
    
    # Check server1 access
    if ! check_server1_access; then
        print_error "Cannot proceed without access to Server 1"
        exit 1
    fi
    
    # Setup IPPAN repository
    setup_ippan_repository
    
    # Configure multi-node setup
    configure_multi_node
    
    # Create Docker Compose configuration
    create_docker_compose
    
    # Setup monitoring
    setup_monitoring
    
    # Deploy services
    deploy_ippan_services
    
    # Verify deployment
    verify_deployment
    
    print_header "🎉 Server 1 Deployment Complete!"
    print_status "Server 1 is now configured and ready for multi-node setup"
    print_status ""
    print_status "Access URLs:"
    print_status "  Server 1 API: http://$SERVER1_IP:3000"
    print_status "  Server 1 Grafana: http://$SERVER1_IP:3001"
    print_status "  Server 1 Prometheus: http://$SERVER1_IP:9090"
    print_status ""
    print_status "Next steps:"
    print_status "1. Run the Server 2 deployment script"
    print_status "2. Run the verification script to test the multi-node setup"
    print_status "3. Monitor the logs: docker-compose logs -f"
}

# Check prerequisites
if ! command_exists ssh; then
    print_error "ssh is required but not installed"
    exit 1
fi

if ! command_exists scp; then
    print_error "scp is required but not installed"
    exit 1
fi

if ! command_exists curl; then
    print_error "curl is required but not installed"
    exit 1
fi

# Run main function
main "$@"
