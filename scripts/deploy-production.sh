#!/bin/bash

# IPPAN Production Deployment Script
# This script deploys IPPAN to production with comprehensive checks and monitoring

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
IPPAN_VERSION="1.0.0"
DEPLOYMENT_ENV="production"
DOCKER_REGISTRY="ippan"
NAMESPACE="ippan"

# Logging function
log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
    exit 1
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."
    
    # Check if Docker is installed and running
    if ! command -v docker &> /dev/null; then
        error "Docker is not installed"
    fi
    
    if ! docker info &> /dev/null; then
        error "Docker is not running"
    fi
    
    # Check if kubectl is installed (for Kubernetes deployment)
    if command -v kubectl &> /dev/null; then
        KUBERNETES_AVAILABLE=true
        log "Kubernetes deployment available"
    else
        KUBERNETES_AVAILABLE=false
        warning "Kubernetes not available, using Docker Compose"
    fi
    
    # Check if required ports are available
    local ports=(8080 3000 9090 3001 9093 80 443)
    for port in "${ports[@]}"; do
        if netstat -tuln | grep -q ":$port "; then
            warning "Port $port is already in use"
        fi
    done
    
    success "Prerequisites check completed"
}

# Build IPPAN Docker image
build_image() {
    log "Building IPPAN Docker image..."
    
    # Build the image
    docker build -t $DOCKER_REGISTRY/ippan:$IPPAN_VERSION -t $DOCKER_REGISTRY/ippan:latest .
    
    if [ $? -eq 0 ]; then
        success "IPPAN Docker image built successfully"
    else
        error "Failed to build IPPAN Docker image"
    fi
}

# Run security scan
security_scan() {
    log "Running security scan..."
    
    # Run cargo audit
    if command -v cargo-audit &> /dev/null; then
        cargo audit
        if [ $? -ne 0 ]; then
            warning "Security vulnerabilities found in dependencies"
        fi
    fi
    
    # Run Docker security scan
    docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
        -v "$(pwd):/workspace" \
        aquasec/trivy image $DOCKER_REGISTRY/ippan:$IPPAN_VERSION
    
    success "Security scan completed"
}

# Deploy with Docker Compose
deploy_docker_compose() {
    log "Deploying with Docker Compose..."
    
    # Create production environment file
    cat > .env.production << EOF
# IPPAN Production Environment Variables
IPPAN_VERSION=$IPPAN_VERSION
DEPLOYMENT_ENV=$DEPLOYMENT_ENV

# Network Configuration
IPPAN_NETWORK_PORT=8080
IPPAN_API_PORT=3000

# Storage Configuration
IPPAN_STORAGE_DIR=/data
IPPAN_KEYS_DIR=/keys
IPPAN_LOG_DIR=/logs

# Security Configuration
BACKUP_ENCRYPTION_KEY=$(openssl rand -hex 32)

# Monitoring Configuration
SLACK_WEBHOOK_URL=${SLACK_WEBHOOK_URL:-}
EMAIL_SMTP_HOST=${EMAIL_SMTP_HOST:-}
EMAIL_SMTP_PORT=${EMAIL_SMTP_PORT:-}
EMAIL_USERNAME=${EMAIL_USERNAME:-}
EMAIL_PASSWORD=${EMAIL_PASSWORD:-}
EOF
    
    # Deploy with docker-compose
    docker-compose -f deployments/production/docker-compose.yml --env-file .env.production up -d
    
    if [ $? -eq 0 ]; then
        success "Docker Compose deployment completed"
    else
        error "Docker Compose deployment failed"
    fi
}

# Deploy with Kubernetes
deploy_kubernetes() {
    log "Deploying with Kubernetes..."
    
    # Create namespace
    kubectl create namespace $NAMESPACE --dry-run=client -o yaml | kubectl apply -f -
    
    # Create secrets
    kubectl create secret generic ippan-secrets \
        --namespace=$NAMESPACE \
        --from-literal=node-id=$(openssl rand -hex 32) \
        --from-literal=private-key=$(openssl genrsa 2048 | base64) \
        --dry-run=client -o yaml | kubectl apply -f -
    
    # Deploy IPPAN
    kubectl apply -f deployments/kubernetes/ippan-deployment.yaml
    
    # Deploy monitoring
    kubectl apply -f deployments/kubernetes/monitoring/
    
    # Wait for deployment
    kubectl rollout status deployment/ippan-node -n $NAMESPACE --timeout=300s
    
    if [ $? -eq 0 ]; then
        success "Kubernetes deployment completed"
    else
        error "Kubernetes deployment failed"
    fi
}

# Health check
health_check() {
    log "Performing health check..."
    
    local max_attempts=30
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        log "Health check attempt $attempt/$max_attempts"
        
        # Check if API is responding
        if curl -f http://localhost:3000/api/v1/status &> /dev/null; then
            success "IPPAN API is responding"
            
            # Check node status
            local status=$(curl -s http://localhost:3000/api/v1/status | jq -r '.status')
            if [ "$status" = "running" ]; then
                success "IPPAN node is running"
                return 0
            else
                warning "IPPAN node status: $status"
            fi
        else
            warning "IPPAN API not responding yet"
        fi
        
        sleep 10
        ((attempt++))
    done
    
    error "Health check failed after $max_attempts attempts"
}

# Performance test
performance_test() {
    log "Running performance tests..."
    
    # Test API response time
    local response_time=$(curl -w "%{time_total}" -o /dev/null -s http://localhost:3000/api/v1/status)
    log "API response time: ${response_time}s"
    
    if (( $(echo "$response_time < 1.0" | bc -l) )); then
        success "API response time is acceptable"
    else
        warning "API response time is slow: ${response_time}s"
    fi
    
    # Test storage operations
    local test_file="/tmp/ippan-test-$(date +%s).txt"
    echo "IPPAN performance test file" > "$test_file"
    
    local upload_start=$(date +%s.%N)
    curl -X POST http://localhost:3000/api/v1/storage/upload \
        -F "file=@$test_file" \
        -F "name=performance-test" &> /dev/null
    local upload_end=$(date +%s.%N)
    
    local upload_time=$(echo "$upload_end - $upload_start" | bc -l)
    log "File upload time: ${upload_time}s"
    
    rm -f "$test_file"
    
    success "Performance tests completed"
}

# Setup monitoring
setup_monitoring() {
    log "Setting up monitoring..."
    
    # Create monitoring directories
    mkdir -p monitoring/prometheus
    mkdir -p monitoring/grafana/dashboards
    mkdir -p monitoring/grafana/datasources
    mkdir -p monitoring/alerts
    
    # Copy monitoring configurations
    cp deployments/monitoring/prometheus.yml monitoring/prometheus/
    cp deployments/monitoring/alerts.yml monitoring/alerts/
    
    # Create Grafana dashboard
    cat > monitoring/grafana/dashboards/ippan-dashboard.json << 'EOF'
{
  "dashboard": {
    "title": "IPPAN Node Dashboard",
    "panels": [
      {
        "title": "Node Status",
        "type": "stat",
        "targets": [
          {
            "expr": "ippan_node_status",
            "legendFormat": "Status"
          }
        ]
      },
      {
        "title": "Connected Peers",
        "type": "graph",
        "targets": [
          {
            "expr": "ippan_network_connected_peers",
            "legendFormat": "Peers"
          }
        ]
      },
      {
        "title": "Storage Usage",
        "type": "graph",
        "targets": [
          {
            "expr": "(ippan_storage_used_bytes / ippan_storage_total_bytes) * 100",
            "legendFormat": "Usage %"
          }
        ]
      }
    ]
  }
}
EOF
    
    success "Monitoring setup completed"
}

# Backup configuration
backup_config() {
    log "Creating backup configuration..."
    
    # Create backup script
    cat > scripts/backup.sh << 'EOF'
#!/bin/bash

# IPPAN Backup Script
BACKUP_DIR="/backups/ippan-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"

# Backup data
docker exec ippan-node tar czf - /data > "$BACKUP_DIR/data.tar.gz"

# Backup keys
docker exec ippan-node tar czf - /keys > "$BACKUP_DIR/keys.tar.gz"

# Backup logs
docker exec ippan-node tar czf - /logs > "$BACKUP_DIR/logs.tar.gz"

# Backup configuration
cp -r config/ "$BACKUP_DIR/"

echo "Backup completed: $BACKUP_DIR"
EOF
    
    chmod +x scripts/backup.sh
    
    success "Backup configuration created"
}

# Main deployment function
main() {
    log "Starting IPPAN production deployment..."
    
    # Check prerequisites
    check_prerequisites
    
    # Build image
    build_image
    
    # Security scan
    security_scan
    
    # Setup monitoring
    setup_monitoring
    
    # Backup configuration
    backup_config
    
    # Deploy based on available tools
    if [ "$KUBERNETES_AVAILABLE" = true ]; then
        deploy_kubernetes
    else
        deploy_docker_compose
    fi
    
    # Health check
    health_check
    
    # Performance test
    performance_test
    
    success "IPPAN production deployment completed successfully!"
    
    # Display deployment information
    echo
    echo "=== IPPAN Production Deployment Summary ==="
    echo "Version: $IPPAN_VERSION"
    echo "Environment: $DEPLOYMENT_ENV"
    echo "API Endpoint: http://localhost:3000"
    echo "P2P Port: 8080"
    echo "Monitoring: http://localhost:3001 (Grafana)"
    echo "Metrics: http://localhost:9090 (Prometheus)"
    echo "Alerts: http://localhost:9093 (Alertmanager)"
    echo
    echo "Useful commands:"
    echo "  View logs: docker logs ippan-node"
    echo "  Check status: curl http://localhost:3000/api/v1/status"
    echo "  Backup: ./scripts/backup.sh"
    echo "  Stop: docker-compose -f deployments/production/docker-compose.yml down"
    echo
}

# Run main function
main "$@" 