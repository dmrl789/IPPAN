#!/bin/bash

# IPPAN Mainnet Deployment Script
# Complete production mainnet deployment with external server support

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
MAINNET_DIR="deployments/mainnet"
BACKUP_DIR="backups/mainnet"
LOG_FILE="deploy-mainnet.log"
ENVIRONMENT=${1:-"production"}

# Functions
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}" | tee -a "$LOG_FILE"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}" | tee -a "$LOG_FILE"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}" | tee -a "$LOG_FILE"
    exit 1
}

info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO: $1${NC}" | tee -a "$LOG_FILE"
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites for mainnet deployment..."
    
    # Check if Docker is installed
    if ! command -v docker &> /dev/null; then
        error "Docker is not installed. Please install Docker first."
    fi
    
    # Check if Docker Compose is installed
    if ! command -v docker-compose &> /dev/null; then
        error "Docker Compose is not installed. Please install Docker Compose first."
    fi
    
    # Check if mainnet directory exists
    if [ ! -d "$MAINNET_DIR" ]; then
        error "Mainnet directory $MAINNET_DIR does not exist."
    fi
    
    # Check if IPPAN image exists
    if ! docker images | grep -q "ippan:latest"; then
        warn "IPPAN Docker image not found. Building image..."
        build_ippan_image
    fi
    
    # Check system resources
    check_system_resources
    
    log "Prerequisites check passed"
}

# Check system resources
check_system_resources() {
    log "Checking system resources..."
    
    # Check available memory
    AVAILABLE_MEMORY=$(free -m | awk 'NR==2{printf "%.0f", $7}')
    REQUIRED_MEMORY=32000  # 32GB
    
    if [ "$AVAILABLE_MEMORY" -lt "$REQUIRED_MEMORY" ]; then
        warn "Available memory ($AVAILABLE_MEMORY MB) is less than recommended ($REQUIRED_MEMORY MB)"
    fi
    
    # Check available disk space
    AVAILABLE_DISK=$(df -BG . | awk 'NR==2{print $4}' | sed 's/G//')
    REQUIRED_DISK=500  # 500GB
    
    if [ "$AVAILABLE_DISK" -lt "$REQUIRED_DISK" ]; then
        warn "Available disk space ($AVAILABLE_DISK GB) is less than recommended ($REQUIRED_DISK GB)"
    fi
    
    # Check CPU cores
    CPU_CORES=$(nproc)
    REQUIRED_CORES=16
    
    if [ "$CPU_CORES" -lt "$REQUIRED_CORES" ]; then
        warn "CPU cores ($CPU_CORES) is less than recommended ($REQUIRED_CORES)"
    fi
    
    log "System resources check completed"
}

# Build IPPAN Docker image
build_ippan_image() {
    log "Building IPPAN Docker image..."
    
    # Build the image
    docker build -f Dockerfile.production -t ippan:latest .
    
    if [ $? -eq 0 ]; then
        log "IPPAN Docker image built successfully"
    else
        error "Failed to build IPPAN Docker image"
    fi
}

# Create backup directory
create_backup_dir() {
    log "Creating backup directory..."
    mkdir -p "$BACKUP_DIR"
    log "Backup directory created: $BACKUP_DIR"
}

# Backup existing deployment
backup_existing_deployment() {
    log "Backing up existing deployment..."
    
    if [ -d "$MAINNET_DIR" ]; then
        cp -r "$MAINNET_DIR" "$BACKUP_DIR/mainnet-$(date +%Y%m%d-%H%M%S)"
        log "Existing deployment backed up"
    fi
}

# Generate configuration files
generate_config_files() {
    log "Generating mainnet configuration files..."
    
    # Create configs directory
    mkdir -p "$MAINNET_DIR/configs"
    
    # Generate all node configurations
    generate_node_configs
    
    # Generate SSL certificates
    generate_ssl_certificates
    
    # Generate database initialization
    generate_database_init
    
    # Generate monitoring configuration
    generate_monitoring_config
    
    log "Configuration files generated"
}

# Generate node configurations
generate_node_configs() {
    log "Generating node configurations..."
    
    # Generate bootstrap node configs
    for i in {1..3}; do
        generate_bootstrap_config $i
    done
    
    # Generate validator node configs
    for i in {1..5}; do
        generate_validator_config $i
    done
    
    log "Node configurations generated"
}

# Generate bootstrap node configuration
generate_bootstrap_config() {
    local node_num=$1
    local config_file="$MAINNET_DIR/configs/bootstrap-$node_num.toml"
    
    cat > "$config_file" << EOF
# IPPAN Mainnet Bootstrap Node $node_num Configuration

[network]
network_id = "ippan-mainnet"
chain_id = "ippan-1"
node_id = "bootstrap-node-$node_num"
is_bootstrap_node = true
listen_address = "0.0.0.0:30333"
api_address = "0.0.0.0:8080"
p2p_address = "0.0.0.0:30333"
external_address = "bootstrap$node_num.ippan.net:30333"

[consensus]
consensus_type = "bft"
block_time = 5
max_block_size = 1048576
max_transactions_per_block = 1000
finality_threshold = 0.67
validator_set_size = 25
min_validator_stake = 1000000
max_validator_stake = 10000000

[security]
enable_tls = true
cert_path = "/etc/ippan/certs/bootstrap$node_num.crt"
key_path = "/etc/ippan/certs/bootstrap$node_num.key"
ca_cert_path = "/etc/ippan/certs/ca.crt"
enable_encryption = true
encryption_key = "mainnet-encryption-key-bootstrap$node_num"
rate_limit = 10000
max_connections = 1000

[storage]
data_dir = "/var/lib/ippan"
max_storage_size = "2TB"
enable_compression = true
enable_deduplication = true
shard_count = 16
replication_factor = 3
backup_interval = 3600
backup_retention = 7

[database]
database_url = "postgresql://ippan:secure-postgres-password@postgres:5432/ippan_mainnet"
max_connections = 100
connection_timeout = 30
query_timeout = 60
enable_wal = true
enable_vacuum = true
vacuum_interval = 86400

[api]
enable_api = true
api_port = 8080
api_host = "0.0.0.0"
enable_cors = true
cors_origins = ["https://ippan.net", "https://wallet.ippan.net", "https://explorer.ippan.net"]
enable_auth = true
auth_type = "jwt"
jwt_secret = "mainnet-jwt-secret-bootstrap$node_num"
rate_limit = 1000
rate_limit_window = 3600

[monitoring]
enable_metrics = true
metrics_port = 9090
metrics_path = "/metrics"
enable_health_checks = true
health_check_interval = 30
enable_tracing = true
tracing_endpoint = "http://jaeger:14268/api/traces"

[logging]
log_level = "info"
log_format = "json"
log_file = "/var/log/ippan/ippan.log"
max_log_size = "100MB"
max_log_files = 10
enable_console = true
enable_file = true
enable_json = true

[performance]
enable_optimization = true
max_concurrent_requests = 1000
request_timeout = 30
enable_caching = true
cache_size = "1GB"
cache_ttl = 3600
enable_compression = true
compression_level = 6

[backup]
enable_backup = true
backup_schedule = "0 2 * * *"
backup_retention = 30
backup_encryption = true
backup_compression = true
backup_destination = "s3://ippan-backups/mainnet/"

[maintenance]
enable_maintenance = true
maintenance_window = "02:00-04:00"
maintenance_timezone = "UTC"
auto_update = false
update_check_interval = 86400
EOF
}

# Generate validator node configuration
generate_validator_config() {
    local node_num=$1
    local config_file="$MAINNET_DIR/configs/validator-$node_num.toml"
    
    cat > "$config_file" << EOF
# IPPAN Mainnet Validator Node $node_num Configuration

[network]
network_id = "ippan-mainnet"
chain_id = "ippan-1"
node_id = "validator-node-$node_num"
is_bootstrap_node = false
listen_address = "0.0.0.0:30333"
api_address = "0.0.0.0:8080"
p2p_address = "0.0.0.0:30333"
external_address = "validator$node_num.ippan.net:30333"

[bootstrap_nodes]
nodes = [
    "bootstrap1.ippan.net:30333",
    "bootstrap2.ippan.net:30333",
    "bootstrap3.ippan.net:30333"
]

[consensus]
consensus_type = "bft"
block_time = 5
max_block_size = 1048576
max_transactions_per_block = 1000
finality_threshold = 0.67
validator_set_size = 25
min_validator_stake = 1000000
max_validator_stake = 10000000

[staking]
validator_address = "0x$(openssl rand -hex 20)"
stake_amount = 5000000
commission_rate = 0.05
delegation_enabled = true
min_delegation = 1000
max_delegations = 1000

[security]
enable_tls = true
cert_path = "/etc/ippan/certs/validator$node_num.crt"
key_path = "/etc/ippan/certs/validator$node_num.key"
ca_cert_path = "/etc/ippan/certs/ca.crt"
enable_encryption = true
encryption_key = "mainnet-encryption-key-validator$node_num"
rate_limit = 10000
max_connections = 1000

[storage]
data_dir = "/var/lib/ippan"
max_storage_size = "2TB"
enable_compression = true
enable_deduplication = true
shard_count = 16
replication_factor = 3
backup_interval = 3600
backup_retention = 7

[database]
database_url = "postgresql://ippan:secure-postgres-password@postgres:5432/ippan_mainnet"
max_connections = 100
connection_timeout = 30
query_timeout = 60
enable_wal = true
enable_vacuum = true
vacuum_interval = 86400

[api]
enable_api = true
api_port = 8080
api_host = "0.0.0.0"
enable_cors = true
cors_origins = ["https://ippan.net", "https://wallet.ippan.net", "https://explorer.ippan.net"]
enable_auth = true
auth_type = "jwt"
jwt_secret = "mainnet-jwt-secret-validator$node_num"
rate_limit = 1000
rate_limit_window = 3600

[monitoring]
enable_metrics = true
metrics_port = 9090
metrics_path = "/metrics"
enable_health_checks = true
health_check_interval = 30
enable_tracing = true
tracing_endpoint = "http://jaeger:14268/api/traces"

[logging]
log_level = "info"
log_format = "json"
log_file = "/var/log/ippan/ippan.log"
max_log_size = "100MB"
max_log_files = 10
enable_console = true
enable_file = true
enable_json = true

[performance]
enable_optimization = true
max_concurrent_requests = 1000
request_timeout = 30
enable_caching = true
cache_size = "1GB"
cache_ttl = 3600
enable_compression = true
compression_level = 6

[backup]
enable_backup = true
backup_schedule = "0 2 * * *"
backup_retention = 30
backup_encryption = true
backup_compression = true
backup_destination = "s3://ippan-backups/mainnet/"

[maintenance]
enable_maintenance = true
maintenance_window = "02:00-04:00"
maintenance_timezone = "UTC"
auto_update = false
update_check_interval = 86400

[mining]
enable_mining = true
mining_threads = 4
mining_algorithm = "bft"
mining_reward = 100
mining_fee = 10
mining_pool = false
mining_solo = true
EOF
}

# Generate SSL certificates
generate_ssl_certificates() {
    log "Generating SSL certificates..."
    
    # Create certs directory
    mkdir -p "$MAINNET_DIR/certs"
    mkdir -p "$MAINNET_DIR/nginx/ssl"
    
    # Generate CA certificate
    openssl genrsa -out "$MAINNET_DIR/certs/ca.key" 4096
    openssl req -new -x509 -days 365 -key "$MAINNET_DIR/certs/ca.key" -out "$MAINNET_DIR/certs/ca.crt" -subj "/C=US/ST=CA/L=San Francisco/O=IPPAN/OU=IT/CN=IPPAN CA"
    
    # Generate certificates for each node
    for i in {1..3}; do
        generate_node_certificate "bootstrap$i" "$MAINNET_DIR/certs"
    done
    
    for i in {1..5}; do
        generate_node_certificate "validator$i" "$MAINNET_DIR/certs"
    done
    
    # Generate web certificates
    generate_web_certificates
    
    log "SSL certificates generated"
}

# Generate node certificate
generate_node_certificate() {
    local node_name=$1
    local cert_dir=$2
    
    # Generate private key
    openssl genrsa -out "$cert_dir/$node_name.key" 4096
    
    # Generate certificate signing request
    openssl req -new -key "$cert_dir/$node_name.key" -out "$cert_dir/$node_name.csr" -subj "/C=US/ST=CA/L=San Francisco/O=IPPAN/OU=IT/CN=$node_name.ippan.net"
    
    # Generate certificate
    openssl x509 -req -days 365 -in "$cert_dir/$node_name.csr" -CA "$cert_dir/ca.crt" -CAkey "$cert_dir/ca.key" -CAcreateserial -out "$cert_dir/$node_name.crt"
    
    # Clean up CSR
    rm "$cert_dir/$node_name.csr"
}

# Generate web certificates
generate_web_certificates() {
    local cert_dir="$MAINNET_DIR/nginx/ssl"
    
    # Generate certificates for web domains
    local domains=("ippan.net" "api.ippan.net" "p2p.ippan.net" "monitor.ippan.net")
    
    for domain in "${domains[@]}"; do
        # Generate private key
        openssl genrsa -out "$cert_dir/$domain.key" 4096
        
        # Generate certificate signing request
        openssl req -new -key "$cert_dir/$domain.key" -out "$cert_dir/$domain.csr" -subj "/C=US/ST=CA/L=San Francisco/O=IPPAN/OU=IT/CN=$domain"
        
        # Generate certificate
        openssl x509 -req -days 365 -in "$cert_dir/$domain.csr" -CA "$MAINNET_DIR/certs/ca.crt" -CAkey "$MAINNET_DIR/certs/ca.key" -CAcreateserial -out "$cert_dir/$domain.crt"
        
        # Clean up CSR
        rm "$cert_dir/$domain.csr"
    done
}

# Generate database initialization
generate_database_init() {
    log "Generating database initialization..."
    
    mkdir -p "$MAINNET_DIR/postgres"
    
    cat > "$MAINNET_DIR/postgres/init.sql" << EOF
-- IPPAN Mainnet Database Initialization

-- Create database
CREATE DATABASE ippan_mainnet;

-- Create user
CREATE USER ippan WITH PASSWORD 'secure-postgres-password';

-- Grant privileges
GRANT ALL PRIVILEGES ON DATABASE ippan_mainnet TO ippan;

-- Connect to database
\c ippan_mainnet;

-- Create tables
CREATE TABLE IF NOT EXISTS blocks (
    id SERIAL PRIMARY KEY,
    hash VARCHAR(64) UNIQUE NOT NULL,
    height BIGINT NOT NULL,
    timestamp BIGINT NOT NULL,
    proposer VARCHAR(64) NOT NULL,
    transaction_count INTEGER NOT NULL,
    size_bytes BIGINT NOT NULL,
    parent_hash VARCHAR(64) NOT NULL,
    state_root VARCHAR(64) NOT NULL,
    transactions_root VARCHAR(64) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS transactions (
    id SERIAL PRIMARY KEY,
    hash VARCHAR(64) UNIQUE NOT NULL,
    from_address VARCHAR(64) NOT NULL,
    to_address VARCHAR(64) NOT NULL,
    amount BIGINT NOT NULL,
    fee BIGINT NOT NULL,
    nonce BIGINT NOT NULL,
    signature TEXT NOT NULL,
    memo TEXT,
    status VARCHAR(20) NOT NULL,
    block_height BIGINT,
    timestamp BIGINT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS accounts (
    id SERIAL PRIMARY KEY,
    address VARCHAR(64) UNIQUE NOT NULL,
    balance BIGINT NOT NULL DEFAULT 0,
    nonce BIGINT NOT NULL DEFAULT 0,
    account_type VARCHAR(20) NOT NULL DEFAULT 'standard',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS validators (
    id SERIAL PRIMARY KEY,
    address VARCHAR(64) UNIQUE NOT NULL,
    stake_amount BIGINT NOT NULL,
    commission_rate DECIMAL(5,4) NOT NULL,
    status VARCHAR(20) NOT NULL,
    voting_power BIGINT NOT NULL,
    last_activity BIGINT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_blocks_height ON blocks(height);
CREATE INDEX IF NOT EXISTS idx_blocks_timestamp ON blocks(timestamp);
CREATE INDEX IF NOT EXISTS idx_transactions_from ON transactions(from_address);
CREATE INDEX IF NOT EXISTS idx_transactions_to ON transactions(to_address);
CREATE INDEX IF NOT EXISTS idx_transactions_status ON transactions(status);
CREATE INDEX IF NOT EXISTS idx_transactions_timestamp ON transactions(timestamp);
CREATE INDEX IF NOT EXISTS idx_accounts_address ON accounts(address);
CREATE INDEX IF NOT EXISTS idx_validators_address ON validators(address);
CREATE INDEX IF NOT EXISTS idx_validators_status ON validators(status);

-- Grant table privileges
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO ippan;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO ippan;
EOF

    log "Database initialization generated"
}

# Generate monitoring configuration
generate_monitoring_config() {
    log "Generating monitoring configuration..."
    
    mkdir -p "$MAINNET_DIR/monitoring"
    
    # Copy monitoring configuration from monitoring directory
    if [ -d "deployments/monitoring" ]; then
        cp deployments/monitoring/prometheus-production.yml "$MAINNET_DIR/monitoring/prometheus.yml"
        cp deployments/monitoring/alertmanager-production.yml "$MAINNET_DIR/monitoring/alertmanager.yml"
        cp deployments/monitoring/ippan-production-rules.yml "$MAINNET_DIR/monitoring/rules.yml"
    fi
    
    log "Monitoring configuration generated"
}

# Create necessary directories
create_directories() {
    log "Creating necessary directories..."
    
    # Create data directories
    for i in {1..3}; do
        mkdir -p "$MAINNET_DIR/data/bootstrap-$i"
        mkdir -p "$MAINNET_DIR/logs/bootstrap-$i"
    done
    
    for i in {1..5}; do
        mkdir -p "$MAINNET_DIR/data/validator-$i"
        mkdir -p "$MAINNET_DIR/logs/validator-$i"
    done
    
    # Create nginx directories
    mkdir -p "$MAINNET_DIR/nginx"
    mkdir -p "$MAINNET_DIR/logs/nginx"
    
    # Create monitoring directories
    mkdir -p "$MAINNET_DIR/monitoring/grafana/dashboards"
    mkdir -p "$MAINNET_DIR/monitoring/grafana/datasources"
    
    log "Directories created"
}

# Deploy mainnet
deploy_mainnet() {
    log "Deploying IPPAN mainnet..."
    
    cd "$MAINNET_DIR"
    
    # Pull Docker images
    log "Pulling Docker images..."
    docker-compose -f docker-compose-mainnet.yml pull
    
    # Start mainnet services
    log "Starting mainnet services..."
    docker-compose -f docker-compose-mainnet.yml up -d
    
    # Wait for services to be ready
    log "Waiting for services to be ready..."
    sleep 60
    
    # Check service health
    check_service_health
    
    cd - > /dev/null
    log "Mainnet deployment completed"
}

# Check service health
check_service_health() {
    log "Checking service health..."
    
    # Check bootstrap nodes
    for i in {1..3}; do
        local port=$((8080 + i - 1))
        if curl -f "http://localhost:$port/health" > /dev/null 2>&1; then
            log "Bootstrap node $i is healthy"
        else
            warn "Bootstrap node $i health check failed"
        fi
    done
    
    # Check validator nodes
    for i in {1..5}; do
        local port=$((8082 + i))
        if curl -f "http://localhost:$port/health" > /dev/null 2>&1; then
            log "Validator node $i is healthy"
        else
            warn "Validator node $i health check failed"
        fi
    done
    
    # Check load balancer
    if curl -f "http://localhost/health" > /dev/null 2>&1; then
        log "Load balancer is healthy"
    else
        warn "Load balancer health check failed"
    fi
    
    # Check database
    if docker exec ippan-postgres pg_isready -U ippan -d ippan_mainnet > /dev/null 2>&1; then
        log "Database is healthy"
    else
        warn "Database health check failed"
    fi
    
    # Check monitoring services
    if curl -f "http://localhost:9090/-/healthy" > /dev/null 2>&1; then
        log "Prometheus is healthy"
    else
        warn "Prometheus health check failed"
    fi
    
    if curl -f "http://localhost:3000/api/health" > /dev/null 2>&1; then
        log "Grafana is healthy"
    else
        warn "Grafana health check failed"
    fi
}

# Set up monitoring
setup_monitoring() {
    log "Setting up monitoring..."
    
    # Wait for monitoring services to be ready
    sleep 30
    
    # Import Grafana dashboards
    import_grafana_dashboards
    
    # Configure alert rules
    configure_alert_rules
    
    log "Monitoring setup completed"
}

# Import Grafana dashboards
import_grafana_dashboards() {
    log "Importing Grafana dashboards..."
    
    # Create datasource configuration
    cat > "$MAINNET_DIR/monitoring/grafana/datasources/prometheus.yml" << EOF
apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
    editable: true
    jsonData:
      httpMethod: POST
      manageAlerts: true
      prometheusType: Prometheus
      prometheusVersion: 2.40.0
      cacheLevel: 'High'
      disableMetricsLookup: false
      incrementalQueryOverlapWindow: 10m
      queryTimeout: 60s
      timeInterval: 15s
EOF

    # Create dashboard provisioning configuration
    cat > "$MAINNET_DIR/monitoring/grafana/dashboards/dashboard.yml" << EOF
apiVersion: 1

providers:
  - name: 'ippan-mainnet-dashboards'
    orgId: 1
    folder: 'IPPAN Mainnet'
    type: file
    disableDeletion: false
    updateIntervalSeconds: 10
    allowUiUpdates: true
    options:
      path: /etc/grafana/provisioning/dashboards
EOF

    log "Grafana dashboards configured"
}

# Configure alert rules
configure_alert_rules() {
    log "Configuring alert rules..."
    
    # Copy alert rules to Prometheus
    if [ -f "$MAINNET_DIR/monitoring/rules.yml" ]; then
        docker cp "$MAINNET_DIR/monitoring/rules.yml" ippan-prometheus:/etc/prometheus/ippan-mainnet-rules.yml
        docker exec ippan-prometheus kill -HUP 1
        log "Alert rules configured"
    fi
}

# Set up backup
setup_backup() {
    log "Setting up backup..."
    
    # Create backup script
    cat > "scripts/backup-mainnet.sh" << 'EOF'
#!/bin/bash

# IPPAN Mainnet Backup Script

BACKUP_DIR="backups/mainnet/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"

echo "Backing up mainnet to $BACKUP_DIR..."

# Backup configuration files
cp -r deployments/mainnet/configs "$BACKUP_DIR/"
cp -r deployments/mainnet/nginx "$BACKUP_DIR/"
cp -r deployments/mainnet/certs "$BACKUP_DIR/"

# Backup database
docker exec ippan-postgres pg_dump -U ippan ippan_mainnet > "$BACKUP_DIR/database.sql"

# Backup node data
for i in {1..3}; do
    docker run --rm -v ippan_mainnet_data_bootstrap_$i:/data -v "$(pwd)/$BACKUP_DIR":/backup alpine tar czf /backup/bootstrap-$i-data.tar.gz -C /data .
done

for i in {1..5}; do
    docker run --rm -v ippan_mainnet_data_validator_$i:/data -v "$(pwd)/$BACKUP_DIR":/backup alpine tar czf /backup/validator-$i-data.tar.gz -C /data .
done

echo "Mainnet backup completed: $BACKUP_DIR"
EOF

    chmod +x "scripts/backup-mainnet.sh"
    
    log "Backup setup completed"
}

# Main deployment function
main() {
    log "Starting IPPAN Mainnet Deployment..."
    log "Environment: $ENVIRONMENT"
    
    # Check prerequisites
    check_prerequisites
    
    # Create backup directory
    create_backup_dir
    
    # Backup existing deployment
    backup_existing_deployment
    
    # Generate configuration files
    generate_config_files
    
    # Create necessary directories
    create_directories
    
    # Deploy mainnet
    deploy_mainnet
    
    # Set up monitoring
    setup_monitoring
    
    # Set up backup
    setup_backup
    
    log "IPPAN Mainnet Deployment completed successfully!"
    
    echo
    echo "=== Mainnet Information ==="
    echo "Bootstrap Nodes:"
    echo "  - bootstrap1.ippan.net:30333 (http://localhost:8080)"
    echo "  - bootstrap2.ippan.net:30333 (http://localhost:8081)"
    echo "  - bootstrap3.ippan.net:30333 (http://localhost:8082)"
    echo
    echo "Validator Nodes:"
    echo "  - validator1.ippan.net:30333 (http://localhost:8083)"
    echo "  - validator2.ippan.net:30333 (http://localhost:8084)"
    echo "  - validator3.ippan.net:30333 (http://localhost:8085)"
    echo "  - validator4.ippan.net:30333 (http://localhost:8086)"
    echo "  - validator5.ippan.net:30333 (http://localhost:8087)"
    echo
    echo "Load Balancer:"
    echo "  - API: https://api.ippan.net"
    echo "  - P2P: https://p2p.ippan.net"
    echo "  - Monitor: https://monitor.ippan.net"
    echo
    echo "Monitoring:"
    echo "  - Prometheus: http://localhost:9090"
    echo "  - Grafana: http://localhost:3000 (admin/secure-grafana-password)"
    echo "  - AlertManager: http://localhost:9093"
    echo
    echo "Database:"
    echo "  - PostgreSQL: localhost:5432"
    echo "  - Redis: localhost:6379"
    echo
    echo "=== Useful Commands ==="
    echo "Check status: docker-compose -f deployments/mainnet/docker-compose-mainnet.yml ps"
    echo "View logs: docker-compose -f deployments/mainnet/docker-compose-mainnet.yml logs -f"
    echo "Backup: ./scripts/backup-mainnet.sh"
    echo "Stop: docker-compose -f deployments/mainnet/docker-compose-mainnet.yml down"
    echo "Restart: docker-compose -f deployments/mainnet/docker-compose-mainnet.yml restart"
    echo
    echo "=== Next Steps ==="
    echo "1. Configure DNS records for all domains"
    echo "2. Set up SSL certificates with Let's Encrypt"
    echo "3. Configure external monitoring and alerting"
    echo "4. Set up automated backups to cloud storage"
    echo "5. Configure load balancing across multiple servers"
    echo "6. Set up disaster recovery procedures"
    echo "7. Train operations team on mainnet management"
    echo "8. Establish mainnet governance procedures"
}

# Run main function
main "$@"
