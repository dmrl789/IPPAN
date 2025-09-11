#!/bin/bash

# IPPAN Production Deployment Script
# This script deploys IPPAN to production environment

set -e

# Configuration
PROJECT_NAME="ippan"
ENVIRONMENT="production"
DOCKER_COMPOSE_FILE="deployments/production/docker-compose.production.yml"
ENV_FILE=".env.production"

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
    
    # Check if Docker is installed
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed. Please install Docker first."
        exit 1
    fi
    
    # Check if Docker Compose is installed
    if ! command -v docker-compose &> /dev/null; then
        log_error "Docker Compose is not installed. Please install Docker Compose first."
        exit 1
    fi
    
    # Check if OpenSSL is installed
    if ! command -v openssl &> /dev/null; then
        log_error "OpenSSL is not installed. Please install OpenSSL first."
        exit 1
    fi
    
    log_success "All prerequisites are met."
}

# Generate SSL certificates
generate_ssl_certificates() {
    log_info "Generating SSL certificates..."
    
    if [ ! -f "scripts/generate-ssl-certs.sh" ]; then
        log_error "SSL certificate generation script not found."
        exit 1
    fi
    
    chmod +x scripts/generate-ssl-certs.sh
    ./scripts/generate-ssl-certs.sh
    
    log_success "SSL certificates generated successfully."
}

# Create environment file
create_environment_file() {
    log_info "Creating environment file..."
    
    if [ ! -f "$ENV_FILE" ]; then
        cat > "$ENV_FILE" << EOF
# IPPAN Production Environment Configuration

# Database
POSTGRES_DB=ippan_production
POSTGRES_USER=ippan
POSTGRES_PASSWORD=${POSTGRES_PASSWORD:-$(openssl rand -base64 32)}

# Redis
REDIS_PASSWORD=${REDIS_PASSWORD:-$(openssl rand -base64 32)}

# JWT
JWT_SECRET=${JWT_SECRET:-$(openssl rand -base64 64)}

# Grafana
GRAFANA_ADMIN_PASSWORD=${GRAFANA_ADMIN_PASSWORD:-$(openssl rand -base64 16)}
GRAFANA_SECRET_KEY=${GRAFANA_SECRET_KEY:-$(openssl rand -base64 32)}

# Backup
BACKUP_ENCRYPTION_KEY=${BACKUP_ENCRYPTION_KEY:-$(openssl rand -base64 32)}

# SMTP (for alerts)
SMTP_PASSWORD=${SMTP_PASSWORD:-}
EMAIL_SMTP_HOST=${EMAIL_SMTP_HOST:-smtp.gmail.com}
EMAIL_SMTP_PORT=${EMAIL_SMTP_PORT:-587}
EMAIL_USERNAME=${EMAIL_USERNAME:-alerts@ippan.network}
EMAIL_PASSWORD=${EMAIL_PASSWORD:-}

# Slack (for alerts)
SLACK_WEBHOOK_URL=${SLACK_WEBHOOK_URL:-}

# Network
IPPAN_NETWORK_PORT=8080
IPPAN_API_PORT=3000
IPPAN_P2P_PORT=8080

# Security
IPPAN_ENABLE_TLS=true
IPPAN_ENABLE_MUTUAL_AUTH=true

# Performance
IPPAN_MAX_CONNECTIONS=1000
IPPAN_THREAD_POOL_SIZE=8
IPPAN_CACHE_SIZE=2147483648
IPPAN_MEMORY_POOL_SIZE=1073741824

# Monitoring
PROMETHEUS_RETENTION=30d
GRAFANA_ADMIN_USER=admin
ALERTMANAGER_CONFIG_FILE=/etc/alertmanager/alertmanager.yml

# Logging
RUST_LOG=info
LOG_LEVEL=info
LOG_FORMAT=json

# Production flags
NODE_ENV=production
IPPAN_ENVIRONMENT=production
IPPAN_ENABLE_METRICS=true
IPPAN_ENABLE_HEALTH_CHECKS=true
EOF
        log_success "Environment file created: $ENV_FILE"
    else
        log_warning "Environment file already exists: $ENV_FILE"
    fi
}

# Build Docker images
build_docker_images() {
    log_info "Building Docker images..."
    
    # Build main IPPAN image
    docker build -f Dockerfile.production -t ippan/ippan:latest .
    
    # Build monitoring images (if needed)
    docker pull prom/prometheus:latest
    docker pull grafana/grafana:latest
    docker pull prom/alertmanager:latest
    docker pull fluent/fluentd:v1.16-debian-1
    docker pull nginx:alpine
    docker pull alpine:latest
    
    log_success "Docker images built successfully."
}

# Deploy services
deploy_services() {
    log_info "Deploying IPPAN services..."
    
    # Stop existing services
    docker-compose -f "$DOCKER_COMPOSE_FILE" --env-file "$ENV_FILE" down
    
    # Start services
    docker-compose -f "$DOCKER_COMPOSE_FILE" --env-file "$ENV_FILE" up -d
    
    log_success "IPPAN services deployed successfully."
}

# Wait for services to be ready
wait_for_services() {
    log_info "Waiting for services to be ready..."
    
    # Wait for IPPAN node
    log_info "Waiting for IPPAN node..."
    timeout 300 bash -c 'until curl -f http://localhost:3000/api/v1/status; do sleep 5; done'
    
    # Wait for Prometheus
    log_info "Waiting for Prometheus..."
    timeout 300 bash -c 'until curl -f http://localhost:9090/-/healthy; do sleep 5; done'
    
    # Wait for Grafana
    log_info "Waiting for Grafana..."
    timeout 300 bash -c 'until curl -f http://localhost:3001/api/health; do sleep 5; done'
    
    log_success "All services are ready."
}

# Verify deployment
verify_deployment() {
    log_info "Verifying deployment..."
    
    # Check IPPAN node health
    if curl -f http://localhost:3000/api/v1/status > /dev/null 2>&1; then
        log_success "IPPAN node is healthy"
    else
        log_error "IPPAN node health check failed"
        exit 1
    fi
    
    # Check Prometheus
    if curl -f http://localhost:9090/-/healthy > /dev/null 2>&1; then
        log_success "Prometheus is healthy"
    else
        log_error "Prometheus health check failed"
        exit 1
    fi
    
    # Check Grafana
    if curl -f http://localhost:3001/api/health > /dev/null 2>&1; then
        log_success "Grafana is healthy"
    else
        log_error "Grafana health check failed"
        exit 1
    fi
    
    # Check Nginx
    if curl -f http://localhost/health > /dev/null 2>&1; then
        log_success "Nginx is healthy"
    else
        log_error "Nginx health check failed"
        exit 1
    fi
    
    log_success "Deployment verification completed successfully."
}

# Show deployment information
show_deployment_info() {
    log_info "Deployment Information:"
    echo ""
    echo "🌐 IPPAN Node API: http://localhost:3000"
    echo "📊 Prometheus: http://localhost:9090"
    echo "📈 Grafana: http://localhost:3001"
    echo "🔔 AlertManager: http://localhost:9093"
    echo "🌍 Load Balancer: http://localhost (HTTP) / https://localhost (HTTPS)"
    echo ""
    echo "📋 Useful Commands:"
    echo "  View logs: docker-compose -f $DOCKER_COMPOSE_FILE logs -f"
    echo "  Stop services: docker-compose -f $DOCKER_COMPOSE_FILE down"
    echo "  Restart services: docker-compose -f $DOCKER_COMPOSE_FILE restart"
    echo "  Scale services: docker-compose -f $DOCKER_COMPOSE_FILE up -d --scale ippan-node=3"
    echo ""
    echo "🔐 Security Notes:"
    echo "  - SSL certificates are generated in deployments/ssl/"
    echo "  - Environment variables are in $ENV_FILE"
    echo "  - Keep private keys secure and never commit them to version control"
    echo "  - Change default passwords in production"
    echo ""
    echo "📊 Monitoring:"
    echo "  - Grafana admin password: Check $ENV_FILE"
    echo "  - Prometheus metrics: http://localhost:9090/metrics"
    echo "  - Alert rules: deployments/monitoring/ippan-production-rules.yml"
    echo ""
}

# Main deployment function
main() {
    log_info "Starting IPPAN production deployment..."
    
    check_prerequisites
    generate_ssl_certificates
    create_environment_file
    build_docker_images
    deploy_services
    wait_for_services
    verify_deployment
    show_deployment_info
    
    log_success "IPPAN production deployment completed successfully! 🚀"
}

# Run main function
main "$@"