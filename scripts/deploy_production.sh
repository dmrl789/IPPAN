#!/bin/bash
# Production deployment script for IPPAN GBDT system
# This script handles the complete deployment of the production-ready GBDT system

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CONFIG_DIR="$PROJECT_ROOT/config"
DEPLOY_DIR="$PROJECT_ROOT/deploy"
LOG_DIR="$PROJECT_ROOT/logs"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Error handling
handle_error() {
    log_error "Deployment failed at line $1"
    cleanup
    exit 1
}

trap 'handle_error $LINENO' ERR

# Cleanup function
cleanup() {
    log_info "Cleaning up..."
    # Add cleanup logic here
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        log_error "Rust/Cargo is not installed. Please install Rust first."
        exit 1
    fi
    
    # Check if Docker is installed (for containerized deployment)
    if ! command -v docker &> /dev/null; then
        log_warning "Docker is not installed. Containerized deployment will be skipped."
    fi
    
    # Check if systemd is available (for service deployment)
    if ! command -v systemctl &> /dev/null; then
        log_warning "systemd is not available. Service deployment will be skipped."
    fi
    
    # Check available memory
    local available_memory=$(free -m | awk 'NR==2{printf "%.0f", $7}')
    if [ "$available_memory" -lt 2048 ]; then
        log_warning "Available memory is less than 2GB. Performance may be affected."
    fi
    
    log_success "Prerequisites check completed"
}

# Build the project
build_project() {
    log_info "Building the project..."
    
    cd "$PROJECT_ROOT"
    
    # Clean previous build
    cargo clean
    
    # Build in release mode
    cargo build --release --bin ippan-gbdt
    
    # Run tests
    log_info "Running tests..."
    cargo test --release
    
    # Run benchmarks
    log_info "Running benchmarks..."
    cargo bench --release
    
    log_success "Build completed successfully"
}

# Generate configuration
generate_config() {
    log_info "Generating production configuration..."
    
    # Create config directory if it doesn't exist
    mkdir -p "$CONFIG_DIR"
    
    # Generate production config
    cat > "$CONFIG_DIR/production.toml" << 'EOF'
[environment]
type = "Production"

[application]
name = "ippan-gbdt"
version = "1.0.0"
instance_id = "ippan-gbdt-001"

[gbdt]
enable_model_caching = true
cache_ttl_seconds = 3600
max_cache_size_bytes = 1073741824  # 1GB
enable_evaluation_batching = true
evaluation_batch_size = 100
enable_parallel_evaluation = true
max_parallel_evaluations = 4

[model_manager]
enable_model_management = true
model_cache_size = 10
model_cache_ttl_seconds = 3600
enable_model_validation = true
enable_model_versioning = true

[feature_engineering]
enable_feature_engineering = true
enable_feature_normalization = true
enable_feature_scaling = true
enable_outlier_detection = true
enable_feature_selection = true

[monitoring]
enable_performance_monitoring = true
enable_health_monitoring = true
enable_security_monitoring = true
metrics_interval_seconds = 30
health_check_interval_seconds = 60

[security]
enable_input_validation = true
enable_integrity_checking = true
enable_rate_limiting = true
max_requests_per_minute = 10000
enable_threat_detection = true

[resources]
max_memory_bytes = 8589934592  # 8GB
max_cpu_percent = 90.0
max_disk_bytes = 10737418240  # 10GB
max_network_bandwidth_bps = 104857600  # 100MB/s
max_file_descriptors = 1024
max_threads = 32

[feature_flags]
enable_new_gbdt_features = true
enable_experimental_features = false
enable_debug_mode = false
enable_performance_profiling = true
enable_detailed_logging = false
enable_model_versioning = true
enable_automatic_model_updates = false

[deployment]
region = "us-west-2"
availability_zone = "us-west-2a"
cluster_name = "ippan-cluster"
node_role = "worker"
enable_auto_scaling = true
min_instances = 1
max_instances = 10
health_check_interval_seconds = 30
graceful_shutdown_timeout_seconds = 30

[logging]
log_level = "info"
log_format = "json"
enable_structured_logging = true
log_file_path = "/var/log/ippan-gbdt.log"
enable_log_rotation = true
max_log_file_size_mb = 100
max_log_files = 10
enable_remote_logging = true
remote_logging_endpoint = "https://logs.ippan.network/api/v1/logs"
EOF

    log_success "Configuration generated"
}

# Deploy as systemd service
deploy_systemd_service() {
    log_info "Deploying as systemd service..."
    
    # Create systemd service file
    sudo tee /etc/systemd/system/ippan-gbdt.service > /dev/null << EOF
[Unit]
Description=IPPAN GBDT Production Service
After=network.target

[Service]
Type=simple
User=ippan
Group=ippan
WorkingDirectory=$PROJECT_ROOT
ExecStart=$PROJECT_ROOT/target/release/ippan-gbdt --config $CONFIG_DIR/production.toml
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=ippan-gbdt

# Resource limits
LimitNOFILE=65536
LimitNPROC=32768
MemoryMax=8G
CPUQuota=90%

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$PROJECT_ROOT/logs
ReadWritePaths=$PROJECT_ROOT/data

[Install]
WantedBy=multi-user.target
EOF

    # Create ippan user if it doesn't exist
    if ! id "ippan" &>/dev/null; then
        sudo useradd -r -s /bin/false ippan
    fi
    
    # Set permissions
    sudo chown -R ippan:ippan "$PROJECT_ROOT"
    sudo chmod +x "$PROJECT_ROOT/target/release/ippan-gbdt"
    
    # Reload systemd and enable service
    sudo systemctl daemon-reload
    sudo systemctl enable ippan-gbdt
    
    log_success "Systemd service deployed"
}

# Deploy with Docker
deploy_docker() {
    log_info "Deploying with Docker..."
    
    # Create Dockerfile for production
    cat > "$PROJECT_ROOT/Dockerfile.production" << 'EOF'
FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .
RUN cargo build --release --bin ippan-gbdt

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/ippan-gbdt /usr/local/bin/
COPY --from=builder /app/config /app/config

RUN useradd -r -s /bin/false ippan
USER ippan

EXPOSE 8080
CMD ["ippan-gbdt", "--config", "/app/config/production.toml"]
EOF

    # Build Docker image
    docker build -f "$PROJECT_ROOT/Dockerfile.production" -t ippan-gbdt:latest "$PROJECT_ROOT"
    
    # Create docker-compose file
    cat > "$PROJECT_ROOT/docker-compose.production.yml" << 'EOF'
version: '3.8'

services:
  ippan-gbdt:
    image: ippan-gbdt:latest
    container_name: ippan-gbdt
    restart: unless-stopped
    ports:
      - "8080:8080"
    volumes:
      - ./logs:/app/logs
      - ./data:/app/data
      - ./config:/app/config:ro
    environment:
      - RUST_LOG=info
      - RUST_BACKTRACE=1
    deploy:
      resources:
        limits:
          memory: 8G
          cpus: '4.0'
        reservations:
          memory: 2G
          cpus: '1.0'
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
EOF

    log_success "Docker deployment prepared"
}

# Deploy with Kubernetes
deploy_kubernetes() {
    log_info "Deploying with Kubernetes..."
    
    # Create Kubernetes manifests
    mkdir -p "$DEPLOY_DIR/kubernetes"
    
    # Create namespace
    cat > "$DEPLOY_DIR/kubernetes/namespace.yaml" << 'EOF'
apiVersion: v1
kind: Namespace
metadata:
  name: ippan-gbdt
EOF

    # Create ConfigMap
    cat > "$DEPLOY_DIR/kubernetes/configmap.yaml" << 'EOF'
apiVersion: v1
kind: ConfigMap
metadata:
  name: ippan-gbdt-config
  namespace: ippan-gbdt
data:
  production.toml: |
    [environment]
    type = "Production"
    
    [gbdt]
    enable_model_caching = true
    cache_ttl_seconds = 3600
    max_cache_size_bytes = 1073741824
    
    [monitoring]
    enable_performance_monitoring = true
    enable_health_monitoring = true
    
    [security]
    enable_input_validation = true
    enable_rate_limiting = true
    max_requests_per_minute = 10000
EOF

    # Create Deployment
    cat > "$DEPLOY_DIR/kubernetes/deployment.yaml" << 'EOF'
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ippan-gbdt
  namespace: ippan-gbdt
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ippan-gbdt
  template:
    metadata:
      labels:
        app: ippan-gbdt
    spec:
      containers:
      - name: ippan-gbdt
        image: ippan-gbdt:latest
        ports:
        - containerPort: 8080
        resources:
          requests:
            memory: "2Gi"
            cpu: "1"
          limits:
            memory: "8Gi"
            cpu: "4"
        volumeMounts:
        - name: config
          mountPath: /app/config
        - name: logs
          mountPath: /app/logs
        - name: data
          mountPath: /app/data
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: ippan-gbdt-config
      - name: logs
        emptyDir: {}
      - name: data
        emptyDir: {}
EOF

    # Create Service
    cat > "$DEPLOY_DIR/kubernetes/service.yaml" << 'EOF'
apiVersion: v1
kind: Service
metadata:
  name: ippan-gbdt-service
  namespace: ippan-gbdt
spec:
  selector:
    app: ippan-gbdt
  ports:
  - port: 80
    targetPort: 8080
  type: LoadBalancer
EOF

    log_success "Kubernetes manifests created"
}

# Run health checks
run_health_checks() {
    log_info "Running health checks..."
    
    # Wait for service to start
    sleep 10
    
    # Check if service is running
    if systemctl is-active --quiet ippan-gbdt; then
        log_success "Service is running"
    else
        log_error "Service is not running"
        return 1
    fi
    
    # Check health endpoint
    if command -v curl &> /dev/null; then
        if curl -f http://localhost:8080/health > /dev/null 2>&1; then
            log_success "Health endpoint is responding"
        else
            log_warning "Health endpoint is not responding"
        fi
    fi
    
    # Check logs for errors
    if journalctl -u ippan-gbdt --since "5 minutes ago" | grep -i error > /dev/null; then
        log_warning "Errors found in logs"
    else
        log_success "No errors in recent logs"
    fi
    
    log_success "Health checks completed"
}

# Main deployment function
main() {
    log_info "Starting IPPAN GBDT production deployment..."
    
    # Parse command line arguments
    DEPLOYMENT_TYPE="systemd"
    while [[ $# -gt 0 ]]; do
        case $1 in
            --type)
                DEPLOYMENT_TYPE="$2"
                shift 2
                ;;
            --help)
                echo "Usage: $0 [--type systemd|docker|kubernetes]"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    # Run deployment steps
    check_prerequisites
    build_project
    generate_config
    
    case $DEPLOYMENT_TYPE in
        systemd)
            deploy_systemd_service
            ;;
        docker)
            deploy_docker
            ;;
        kubernetes)
            deploy_kubernetes
            ;;
        *)
            log_error "Unknown deployment type: $DEPLOYMENT_TYPE"
            exit 1
            ;;
    esac
    
    run_health_checks
    
    log_success "Deployment completed successfully!"
    log_info "Deployment type: $DEPLOYMENT_TYPE"
    log_info "Configuration: $CONFIG_DIR/production.toml"
    log_info "Logs: $LOG_DIR"
}

# Run main function
main "$@"