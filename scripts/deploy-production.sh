#!/bin/bash

# IPPAN Production Deployment Script
# This script deploys IPPAN to production with comprehensive monitoring and security

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DEPLOYMENT_ENV="${DEPLOYMENT_ENV:-production}"
DOCKER_REGISTRY="${DOCKER_REGISTRY:-ippan}"
IMAGE_TAG="${IMAGE_TAG:-latest}"
NAMESPACE="${NAMESPACE:-ippan-production}"

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

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check if Docker is installed and running
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        exit 1
    fi
    
    if ! docker info &> /dev/null; then
        log_error "Docker is not running"
        exit 1
    fi
    
    # Check if Docker Compose is installed
    if ! command -v docker-compose &> /dev/null; then
        log_error "Docker Compose is not installed"
        exit 1
    fi
    
    # Check if kubectl is installed (for Kubernetes deployment)
    if ! command -v kubectl &> /dev/null; then
        log_warning "kubectl is not installed - Kubernetes deployment will be skipped"
    fi
    
    # Check if required environment variables are set
    if [[ -z "${BACKUP_ENCRYPTION_KEY:-}" ]]; then
        log_warning "BACKUP_ENCRYPTION_KEY is not set - using default (not recommended for production)"
        export BACKUP_ENCRYPTION_KEY="default-backup-key-change-in-production"
    fi
    
    log_success "Prerequisites check completed"
}

# Build Docker images
build_images() {
    log_info "Building Docker images..."
    
    cd "$PROJECT_ROOT"
    
    # Build the main IPPAN image
    log_info "Building IPPAN node image..."
    docker build -f Dockerfile.optimized -t "$DOCKER_REGISTRY/ippan:$IMAGE_TAG" .
    
    # Build frontend image
    log_info "Building frontend image..."
    docker build -f apps/unified-ui/Dockerfile -t "$DOCKER_REGISTRY/ippan-frontend:$IMAGE_TAG" apps/unified-ui/
    
    # Tag images for production
    docker tag "$DOCKER_REGISTRY/ippan:$IMAGE_TAG" "$DOCKER_REGISTRY/ippan:production"
    docker tag "$DOCKER_REGISTRY/ippan-frontend:$IMAGE_TAG" "$DOCKER_REGISTRY/ippan-frontend:production"
    
    log_success "Docker images built successfully"
}

# Generate SSL certificates
generate_ssl_certificates() {
    log_info "Generating SSL certificates..."
    
    SSL_DIR="$PROJECT_ROOT/ssl"
    mkdir -p "$SSL_DIR"
    
    # Generate self-signed certificate for development
    # In production, you should use Let's Encrypt or a proper CA
    if [[ ! -f "$SSL_DIR/ippan.crt" ]] || [[ ! -f "$SSL_DIR/ippan.key" ]]; then
        log_info "Generating self-signed SSL certificate..."
        openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
            -keyout "$SSL_DIR/ippan.key" \
            -out "$SSL_DIR/ippan.crt" \
            -subj "/C=US/ST=State/L=City/O=IPPAN/CN=ippan.network"
        
        chmod 600 "$SSL_DIR/ippan.key"
        chmod 644 "$SSL_DIR/ippan.crt"
    fi
    
    log_success "SSL certificates generated"
}

# Deploy with Docker Compose
deploy_docker_compose() {
    log_info "Deploying with Docker Compose..."
    
    cd "$PROJECT_ROOT"
    
    # Stop existing containers
    log_info "Stopping existing containers..."
    docker-compose -f docker-compose.production.yml down --remove-orphans || true
    
    # Start new containers
    log_info "Starting new containers..."
    docker-compose -f docker-compose.production.yml up -d
    
    # Wait for services to be healthy
    log_info "Waiting for services to be healthy..."
    sleep 30
    
    # Check service health
    if docker-compose -f docker-compose.production.yml ps | grep -q "Up (healthy)"; then
        log_success "Docker Compose deployment completed successfully"
    else
        log_error "Some services are not healthy"
        docker-compose -f docker-compose.production.yml ps
        exit 1
    fi
}

# Deploy with Kubernetes
deploy_kubernetes() {
    if ! command -v kubectl &> /dev/null; then
        log_warning "kubectl not available - skipping Kubernetes deployment"
        return
    fi
    
    log_info "Deploying with Kubernetes..."
    
    # Create namespace
    kubectl create namespace "$NAMESPACE" --dry-run=client -o yaml | kubectl apply -f -
    
    # Apply Kubernetes manifests
    kubectl apply -f "$PROJECT_ROOT/deployments/kubernetes/ippan-production.yaml"
    
    # Wait for deployment to be ready
    log_info "Waiting for deployment to be ready..."
    kubectl wait --for=condition=available --timeout=300s deployment/ippan-node -n "$NAMESPACE"
    
    # Check pod status
    kubectl get pods -n "$NAMESPACE"
    
    log_success "Kubernetes deployment completed successfully"
}

# Setup monitoring
setup_monitoring() {
    log_info "Setting up monitoring..."
    
    # Wait for monitoring services to be ready
    sleep 60
    
    # Check if Prometheus is accessible
    if curl -f http://localhost:9090/-/healthy &> /dev/null; then
        log_success "Prometheus is running"
    else
        log_warning "Prometheus is not accessible"
    fi
    
    # Check if Grafana is accessible
    if curl -f http://localhost:3001/api/health &> /dev/null; then
        log_success "Grafana is running"
        log_info "Grafana dashboard: http://localhost:3001 (admin/admin123)"
    else
        log_warning "Grafana is not accessible"
    fi
    
    # Check if Alertmanager is accessible
    if curl -f http://localhost:9093/-/healthy &> /dev/null; then
        log_success "Alertmanager is running"
    else
        log_warning "Alertmanager is not accessible"
    fi
}

# Setup backup
setup_backup() {
    log_info "Setting up backup..."
    
    # Create backup directory
    mkdir -p "$PROJECT_ROOT/backups"
    
    # Set up backup script
    cat > "$PROJECT_ROOT/scripts/backup.sh" << 'EOF'
#!/bin/bash
# IPPAN Backup Script

BACKUP_DIR="/backups"
DATA_DIR="/data"
KEYS_DIR="/keys"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="ippan_backup_${TIMESTAMP}.tar.gz"

# Create backup
tar -czf "${BACKUP_DIR}/${BACKUP_FILE}" \
    -C "$DATA_DIR" . \
    -C "$KEYS_DIR" .

# Encrypt backup if encryption key is provided
if [[ -n "${BACKUP_ENCRYPTION_KEY:-}" ]]; then
    openssl enc -aes-256-cbc -salt -in "${BACKUP_DIR}/${BACKUP_FILE}" \
        -out "${BACKUP_DIR}/${BACKUP_FILE}.enc" \
        -k "$BACKUP_ENCRYPTION_KEY"
    rm "${BACKUP_DIR}/${BACKUP_FILE}"
    BACKUP_FILE="${BACKUP_FILE}.enc"
fi

# Clean up old backups (keep last 30 days)
find "$BACKUP_DIR" -name "ippan_backup_*.tar.gz*" -mtime +30 -delete

echo "Backup completed: ${BACKUP_FILE}"
EOF
    
    chmod +x "$PROJECT_ROOT/scripts/backup.sh"
    
    log_success "Backup setup completed"
}

# Health check
health_check() {
    log_info "Performing health check..."
    
    # Check if IPPAN node is responding
    if curl -f http://localhost:80/health &> /dev/null; then
        log_success "IPPAN node is healthy"
    else
        log_error "IPPAN node is not responding"
        return 1
    fi
    
    # Check if API is responding
    if curl -f http://localhost:3000/api/v1/status &> /dev/null; then
        log_success "IPPAN API is healthy"
    else
        log_error "IPPAN API is not responding"
        return 1
    fi
    
    # Check if P2P port is open
    if nc -z localhost 8080; then
        log_success "IPPAN P2P port is open"
    else
        log_error "IPPAN P2P port is not accessible"
        return 1
    fi
    
    log_success "Health check completed successfully"
}

# Main deployment function
main() {
    log_info "Starting IPPAN production deployment..."
    log_info "Environment: $DEPLOYMENT_ENV"
    log_info "Docker Registry: $DOCKER_REGISTRY"
    log_info "Image Tag: $IMAGE_TAG"
    
    # Check prerequisites
    check_prerequisites
    
    # Generate SSL certificates
    generate_ssl_certificates
    
    # Build images
    build_images
    
    # Deploy based on environment
    case "${DEPLOYMENT_TYPE:-docker-compose}" in
        "docker-compose")
            deploy_docker_compose
            ;;
        "kubernetes")
            deploy_kubernetes
            ;;
        "both")
            deploy_docker_compose
            deploy_kubernetes
            ;;
        *)
            log_error "Invalid deployment type: ${DEPLOYMENT_TYPE}"
            exit 1
            ;;
    esac
    
    # Setup monitoring
    setup_monitoring
    
    # Setup backup
    setup_backup
    
    # Perform health check
    health_check
    
    log_success "IPPAN production deployment completed successfully!"
    log_info "Access points:"
    log_info "  - Frontend: http://localhost:80"
    log_info "  - API: http://localhost:3000"
    log_info "  - P2P: localhost:8080"
    log_info "  - Prometheus: http://localhost:9090"
    log_info "  - Grafana: http://localhost:3001 (admin/admin123)"
    log_info "  - Alertmanager: http://localhost:9093"
}

# Run main function
main "$@"